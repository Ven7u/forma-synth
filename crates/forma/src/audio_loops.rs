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

pub const LOOP_PADS: usize = 12;
/// Number of peak-envelope bins stored per pad for the UI waveform preview.
pub const LOOP_WAVE_BINS: usize = 48;

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
}

impl LoopProcessor {
    pub fn new(atomics: Arc<LoopGridAtomics>, sr: f64) -> Self {
        // Pre-allocate ~8 s per pad — covers 4 bars of 4/4 down to ~60 BPM.
        let cap = (sr * 8.0).ceil() as usize;
        let pads = (0..LOOP_PADS).map(|_| Pad::new(cap)).collect();
        Self {
            atomics,
            pads,
            transport: 0,
            was_running: false,
            cap,
            sr,
            samples_per_bar: (sr * 2.0) as u64, // recomputed each buffer
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
        let bpm = bpm.clamp(20.0, 999.0);
        let beats = self.atomics.beats_per_bar.load(Ordering::Relaxed).max(1) as f64;
        let samples_per_beat = self.sr * 60.0 / bpm;
        self.samples_per_bar = (samples_per_beat * beats).max(1.0) as u64;

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
        let has = self.pads[i].has_content();
        let next = match self.pads[i].state {
            PadState::Empty => PadState::ArmedRec,
            PadState::Stopped => PadState::ArmedPlay,
            PadState::Playing => PadState::ArmedStop,
            // Commit a recording early → play from next bar.
            PadState::Recording => PadState::ArmedPlay,
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

        let mut out_l = 0.0;
        let mut out_r = 0.0;

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
        let p = &mut self.pads[i];
        p.len = target;
        p.pos = 0;
        p.state = PadState::Playing;
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
