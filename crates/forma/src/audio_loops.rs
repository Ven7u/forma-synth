//! Audio loop grid — sample-accurate loop station running on the DSP thread.
//!
//! Each of the 12 pads holds a stereo audio buffer. The pad captures the *live*
//! instrument's rendered output (the focused track's dry synth signal), so each
//! loop freezes whatever patch was live at record time — change the patch freely
//! between loops and existing pads are untouched. Pads are fully independent:
//! any pad can hold any sound, and any number can play at once.
//!
//! Timing is driven by an internal sample counter (not the UI-thread metronome),
//! so record/play start and stop land exactly on the bar. Audio bakes tempo and
//! pitch at record time — the session commits to one tempo.
//!
//! Threading contract:
//!   * The audio thread OWNS the sample buffers ([`LoopProcessor`]) — it is the
//!     only thing that ever touches audio data.
//!   * The UI thread only reads/writes the lock-free [`LoopGridAtomics`]: it
//!     posts taps / length / clear requests and reads back state, playhead, and
//!     a peak-envelope for the waveform preview.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;

// ── Pure beat-state helpers ────────────────────────────────────────────────────
//
// These functions derive UI display state from the audio-thread bar phase.
// Keeping them pure (no atomics, no side effects) makes them trivially testable.

/// Compute per-beat display state from the bar phase written by the audio thread.
///
/// - `bar_phase` – position within the current bar, 0.0..1.0
///   (from [`LoopGridAtomics::bar_phase`], updated every audio buffer)
/// - `beats_per_bar` – time-signature numerator, ≥ 1
///
/// Returns `(beat_idx, beat_frac)`:
/// - `beat_idx`  – index of the currently active beat (0 = downbeat)
/// - `beat_frac` – progress within that beat, 0.0..1.0
///
/// This is the single source of truth for beat animations in the loop grid.
/// Using it instead of the UI-thread metro clock ensures the visualization is
/// always sample-accurately aligned to the actual recording start, regardless
/// of wall-clock drift or tempo changes.
pub fn beat_state_from_bar_phase(bar_phase: f32, beats_per_bar: u32) -> (usize, f32) {
    let beats = beats_per_bar.max(1) as f32;
    // Keep phase strictly in [0, 1) so beat_idx never equals beats_per_bar.
    let phase = bar_phase.clamp(0.0, 1.0 - f32::EPSILON);
    let pos = phase * beats;
    let beat_idx = pos as usize;
    let beat_frac = pos - beat_idx as f32; // avoids float fract() precision edge cases
    (beat_idx, beat_frac)
}

/// Compute the fill fraction for one segment of the beat timeline strip.
///
/// - `segment`   – which beat slot (0 = downbeat)
/// - `beat_idx`  – currently active beat index
/// - `beat_frac` – progress within the active beat, 0.0..1.0
///
/// Returns 0.0 (empty), 0..1 (filling), or 1.0 (complete).
///
/// **Invariant**: `sum over all segments == beat_idx + beat_frac == bar_phase * beats_per_bar`
pub fn segment_fill(segment: usize, beat_idx: usize, beat_frac: f32) -> f32 {
    if segment < beat_idx {
        1.0
    } else if segment == beat_idx {
        beat_frac.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub const LOOP_PADS: usize = 12;
/// Number of peak-envelope bins stored per pad for the UI waveform preview.
pub const LOOP_WAVE_BINS: usize = 48;

// ── Worst-case sizing bounds ────────────────────────────────────────────────
//
// These mirror the ranges the UI actually exposes (BPM drag value in
// main.rs, beats-per-bar drag value in metronome.rs, and the loop-length
// chips in loop_grid.rs / loop_pad.rs) so pad buffers are pre-allocated
// large enough that recording can never silently truncate.

/// Slowest tempo reachable from the BPM control — matches the DragValue
/// range in `main.rs` (`40..=600`).
pub(crate) const MIN_BPM: f64 = 40.0;
/// Widest time signature reachable from the beats-per-bar control — matches
/// the DragValue range in `ui/metronome.rs` (`2..=8`).
pub(crate) const MAX_BEATS_PER_BAR: u32 = 8;
/// Longest loop length offered by the length chips.
pub(crate) const MAX_LOOP_BARS: u32 = 4;

/// Playback / record state of a single pad. Mirrors the UI enum; kept as `u8`
/// for the shared atomic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PadState {
    Empty = 0,
    ArmedRec = 1,
    Recording = 2,
    ArmedPlay = 3,
    Playing = 4,
    ArmedStop = 5,
    Stopped = 6,
}

impl PadState {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => PadState::ArmedRec,
            2 => PadState::Recording,
            3 => PadState::ArmedPlay,
            4 => PadState::Playing,
            5 => PadState::ArmedStop,
            6 => PadState::Stopped,
            _ => PadState::Empty,
        }
    }
}

/// Lock-free control + readback block shared between the UI and audio threads.
pub struct LoopGridAtomics {
    // ── Transport (UI writes) ──────────────────────────────────────────────
    /// Session tempo, f32 bits.
    pub bpm: AtomicU32,
    /// Beats per bar (time-signature numerator).
    pub beats_per_bar: AtomicU32,
    /// Which track engine to tap when recording (the live instrument).
    pub live_track: AtomicUsize,
    /// True while the LIVE loop grid is on screen; gates the transport clock.
    pub running: AtomicBool,
    /// Position within the current bar, 0.0..1.0. Written each buffer; UI reads for beat feedback.
    pub bar_phase: AtomicU32,
    /// Whether to synthesize an audible metronome click on every beat.
    pub click_enabled: AtomicBool,
    /// Click volume (f32 bits), 0.0–1.0.
    pub click_volume: AtomicU32,

    // ── Per-pad requests (UI writes, audio consumes) ───────────────────────
    /// Monotonic tap counter — UI increments on each user tap of the pad.
    pub tap: [AtomicU32; LOOP_PADS],
    /// Monotonic clear counter — UI increments to erase the pad.
    pub clear: [AtomicU32; LOOP_PADS],
    /// Requested loop length in bars (1 / 2 / 4). Applied when empty/stopped.
    pub len_bars: [AtomicU32; LOOP_PADS],

    // ── Per-pad readback (audio writes, UI reads) ──────────────────────────
    pub state: [AtomicU8; LOOP_PADS],
    /// Current playback/record position in samples.
    pub playhead: [AtomicU32; LOOP_PADS],
    /// Recorded length in samples (0 = empty).
    pub length_samp: [AtomicU32; LOOP_PADS],
    /// Peak-envelope bins (f32 bits) for the mini waveform, written on commit.
    pub wave: [[AtomicU32; LOOP_WAVE_BINS]; LOOP_PADS],
}

impl LoopGridAtomics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            bpm: AtomicU32::new(120.0_f32.to_bits()),
            beats_per_bar: AtomicU32::new(4),
            live_track: AtomicUsize::new(0),
            running: AtomicBool::new(false),
            bar_phase: AtomicU32::new(0),
            click_enabled: AtomicBool::new(false),
            click_volume: AtomicU32::new(0.5_f32.to_bits()),
            tap: std::array::from_fn(|_| AtomicU32::new(0)),
            clear: std::array::from_fn(|_| AtomicU32::new(0)),
            len_bars: std::array::from_fn(|_| AtomicU32::new(2)),
            state: std::array::from_fn(|_| AtomicU8::new(PadState::Empty as u8)),
            playhead: std::array::from_fn(|_| AtomicU32::new(0)),
            length_samp: std::array::from_fn(|_| AtomicU32::new(0)),
            wave: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU32::new(0))),
        })
    }

    // ── UI-side helpers ────────────────────────────────────────────────────

    pub fn set_bpm(&self, bpm: f32) {
        self.bpm.store(bpm.to_bits(), Ordering::Relaxed);
    }
    pub fn set_beats_per_bar(&self, beats: u32) {
        self.beats_per_bar.store(beats.max(1), Ordering::Relaxed);
    }
    pub fn set_live_track(&self, t: usize) {
        self.live_track.store(t, Ordering::Relaxed);
    }
    pub fn set_running(&self, on: bool) {
        self.running.store(on, Ordering::Relaxed);
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    /// Current bar position 0.0..1.0 (0 when stopped).
    pub fn bar_phase(&self) -> f32 {
        f32::from_bits(self.bar_phase.load(Ordering::Relaxed))
    }
    /// Beats per bar as stored by the UI thread.
    pub fn beats_per_bar_value(&self) -> u32 {
        self.beats_per_bar.load(Ordering::Relaxed).max(1)
    }

    pub fn set_click_enabled(&self, on: bool) {
        self.click_enabled.store(on, Ordering::Relaxed);
    }
    pub fn is_click_enabled(&self) -> bool {
        self.click_enabled.load(Ordering::Relaxed)
    }
    pub fn set_click_volume(&self, v: f32) {
        self.click_volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
    pub fn click_volume_value(&self) -> f32 {
        f32::from_bits(self.click_volume.load(Ordering::Relaxed))
    }

    /// Post a user tap for a pad (arm / cancel — interpreted on the audio thread).
    pub fn tap(&self, pad: usize) {
        self.tap[pad].fetch_add(1, Ordering::Relaxed);
    }
    /// Request the pad be cleared back to empty.
    pub fn clear(&self, pad: usize) {
        self.clear[pad].fetch_add(1, Ordering::Relaxed);
    }
    /// Set a pad's loop length in bars (takes effect on the next record).
    pub fn set_len_bars(&self, pad: usize, bars: u32) {
        self.len_bars[pad].store(bars, Ordering::Relaxed);
    }

    pub fn pad_state(&self, pad: usize) -> PadState {
        PadState::from_u8(self.state[pad].load(Ordering::Relaxed))
    }
    pub fn pad_len_bars(&self, pad: usize) -> u32 {
        self.len_bars[pad].load(Ordering::Relaxed)
    }
    pub fn pad_has_content(&self, pad: usize) -> bool {
        self.length_samp[pad].load(Ordering::Relaxed) > 0
    }
    /// Playback/record progress in 0..1 (0 when empty).
    pub fn pad_progress(&self, pad: usize) -> f32 {
        let len = self.length_samp[pad].load(Ordering::Relaxed);
        if len == 0 {
            return 0.0;
        }
        let pos = self.playhead[pad].load(Ordering::Relaxed);
        (pos as f32 / len as f32).clamp(0.0, 1.0)
    }
    /// Copy the pad's peak envelope into `out` (values 0..1).
    pub fn pad_wave(&self, pad: usize, out: &mut [f32; LOOP_WAVE_BINS]) {
        for (i, o) in out.iter_mut().enumerate() {
            *o = f32::from_bits(self.wave[pad][i].load(Ordering::Relaxed));
        }
    }
}

// ── Audio-thread processor ─────────────────────────────────────────────────

/// One pad's owned playback/record buffer + cursors (audio thread only).
struct Pad {
    /// Pre-allocated stereo buffer; never reallocated on the audio thread.
    buf: Vec<[f32; 2]>,
    /// Recorded length in samples (0 = empty).
    len: usize,
    /// Playback cursor.
    pos: usize,
    /// Record cursor.
    rec_pos: usize,
    /// Target record length in samples (set at the record-start bar).
    rec_target: usize,
    /// Peak-envelope accumulator, filled while recording.
    env: [f32; LOOP_WAVE_BINS],
    /// Last-seen tap / clear counters (edge detection).
    last_tap: u32,
    last_clear: u32,
    state: PadState,
}

impl Pad {
    fn new(cap: usize) -> Self {
        Self {
            buf: vec![[0.0, 0.0]; cap],
            len: 0,
            pos: 0,
            rec_pos: 0,
            rec_target: 0,
            env: [0.0; LOOP_WAVE_BINS],
            last_tap: 0,
            last_clear: 0,
            state: PadState::Empty,
        }
    }

    fn has_content(&self) -> bool {
        self.len > 0
    }
}

/// The audio-thread half of the loop grid: owns all buffers and the transport.
pub struct LoopProcessor {
    atomics: Arc<LoopGridAtomics>,
    pads: Vec<Pad>,
    /// Free-running sample counter; resets when the grid becomes active.
    transport: u64,
    was_running: bool,
    /// Max buffer capacity per pad (samples) — worst-case loop length.
    cap: usize,
    sr: f64,
    // Cached per-buffer transport parameters.
    samples_per_bar: u64,
    samples_per_beat: u64,
    // Click oscillator state (audio-thread only).
    click_env: f32,
    click_osc: f32,
    click_is_down: bool,
}

impl LoopProcessor {
    pub fn new(atomics: Arc<LoopGridAtomics>, sr: f64) -> Self {
        // Pre-allocate for the worst case reachable via the UI: slowest
        // tempo × widest time signature × longest loop length. Sized once
        // up front (never reallocated on the audio thread) so recording can
        // never silently truncate — e.g. ≈48 s at 40 BPM, 8 beats/bar, 4 bars.
        let seconds_per_bar = (60.0 / MIN_BPM) * MAX_BEATS_PER_BAR as f64;
        let cap = (sr * seconds_per_bar * MAX_LOOP_BARS as f64).ceil() as usize;
        let pads = (0..LOOP_PADS).map(|_| Pad::new(cap)).collect();
        Self {
            atomics,
            pads,
            transport: 0,
            was_running: false,
            cap,
            sr,
            samples_per_bar: (sr * 2.0) as u64, // recomputed each buffer
            samples_per_beat: (sr * 0.5) as u64,
            click_env: 0.0,
            click_osc: 0.0,
            click_is_down: false,
        }
    }

    /// Index of the live instrument track to tap while recording.
    #[inline]
    pub fn live_track(&self) -> usize {
        self.atomics.live_track.load(Ordering::Relaxed)
    }

    /// Per-buffer setup: refresh transport parameters and process control
    /// requests (taps / clears / running edge). Call once before the frame loop.
    pub fn begin_buffer(&mut self) {
        let running = self.atomics.running.load(Ordering::Relaxed);
        // Reset the bar grid when the grid becomes active so bar 1 is aligned.
        if running && !self.was_running {
            self.transport = 0;
        }
        self.was_running = running;

        let bpm = f32::from_bits(self.atomics.bpm.load(Ordering::Relaxed)) as f64;
        // Lower bound matches MIN_BPM so pad buffers (sized for that worst
        // case) can never be asked to hold more than they can — this can be
        // hit at reads other than the UI's own 40..=600 slider, e.g. a
        // hand-edited or corrupted save file.
        let bpm = bpm.clamp(MIN_BPM, 999.0);
        let beats = self.atomics.beats_per_bar.load(Ordering::Relaxed).max(1) as f64;
        let samples_per_beat = self.sr * 60.0 / bpm;
        self.samples_per_beat = samples_per_beat.max(1.0) as u64;
        self.samples_per_bar = (samples_per_beat * beats).max(1.0) as u64;

        // Publish bar position for the UI beat-clock.
        let phase = if running && self.samples_per_bar > 0 {
            (self.transport % self.samples_per_bar) as f32 / self.samples_per_bar as f32
        } else {
            0.0
        };
        self.atomics.bar_phase.store(phase.to_bits(), Ordering::Relaxed);

        // Consume per-pad control requests.
        for i in 0..LOOP_PADS {
            // Clear takes precedence.
            let clear = self.atomics.clear[i].load(Ordering::Relaxed);
            if clear != self.pads[i].last_clear {
                self.pads[i].last_clear = clear;
                self.clear_pad(i);
            }
            let tap = self.atomics.tap[i].load(Ordering::Relaxed);
            if tap != self.pads[i].last_tap {
                self.pads[i].last_tap = tap;
                self.on_tap(i);
            }
        }
    }

    fn clear_pad(&mut self, i: usize) {
        let p = &mut self.pads[i];
        p.state = PadState::Empty;
        p.len = 0;
        p.pos = 0;
        p.rec_pos = 0;
        p.env = [0.0; LOOP_WAVE_BINS];
        self.atomics.length_samp[i].store(0, Ordering::Relaxed);
        self.atomics.playhead[i].store(0, Ordering::Relaxed);
        for b in 0..LOOP_WAVE_BINS {
            self.atomics.wave[i][b].store(0.0_f32.to_bits(), Ordering::Relaxed);
        }
        self.publish_state(i);
    }

    /// Interpret a user tap given the pad's current state.
    fn on_tap(&mut self, i: usize) {
        // Commit a recording early → play from next bar. Handled separately
        // from the match below because it needs to call `commit_recording`
        // (which owns `&mut self`) rather than just producing a next state.
        if self.pads[i].state == PadState::Recording {
            let rec_pos = self.pads[i].rec_pos;
            if rec_pos == 0 {
                // Nothing captured yet — treat like cancelling an arm.
                self.pads[i].state = PadState::Empty;
            } else {
                self.commit_recording(i, rec_pos);
                self.pads[i].state = PadState::ArmedPlay;
            }
            self.publish_state(i);
            return;
        }

        let has = self.pads[i].has_content();
        let next = match self.pads[i].state {
            PadState::Empty => PadState::ArmedRec,
            PadState::Stopped => PadState::ArmedPlay,
            PadState::Playing => PadState::ArmedStop,
            // Cancel a pending arm.
            PadState::ArmedRec => PadState::Empty,
            PadState::ArmedPlay => {
                if has {
                    PadState::Stopped
                } else {
                    PadState::Empty
                }
            }
            PadState::ArmedStop => PadState::Playing,
            PadState::Recording => unreachable!("handled above"),
        };
        self.pads[i].state = next;
        self.publish_state(i);
    }

    fn publish_state(&self, i: usize) {
        self.atomics.state[i].store(self.pads[i].state as u8, Ordering::Relaxed);
    }

    /// Promote armed pads on a bar boundary.
    fn on_bar(&mut self) {
        let spb = self.samples_per_bar as usize;
        for i in 0..LOOP_PADS {
            match self.pads[i].state {
                PadState::ArmedRec => {
                    let len_bars = self.atomics.len_bars[i].load(Ordering::Relaxed).max(1) as usize;
                    let len = (len_bars * spb).min(self.cap);
                    let p = &mut self.pads[i];
                    p.len = 0; // not committed until recording finishes
                    p.rec_pos = 0;
                    p.pos = 0;
                    p.rec_target = len;
                    p.env = [0.0; LOOP_WAVE_BINS];
                    p.state = PadState::Recording;
                    self.publish_state(i);
                }
                PadState::ArmedPlay => {
                    self.pads[i].pos = 0;
                    self.pads[i].state = PadState::Playing;
                    self.publish_state(i);
                }
                PadState::ArmedStop => {
                    self.pads[i].state = PadState::Stopped;
                    self.publish_state(i);
                }
                _ => {}
            }
        }
    }

    /// Process a single stereo frame. `live` is the live instrument's dry output
    /// (used as the record source); returns the summed loop playback to add into
    /// the mix bus (pre-EQ/limiter). Advances the transport.
    #[inline]
    pub fn process_frame(&mut self, live_l: f32, live_r: f32) -> (f32, f32) {
        if !self.was_running {
            return (0.0, 0.0);
        }

        // Bar-boundary promotion (sample-accurate).
        if self.transport.is_multiple_of(self.samples_per_bar) {
            self.on_bar();
        }

        // Beat-boundary click trigger.
        let spbeat = self.samples_per_beat.max(1);
        if self.transport.is_multiple_of(spbeat)
            && self.atomics.click_enabled.load(Ordering::Relaxed)
        {
            self.click_env = 1.0;
            self.click_osc = 0.0;
            let beat_idx = (self.transport / spbeat)
                % (self.samples_per_bar / spbeat).max(1);
            self.click_is_down = beat_idx == 0;
        }

        let mut out_l = 0.0f32;
        let mut out_r = 0.0f32;

        for i in 0..LOOP_PADS {
            match self.pads[i].state {
                PadState::Recording => {
                    self.record_sample(i, live_l, live_r);
                }
                PadState::Playing => {
                    let p = &mut self.pads[i];
                    if p.len > 0 {
                        let s = p.buf[p.pos];
                        out_l += s[0];
                        out_r += s[1];
                        p.pos += 1;
                        if p.pos >= p.len {
                            p.pos = 0;
                        }
                        self.atomics.playhead[i].store(p.pos as u32, Ordering::Relaxed);
                    }
                }
                _ => {}
            }
        }

        // Click synthesis — short exponential-decay sine (12 ms).
        if self.click_env > 1e-4 {
            let vol = f32::from_bits(self.atomics.click_volume.load(Ordering::Relaxed));
            let freq = if self.click_is_down { 1000.0f32 } else { 700.0f32 };
            let click_s = (self.click_osc * std::f32::consts::TAU).sin()
                * self.click_env
                * vol
                * 0.35;
            self.click_osc = (self.click_osc + freq / self.sr as f32).fract();
            // Time constant: -1/(0.012 * sr) ≈ -1/529 at 44100 Hz → ~12 ms decay.
            self.click_env *= (-1.0f32 / (0.012 * self.sr as f32)).exp();
            out_l += click_s;
            out_r += click_s;
        }

        self.transport = self.transport.wrapping_add(1);
        (out_l, out_r)
    }

    #[inline]
    fn record_sample(&mut self, i: usize, l: f32, r: f32) {
        let target = self.pads[i].rec_target;
        let p = &mut self.pads[i];
        if p.rec_pos < target && p.rec_pos < self.cap {
            p.buf[p.rec_pos] = [l, r];
            // Peak envelope bin.
            let bin = (p.rec_pos * LOOP_WAVE_BINS / target.max(1)).min(LOOP_WAVE_BINS - 1);
            let mag = l.abs().max(r.abs());
            if mag > p.env[bin] {
                p.env[bin] = mag;
            }
            p.rec_pos += 1;
            self.atomics.playhead[i].store(p.rec_pos as u32, Ordering::Relaxed);
        }
        if self.pads[i].rec_pos >= target {
            self.commit_recording(i, target);
        }
    }

    fn commit_recording(&mut self, i: usize, target: usize) {
        // Declick: short linear fade in/out at the buffer edges so the
        // hard-cut loop wrap in `process_frame` doesn't click when the
        // recording doesn't happen to start/end near silence. ~5 ms, capped
        // at half the buffer so it can't overlap itself on tiny loops (e.g.
        // an early-committed recording).
        let fade_len = ((self.sr * 0.005) as usize).min(target / 2);

        let p = &mut self.pads[i];
        p.len = target;
        p.pos = 0;
        p.state = PadState::Playing;
        for s in 0..fade_len {
            let gain = s as f32 / fade_len as f32;
            let head = &mut p.buf[s];
            head[0] *= gain;
            head[1] *= gain;
            let tail = &mut p.buf[target - 1 - s];
            tail[0] *= gain;
            tail[1] *= gain;
        }
        // Publish length + envelope for the UI.
        self.atomics.length_samp[i].store(target as u32, Ordering::Relaxed);
        // Normalise envelope to 0..1 for display.
        let peak = p.env.iter().cloned().fold(0.0_f32, f32::max).max(1e-6);
        for b in 0..LOOP_WAVE_BINS {
            let v = (p.env[b] / peak).clamp(0.0, 1.0);
            self.atomics.wave[i][b].store(v.to_bits(), Ordering::Relaxed);
        }
        self.publish_state(i);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        beat_state_from_bar_phase, segment_fill, LoopGridAtomics, LoopProcessor, PadState,
        LOOP_WAVE_BINS, MAX_BEATS_PER_BAR, MAX_LOOP_BARS, MIN_BPM,
    };
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    // ── beat_state_from_bar_phase ─────────────────────────────────────────────

    #[test]
    fn test_4_4_beat_boundaries() {
        // 4/4: beats start at 0.0, 0.25, 0.5, 0.75
        let cases = [
            (0.0f32,  0usize, 0.0f32),
            (0.25,    1,      0.0),
            (0.5,     2,      0.0),
            (0.75,    3,      0.0),
        ];
        for (phase, exp_idx, exp_frac) in cases {
            let (idx, frac) = beat_state_from_bar_phase(phase, 4);
            assert_eq!(idx, exp_idx, "4/4 beat_idx at phase={phase}");
            assert!(approx_eq(frac, exp_frac), "4/4 beat_frac at phase={phase}: got {frac}");
        }
    }

    #[test]
    fn test_4_4_midbeat_frac() {
        // Halfway through beat 0 → frac=0.5
        let (idx, frac) = beat_state_from_bar_phase(0.125, 4);
        assert_eq!(idx, 0);
        assert!(approx_eq(frac, 0.5), "got {frac}");

        // Halfway through beat 1 → frac=0.5
        let (idx, frac) = beat_state_from_bar_phase(0.375, 4);
        assert_eq!(idx, 1);
        assert!(approx_eq(frac, 0.5), "got {frac}");

        // 3/4 through beat 3 → frac=0.75
        let (idx, frac) = beat_state_from_bar_phase(0.9375, 4);
        assert_eq!(idx, 3);
        assert!(approx_eq(frac, 0.75), "got {frac}");
    }

    #[test]
    fn test_3_4() {
        // 3/4: beats at 0, 1/3, 2/3
        let (idx, frac) = beat_state_from_bar_phase(0.0, 3);
        assert_eq!(idx, 0);
        assert!(approx_eq(frac, 0.0));

        let (idx, frac) = beat_state_from_bar_phase(1.0 / 3.0, 3);
        assert_eq!(idx, 1);
        assert!(approx_eq(frac, 0.0), "got frac={frac}");

        let (idx, frac) = beat_state_from_bar_phase(2.0 / 3.0, 3);
        assert_eq!(idx, 2);
        assert!(approx_eq(frac, 0.0), "got frac={frac}");

        // Halfway through bar → beat 1, frac=0.5
        let (idx, frac) = beat_state_from_bar_phase(0.5, 3);
        assert_eq!(idx, 1);
        assert!(approx_eq(frac, 0.5), "got frac={frac}");
    }

    #[test]
    fn test_7_8() {
        // 7/8: each beat is 1/7 of the bar
        for b in 0..7usize {
            let phase = b as f32 / 7.0;
            let (idx, frac) = beat_state_from_bar_phase(phase, 7);
            assert_eq!(idx, b, "7/8 beat_idx wrong at beat {b}");
            assert!(approx_eq(frac, 0.0), "7/8 beat_frac should be 0 at beat boundary {b}, got {frac}");
        }
    }

    #[test]
    fn test_boundary_phase_zero() {
        // phase=0 always means downbeat, frac=0
        for beats in [2u32, 3, 4, 5, 6, 7, 8] {
            let (idx, frac) = beat_state_from_bar_phase(0.0, beats);
            assert_eq!(idx, 0, "beats={beats}");
            assert!(approx_eq(frac, 0.0), "beats={beats}");
        }
    }

    #[test]
    fn test_boundary_phase_near_one() {
        // phase just under 1.0 should be last beat, nearly done
        let (idx, frac) = beat_state_from_bar_phase(0.9999, 4);
        assert_eq!(idx, 3, "last beat of 4/4");
        assert!(frac > 0.99, "frac should be near 1, got {frac}");

        let (idx, _frac) = beat_state_from_bar_phase(0.9999, 7);
        assert_eq!(idx, 6, "last beat of 7/8");
    }

    #[test]
    fn test_phase_one_clamps_safely() {
        // phase=1.0 is not produced by the audio thread but must not panic or
        // return beat_idx == beats_per_bar.
        for beats in [2u32, 3, 4, 7] {
            let (idx, _frac) = beat_state_from_bar_phase(1.0, beats);
            assert!((idx as u32) < beats, "beat_idx={idx} out of range for beats={beats}");
        }
    }

    // ── segment_fill ─────────────────────────────────────────────────────────

    #[test]
    fn test_segment_fill_past_current_future() {
        // beat_idx=2, beat_frac=0.75, 4 beats
        // seg 0 → 1.0 (past), seg 1 → 1.0 (past), seg 2 → 0.75 (current), seg 3 → 0.0 (future)
        assert!(approx_eq(segment_fill(0, 2, 0.75), 1.0));
        assert!(approx_eq(segment_fill(1, 2, 0.75), 1.0));
        assert!(approx_eq(segment_fill(2, 2, 0.75), 0.75));
        assert!(approx_eq(segment_fill(3, 2, 0.75), 0.0));
    }

    #[test]
    fn test_segment_fill_downbeat() {
        // At the downbeat (beat_idx=0, beat_frac=0.0), all segments except 0 are empty
        assert!(approx_eq(segment_fill(0, 0, 0.0), 0.0));
        assert!(approx_eq(segment_fill(1, 0, 0.0), 0.0));
        assert!(approx_eq(segment_fill(3, 0, 0.0), 0.0));
    }

    #[test]
    fn test_segment_fill_sum_invariant() {
        // The invariant: sum(fills) == beat_idx + beat_frac == bar_phase * beats
        // Test at several positions in 4/4 and 7/8.
        let cases_4_4: &[(f32, u32)] = &[
            (0.0, 4), (0.1, 4), (0.25, 4), (0.37, 4), (0.5, 4), (0.75, 4), (0.999, 4),
        ];
        for &(phase, beats) in cases_4_4 {
            let (beat_idx, beat_frac) = beat_state_from_bar_phase(phase, beats);
            let sum: f32 = (0..beats as usize)
                .map(|b| segment_fill(b, beat_idx, beat_frac))
                .sum();
            let expected = beat_idx as f32 + beat_frac;
            assert!(
                approx_eq(sum, expected),
                "sum invariant failed at phase={phase} beats={beats}: sum={sum} expected={expected}"
            );
        }

        let cases_7_8: &[(f32, u32)] = &[
            (0.0, 7), (1.0/7.0, 7), (3.0/7.0, 7), (6.5/7.0, 7),
        ];
        for &(phase, beats) in cases_7_8 {
            let (beat_idx, beat_frac) = beat_state_from_bar_phase(phase, beats);
            let sum: f32 = (0..beats as usize)
                .map(|b| segment_fill(b, beat_idx, beat_frac))
                .sum();
            let expected = beat_idx as f32 + beat_frac;
            assert!(
                approx_eq(sum, expected),
                "7/8 sum invariant failed at phase={phase}: sum={sum} expected={expected}"
            );
        }
    }

    // ── bar_phase produced by LoopProcessor ──────────────────────────────────

    /// Simulate the bar_phase formula from `LoopProcessor::begin_buffer`.
    fn simulated_bar_phase(transport: u64, samples_per_bar: u64) -> f32 {
        if samples_per_bar == 0 {
            return 0.0;
        }
        (transport % samples_per_bar) as f32 / samples_per_bar as f32
    }

    #[test]
    fn test_bar_phase_always_in_range() {
        // For any transport value and samples_per_bar, phase must be in [0, 1).
        let spb_values = [100u64, 441, 4410, 44100, 88199];
        for spb in spb_values {
            for t in [0u64, 1, spb / 2, spb - 1, spb, spb + 1, spb * 3 + 7, u32::MAX as u64] {
                let phase = simulated_bar_phase(t, spb);
                assert!(
                    (0.0..1.0).contains(&phase),
                    "phase={phase} out of [0,1) for transport={t} spb={spb}"
                );
            }
        }
    }

    #[test]
    fn test_bar_phase_resets_at_bar_boundary() {
        // At transport = N * samples_per_bar, phase must be exactly 0.
        let spb = 44100u64;
        for n in [0u64, 1, 2, 100] {
            let phase = simulated_bar_phase(n * spb, spb);
            assert!(approx_eq(phase, 0.0), "phase={phase} at bar boundary n={n}");
        }
    }

    #[test]
    fn test_bar_phase_monotone_within_bar() {
        // Within a bar, bar_phase strictly increases with transport.
        let spb = 1000u64;
        let mut prev = simulated_bar_phase(0, spb);
        for t in 1..spb {
            let cur = simulated_bar_phase(t, spb);
            assert!(cur > prev, "phase not monotone at t={t}");
            prev = cur;
        }
    }

    #[test]
    fn test_beat_state_consistent_across_tempos() {
        // beat_state_from_bar_phase is tempo-independent: only bar_phase and
        // beats_per_bar matter. Verify that two different tempos with the same
        // fractional bar position yield the same beat state.
        //
        // 120 BPM, 4/4, sr=44100: samples_per_beat = 44100*60/120 = 22050
        // 90 BPM,  4/4, sr=44100: samples_per_beat = 44100*60/90  = 29400
        let sr = 44100u64;
        let spb_120 = sr * 60 / 120 * 4; // samples_per_bar at 120 BPM 4/4
        let spb_90  = sr * 60 / 90  * 4; // samples_per_bar at 90 BPM 4/4

        // Both at "halfway through beat 2" — pick transport values accordingly.
        // For 120 BPM: beat 2 starts at 2*22050, halfway = 2*22050 + 11025
        let t_120 = 2 * (sr * 60 / 120) + (sr * 60 / 120) / 2;
        // For 90 BPM: beat 2 starts at 2*29400, halfway = 2*29400 + 14700
        let t_90  = 2 * (sr * 60 / 90)  + (sr * 60 / 90)  / 2;

        let phase_120 = simulated_bar_phase(t_120, spb_120);
        let phase_90  = simulated_bar_phase(t_90,  spb_90);

        let (idx_120, frac_120) = beat_state_from_bar_phase(phase_120, 4);
        let (idx_90,  frac_90)  = beat_state_from_bar_phase(phase_90,  4);

        assert_eq!(idx_120, 2, "120 BPM beat_idx");
        assert_eq!(idx_90,  2, "90 BPM beat_idx");
        assert!(approx_eq(frac_120, 0.5), "120 BPM beat_frac={frac_120}");
        assert!(approx_eq(frac_90,  0.5), " 90 BPM beat_frac={frac_90}");
    }

    #[test]
    fn test_time_signature_change_mid_bar() {
        // Simulate changing from 4/4 to 3/4 while bar_phase is mid-bar.
        // The bar_phase itself doesn't change — only how many beats it divides into.
        let bar_phase = 0.5f32;

        // In 4/4: beat 2, frac 0.0
        let (idx_4, frac_4) = beat_state_from_bar_phase(bar_phase, 4);
        assert_eq!(idx_4, 2);
        assert!(approx_eq(frac_4, 0.0));

        // In 3/4: beat 1, frac 0.5
        let (idx_3, frac_3) = beat_state_from_bar_phase(bar_phase, 3);
        assert_eq!(idx_3, 1);
        assert!(approx_eq(frac_3, 0.5), "got {frac_3}");

        // In 6/8: beat 3, frac 0.0
        let (idx_6, frac_6) = beat_state_from_bar_phase(bar_phase, 6);
        assert_eq!(idx_6, 3);
        assert!(approx_eq(frac_6, 0.0), "got {frac_6}");
    }

    // ── LoopProcessor integration regression tests ─────────────────────────────
    //
    // These drive LoopProcessor the same way the audio callback does: one
    // begin_buffer() per simulated hardware buffer, then process_frame() per
    // sample. No hardware/cpal dependency — it's pure computation over
    // LoopGridAtomics, so it runs fine as a plain unit test.

    #[test]
    fn test_early_commit_actually_commits_instead_of_going_silent() {
        let atomics = LoopGridAtomics::new();
        atomics.set_bpm(120.0);
        atomics.set_beats_per_bar(4);
        atomics.set_live_track(0);
        atomics.set_running(true);

        let sr = 1000.0;
        let mut proc = LoopProcessor::new(Arc::clone(&atomics), sr);

        // Arm + start recording pad 0.
        atomics.tap(0);
        proc.begin_buffer();

        // Record 50 frames of a constant signal.
        for _ in 0..50 {
            proc.process_frame(1.0, 1.0);
        }
        assert_eq!(
            atomics.pad_state(0),
            PadState::Recording,
            "should still be recording before the early-stop tap"
        );

        // Tap again mid-recording to stop early — this must commit what was
        // captured so far, not just switch state and strand len at 0.
        atomics.tap(0);
        proc.begin_buffer();

        // Run forward through the next bar boundary so ArmedPlay promotes to
        // Playing (bar is 2000 samples at 120 BPM/4-4/sr=1000).
        let mut promoted = false;
        for _ in 0..4000 {
            proc.process_frame(1.0, 1.0);
            if atomics.pad_state(0) == PadState::Playing {
                promoted = true;
                break;
            }
        }

        assert!(
            promoted,
            "pad never promoted to Playing after early-commit + bar boundary"
        );
        assert!(
            atomics.pad_has_content(0),
            "early-committed pad should have content, not be silently empty"
        );
        assert_eq!(
            atomics.length_samp[0].load(Ordering::Relaxed),
            50,
            "committed length should match samples captured before the early-stop tap"
        );

        let mut wave = [0.0f32; LOOP_WAVE_BINS];
        atomics.pad_wave(0, &mut wave);
        assert!(
            wave.iter().any(|&v| v > 0.0),
            "waveform should be published for an early-committed recording"
        );
    }

    #[test]
    fn test_early_commit_with_nothing_captured_cancels_to_empty() {
        // Tapping in the same instant recording starts (rec_pos == 0) should
        // behave like cancelling an arm, not commit an empty loop.
        //
        // This state (Recording with rec_pos == 0) can't actually be reached
        // through begin_buffer()/process_frame() alone in real operation —
        // entering Recording always records at least one sample within that
        // same process_frame() call, and every begin_buffer() in the real
        // audio callback is followed by many process_frame() calls before
        // the next one. So this exercises the on_tap branch directly, as a
        // defensive-code regression test.
        let atomics = LoopGridAtomics::new();
        let mut proc = LoopProcessor::new(Arc::clone(&atomics), 1000.0);

        proc.pads[0].state = PadState::Recording;
        proc.pads[0].rec_pos = 0;

        proc.on_tap(0);

        assert_eq!(atomics.pad_state(0), PadState::Empty);
        assert!(!atomics.pad_has_content(0));
    }

    #[test]
    fn test_buffer_capacity_covers_full_ui_range_without_truncation() {
        // Worst case reachable via the UI: slowest tempo, widest time
        // signature, longest loop length. Before the capacity fix this
        // combination silently truncated to the old fixed 8s cap.
        let atomics = LoopGridAtomics::new();
        atomics.set_bpm(MIN_BPM as f32);
        atomics.set_beats_per_bar(MAX_BEATS_PER_BAR);
        atomics.set_live_track(0);
        atomics.set_running(true);
        atomics.set_len_bars(0, MAX_LOOP_BARS);

        let sr = 200.0;
        let mut proc = LoopProcessor::new(Arc::clone(&atomics), sr);

        let seconds_per_bar = (60.0 / MIN_BPM) * MAX_BEATS_PER_BAR as f64;
        let expected_len = (sr * seconds_per_bar * MAX_LOOP_BARS as f64).ceil() as u32;

        atomics.tap(0);
        proc.begin_buffer();

        let mut committed = false;
        for _ in 0..(expected_len as usize * 2) {
            proc.process_frame(1.0, 1.0);
            if atomics.pad_state(0) == PadState::Playing {
                committed = true;
                break;
            }
        }

        assert!(committed, "recording never completed naturally");
        assert_eq!(
            atomics.length_samp[0].load(Ordering::Relaxed),
            expected_len,
            "recorded length should match the full requested length, not be clamped short by a too-small cap"
        );
    }

    #[test]
    fn test_commit_applies_declick_fade_at_buffer_edges() {
        let atomics = LoopGridAtomics::new();
        atomics.set_bpm(120.0);
        atomics.set_beats_per_bar(4);
        atomics.set_live_track(0);
        atomics.set_running(true);
        atomics.set_len_bars(0, 1);

        let sr = 1000.0;
        let mut proc = LoopProcessor::new(Arc::clone(&atomics), sr);

        atomics.tap(0);
        proc.begin_buffer();

        // 1 bar at 120 BPM/4-4/sr=1000 = 2000 samples.
        let mut committed = false;
        for _ in 0..4000 {
            proc.process_frame(1.0, 1.0);
            if atomics.pad_state(0) == PadState::Playing {
                committed = true;
                break;
            }
        }
        assert!(committed, "recording never completed naturally");

        let pad = &proc.pads[0];
        let target = pad.len;
        assert_eq!(target, 2000);
        let fade_len = ((sr * 0.005) as usize).min(target / 2);
        assert_eq!(fade_len, 5, "sanity-check the expected fade length at sr=1000");

        assert_eq!(pad.buf[0], [0.0, 0.0], "first sample should be fully faded to silence");
        assert_eq!(
            pad.buf[target - 1],
            [0.0, 0.0],
            "last sample should be fully faded to silence"
        );
        // Mid-ramp sample: gain = 2/5 = 0.4.
        assert!(
            (pad.buf[2][0] - 0.4).abs() < EPSILON,
            "got {}",
            pad.buf[2][0]
        );
        // Just past the fade zone on each edge should be untouched.
        assert_eq!(pad.buf[fade_len], [1.0, 1.0], "sample just past the fade-in zone should be untouched");
        assert_eq!(
            pad.buf[target - fade_len - 1],
            [1.0, 1.0],
            "sample just before the fade-out zone should be untouched"
        );
        // Middle of the loop should be completely unaffected.
        assert_eq!(pad.buf[target / 2], [1.0, 1.0]);
    }
}
