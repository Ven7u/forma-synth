use crate::sequencer::{
    chord_name, chord_quality, ScaleType, SeqClockDiv, SeqMode, DEGREE_LABELS, NOTE_NAMES,
};
use crate::ui::design::layout::{note_seq_step, NoteSeqStepState};
use crate::ui::design::mini_bar::{MiniBar, MiniBarOrientation};
use crate::SynthApp;
use eframe::egui;
use egui::{Color32, CornerRadius, Sense, Stroke, StrokeKind, Vec2};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const SEQ_CHROMATIC: &[u8] = &[
    36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
    60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83,
    84,
];

impl SynthApp {
    pub fn ui_sequencer_panel(&mut self, ui: &mut egui::Ui) {
        let seq_mode = SeqMode::from_u8(self.seq.mode.load(Ordering::Relaxed));
        let seq_playing = self.seq.playing.load(Ordering::Relaxed);

        // --- Shared toolbar ---
        // Use horizontal_wrapped so the toolbar gracefully wraps onto multiple
        // rows on narrow windows instead of overflowing the panel.
        // TODO(Phase 6): redesign as tier-stratified rows — transport (Tier 1) /
        // mode + BPM (Tier 2) / length + division behind a menu (Tier 3).
        ui.horizontal_wrapped(|ui| {
            // Mode tabs
            for &mode in &[SeqMode::NoteSeq, SeqMode::ChordSeq] {
                let active = seq_mode == mode;
                let label = egui::RichText::new(mode.label())
                    .color(if active {
                        self.theme.c(&self.theme.accent)
                    } else {
                        Color32::GRAY
                    })
                    .strong();
                let tip = match mode {
                    SeqMode::NoteSeq => "Note Sequencer — step-sequence individual notes.",
                    SeqMode::ChordSeq => {
                        "Chord Sequencer — step-sequence chords from a diatonic scale."
                    }
                    SeqMode::ChordKb => unreachable!(),
                };
                if ui.button(label).on_hover_text(tip).clicked() && !active {
                    self.seq.playing.store(false, Ordering::Relaxed);
                    self.seq.current_step.store(0, Ordering::Relaxed);
                    self.seq.mode.store(mode.to_u8(), Ordering::Relaxed);
                }
            }

            ui.separator();

            // Play/Stop
            {
                let bar_quantize = self.seq.bar_quantize.load(Ordering::Relaxed);
                let btn = if seq_playing {
                    "■ Stop"
                } else if self.seq_pending_start {
                    "… Bar"
                } else {
                    "▶ Play"
                };
                let tip = if self.seq_pending_start {
                    "Waiting for the next bar boundary — click to cancel."
                } else if seq_playing {
                    "Stop the sequencer."
                } else if bar_quantize {
                    "Start the sequencer on the next bar boundary (bar-quantize is on)."
                } else {
                    "Start the sequencer immediately."
                };
                if ui.button(btn).on_hover_text(tip).clicked() {
                    if seq_playing || self.seq_pending_start {
                        // Stop: reset to step 0 so next Play starts from the beginning.
                        self.seq.playing.store(false, Ordering::Relaxed);
                        self.seq.current_step.store(0, Ordering::Relaxed);
                        self.seq_pending_start = false;
                    } else if bar_quantize {
                        // Defer: sequencer fires on next bar boundary detected in tick_metronome.
                        self.seq_pending_start = true;
                        self.seq.current_step.store(0, Ordering::Relaxed);
                    } else {
                        // Start from step 0, align metronome to beat 1.
                        self.seq.current_step.store(0, Ordering::Relaxed);
                        self.seq.playing.store(true, Ordering::Relaxed);
                        self.metro_reset();
                        if self.seq.arp_restart.load(Ordering::Relaxed) {
                            self.engine.arp_restart();
                            self.seq.arp_restart.store(false, Ordering::Relaxed);
                        }
                        if self.seq.walker_restart.load(Ordering::Relaxed) {
                            self.engine.walker_restart();
                            self.seq.walker_restart.store(false, Ordering::Relaxed);
                        }
                    }
                }

                // Record button — step entry (stopped) or live overdub (playing).
                // NoteSeq records notes directly; ChordSeq maps played notes to scale degrees.
                if seq_mode != crate::sequencer::SeqMode::ChordKb {
                    let recording = self.seq.recording.load(Ordering::Relaxed);
                    let rec_label = egui::RichText::new("● REC")
                        .color(if recording {
                            egui::Color32::from_rgb(220, 50, 50)
                        } else {
                            Color32::GRAY
                        })
                        .strong();
                    let rec_tip = if recording {
                        if seq_playing {
                            "Live overdub active — notes you play overwrite the current step. Click to stop recording."
                        } else {
                            "Step entry active — each key press fills the highlighted step and advances. Click to stop."
                        }
                    } else if seq_playing {
                        "Start live overdub — notes you play will overwrite steps as the sequencer runs."
                    } else {
                        "Start step entry — press keys to fill steps one by one."
                    };
                    if ui.button(rec_label).on_hover_text(rec_tip).clicked() {
                        let next = !recording;
                        self.seq.recording.store(next, Ordering::Relaxed);
                        if next && !seq_playing {
                            // Reset step cursor to beginning when starting step entry.
                            self.seq.rec_step.store(0, Ordering::Relaxed);
                        }
                    }

                    // REST and ← only matter in step-entry mode (stopped + recording).
                    if recording && !seq_playing {
                        if ui
                            .button("REST")
                            .on_hover_text("Insert a rest (empty step) and advance.")
                            .clicked()
                        {
                            self.seq_record_rest();
                        }
                        if ui
                            .button("←")
                            .on_hover_text("Go back one step.")
                            .clicked()
                        {
                            self.seq_record_back();
                        }
                    }
                }

                // Sequencer BPM — locked to global when seq_sync is active
                let seq_sync_on = self.seq_sync_active();
                if seq_sync_on {
                    self.seq.bpm.store(self.global_bpm, Ordering::Relaxed);
                }
                let mut bpm_val = self.seq.bpm.load(Ordering::Relaxed);
                ui.label("BPM:")
                    .on_hover_text("Sequencer tempo. Follows Global BPM when Sync is enabled.");
                ui.add_enabled_ui(!seq_sync_on, |ui| {
                    if ui
                        .add(egui::Slider::new(&mut bpm_val, 40..=600))
                        .on_hover_text("Sequencer tempo (40–600 BPM).")
                        .changed()
                    {
                        self.seq.bpm.store(bpm_val, Ordering::Relaxed);
                    }
                });
                ui.add_enabled_ui(!self.global_sync, |ui| {
                    let sync_label = egui::RichText::new("Sync").color(if self.seq_sync_active() {
                        self.theme.c(&self.theme.accent)
                    } else {
                        Color32::GRAY
                    });
                    if ui
                        .button(sync_label)
                        .on_hover_text("Lock sequencer BPM to the Global BPM.")
                        .clicked()
                    {
                        self.seq_sync = !self.seq_sync;
                        if self.seq_sync {
                            self.apply_clock_sync();
                        }
                    }
                });

                // Step length selector
                let cur_length = match seq_mode {
                    SeqMode::NoteSeq => self.seq.note_seq.lock().unwrap().length,
                    SeqMode::ChordSeq => self.seq.chord_seq.lock().unwrap().length,
                    SeqMode::ChordKb => unreachable!(),
                };
                ui.label("Steps:")
                    .on_hover_text("Number of steps in the sequencer pattern.");
                for &len in &[8usize, 16, 24] {
                    let active = cur_length == len;
                    let label = egui::RichText::new(format!("{len}")).color(if active {
                        self.theme.c(&self.theme.accent_dim)
                    } else {
                        Color32::GRAY
                    });
                    if ui
                        .button(label)
                        .on_hover_text(format!("Set pattern length to {len} steps."))
                        .clicked()
                    {
                        match seq_mode {
                            SeqMode::NoteSeq => self.seq.note_seq.lock().unwrap().length = len,
                            SeqMode::ChordSeq => self.seq.chord_seq.lock().unwrap().length = len,
                            SeqMode::ChordKb => {}
                        }
                        let current = self.seq.current_step.load(Ordering::Relaxed);
                        if current >= len {
                            self.seq.current_step.store(0, Ordering::Relaxed);
                        }
                    }
                }

                // Clock division selector
                ui.label("Div:")
                    .on_hover_text("Duration of each step. Short values = fast sequencing; long values = slow chord changes.");
                let (cur_div, div_atomic) = match seq_mode {
                    SeqMode::NoteSeq => (
                        self.seq.note_div.load(Ordering::Relaxed),
                        Arc::clone(&self.seq.note_div),
                    ),
                    SeqMode::ChordSeq => (
                        self.seq.chord_div.load(Ordering::Relaxed),
                        Arc::clone(&self.seq.chord_div),
                    ),
                    SeqMode::ChordKb => unreachable!(),
                };
                for (i, &label) in SeqClockDiv::LABELS.iter().enumerate() {
                    let active = cur_div == i as u8;
                    let col = if active {
                        self.theme.c(&self.theme.accent_dim)
                    } else {
                        Color32::GRAY
                    };
                    if ui
                        .button(egui::RichText::new(label).color(col))
                        .on_hover_text(format!("Step duration: {} note/bar(s).", label))
                        .clicked()
                        && !active
                    {
                        div_atomic.store(i as u8, Ordering::Relaxed);
                        // Also persist to SynthApp mirror
                        match seq_mode {
                            SeqMode::NoteSeq => self.note_seq_div = i as u8,
                            SeqMode::ChordSeq => self.chord_seq_div = i as u8,
                            SeqMode::ChordKb => {}
                        }
                    }
                }

                // Gate length and timing humanization.
                ui.separator();
                ui.label("Gate:").on_hover_text("Note gate length — how long each note is held within its step. 100% = hold until next step, lower values = shorter, more staccato notes.");
                let mut gate = self.seq.gate.load(Ordering::Relaxed);
                if ui
                    .add(egui::Slider::new(&mut gate, 1u8..=100).suffix("%").fixed_decimals(0))
                    .on_hover_text("1% = very staccato, 90% = slight separation (default), 100% = legato/tied.")
                    .changed()
                {
                    self.seq.gate.store(gate, Ordering::Relaxed);
                }
                ui.label("Human:").on_hover_text("Timing humanization — each step fires slightly early or late by a random amount. 0 = perfectly on-grid.");
                let mut human = self.seq.humanize.load(Ordering::Relaxed);
                if ui
                    .add(egui::Slider::new(&mut human, 0u8..=100).suffix("%").fixed_decimals(0))
                    .on_hover_text("0% = robot-tight, 100% = maximum humanization (±45% of step duration).")
                    .changed()
                {
                    self.seq.humanize.store(human, Ordering::Relaxed);
                }

                ui.separator();

                // Random fill
                if ui
                    .button("RND")
                    .on_hover_text("Randomly fill all steps with notes.")
                    .clicked()
                {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut h = DefaultHasher::new();
                    std::time::SystemTime::now().hash(&mut h);
                    let seed = h.finish();
                    match seq_mode {
                        SeqMode::NoteSeq => {
                            let mut ns = self.seq.note_seq.lock().unwrap();
                            let len = ns.length;
                            for i in 0..len {
                                ns.steps[i] = seed.wrapping_shr(i as u32) & 1 == 1;
                                ns.notes[i] = SEQ_CHROMATIC[(seed.wrapping_shr((i * 3) as u32)
                                    & 0xff)
                                    as usize
                                    % SEQ_CHROMATIC.len()];
                            }
                        }
                        SeqMode::ChordSeq => {
                            let mut cs = self.seq.chord_seq.lock().unwrap();
                            let len = cs.length;
                            for i in 0..len {
                                cs.steps[i] = seed.wrapping_shr(i as u32) & 1 == 1;
                                cs.degrees[i] =
                                    (seed.wrapping_shr((i * 4) as u32) & 0xff) as usize % 7;
                            }
                        }
                        SeqMode::ChordKb => {}
                    }
                }

                // Euclidean rhythm generator.
                let euclid_label = egui::RichText::new("EUCLID").color(
                    if self.seq_euclid_open { self.theme.c(&self.theme.accent) } else { Color32::GRAY }
                );
                if ui.button(euclid_label).on_hover_text("Generate a Euclidean (evenly-spaced) rhythm pattern.").clicked() {
                    self.seq_euclid_open = !self.seq_euclid_open;
                    if self.seq_euclid_open {
                        // Pre-fill hits to half the current pattern length.
                        let cur_len = match seq_mode {
                            SeqMode::NoteSeq => self.seq.note_seq.lock().unwrap().length,
                            SeqMode::ChordSeq => self.seq.chord_seq.lock().unwrap().length,
                            SeqMode::ChordKb => 8,
                        };
                        self.seq_euclid_hits = self.seq_euclid_hits.min(cur_len);
                    }
                }

                // Transpose
                ui.separator();
                ui.label(egui::RichText::new("Transpose:").weak().small());
                for (label, semitones, tip) in [
                    ("−12", -12i32, "Down one octave"),
                    ("−1",  -1,     "Down one semitone"),
                    ("+1",   1,     "Up one semitone"),
                    ("+12", 12,     "Up one octave"),
                ] {
                    if ui.button(egui::RichText::new(label).small()).on_hover_text(tip).clicked() {
                        match seq_mode {
                            SeqMode::NoteSeq => {
                                let mut ns = self.seq.note_seq.lock().unwrap();
                                let len = ns.length;
                                for i in 0..len {
                                    ns.notes[i] = (ns.notes[i] as i32 + semitones)
                                        .clamp(21, 108) as u8;
                                }
                            }
                            SeqMode::ChordSeq => {
                                let mut cs = self.seq.chord_seq.lock().unwrap();
                                if semitones.abs() == 12 {
                                    cs.octave = (cs.octave + semitones / 12).clamp(1, 7);
                                } else {
                                    cs.root = ((cs.root as i32 + semitones).rem_euclid(12)) as u8;
                                }
                            }
                            SeqMode::ChordKb => {}
                        }
                    }
                }

                // Pattern library button — mode-appropriate popup
                ui.separator();
                let lib_open = match seq_mode {
                    SeqMode::NoteSeq => self.show_melody_library,
                    SeqMode::ChordSeq => self.show_harmony_library,
                    SeqMode::ChordKb => false,
                };
                let lib_label = egui::RichText::new("Library")
                    .color(if lib_open { self.theme.c(&self.theme.accent) } else { Color32::GRAY });
                if ui.button(lib_label).on_hover_text("Open the pattern library to load a preset into this sequencer.").clicked() {
                    match seq_mode {
                        SeqMode::NoteSeq => {
                            self.show_melody_library = !self.show_melody_library;
                            self.show_harmony_library = false;
                        }
                        SeqMode::ChordSeq => {
                            self.show_harmony_library = !self.show_harmony_library;
                            self.show_melody_library = false;
                        }
                        SeqMode::ChordKb => {}
                    }
                    self.pattern_lib_category = None;
                    self.harmony_lib_selected = None;
                    self.melody_lib_selected = None;
                }
            }

            // Euclidean controls (shown inline below toolbar when open).
            if self.seq_euclid_open && seq_mode != SeqMode::ChordKb {
                let cur_len = match seq_mode {
                    SeqMode::NoteSeq => self.seq.note_seq.lock().unwrap().length,
                    SeqMode::ChordSeq => self.seq.chord_seq.lock().unwrap().length,
                    SeqMode::ChordKb => 8,
                };
                self.seq_euclid_hits = self.seq_euclid_hits.clamp(1, cur_len);
                self.seq_euclid_offset %= cur_len;
                ui.horizontal(|ui| {
                    ui.label("Hits:");
                    let mut h = self.seq_euclid_hits;
                    if ui.add(egui::Slider::new(&mut h, 1..=cur_len)).changed() {
                        self.seq_euclid_hits = h;
                    }
                    ui.label("Offset:");
                    let mut off = self.seq_euclid_offset;
                    if ui.add(egui::Slider::new(&mut off, 0..=cur_len - 1)).changed() {
                        self.seq_euclid_offset = off;
                    }
                    if ui.button("Apply").on_hover_text("Fill the step on/off pattern with the Euclidean rhythm.").clicked() {
                        let pattern = crate::sequencer::bjorklund(self.seq_euclid_hits, cur_len, self.seq_euclid_offset);
                        match seq_mode {
                            SeqMode::NoteSeq => {
                                let mut ns = self.seq.note_seq.lock().unwrap();
                                for (i, &on) in pattern.iter().enumerate() {
                                    ns.steps[i] = on;
                                }
                            }
                            SeqMode::ChordSeq => {
                                let mut cs = self.seq.chord_seq.lock().unwrap();
                                for (i, &on) in pattern.iter().enumerate() {
                                    cs.steps[i] = on;
                                }
                            }
                            SeqMode::ChordKb => {}
                        }
                        self.seq_euclid_open = false;
                    }
                    if ui.button("✕").clicked() {
                        self.seq_euclid_open = false;
                    }
                });
            }

            // Chord key/scale selector (ChordSeq only)
            if seq_mode == SeqMode::ChordSeq {
                ui.separator();
                ui.label("Key:")
                    .on_hover_text("Root note for the chord scale.");
                let cur_root = self.seq.chord_seq.lock().unwrap().root;
                egui::ComboBox::from_id_salt("chord_root")
                    .selected_text(NOTE_NAMES[cur_root as usize])
                    .show_ui(ui, |ui| {
                        let mut root = cur_root;
                        for (i, name) in NOTE_NAMES.iter().enumerate() {
                            if ui.selectable_value(&mut root, i as u8, *name).changed() {
                                self.seq.chord_seq.lock().unwrap().root = root;
                            }
                        }
                    });
                ui.label("Scale:");
                let cur_scale = self.seq.chord_seq.lock().unwrap().scale;
                for &sc in &[ScaleType::Major, ScaleType::Minor] {
                    let active = cur_scale == sc;
                    let label = egui::RichText::new(sc.label()).color(if active {
                        self.theme.c(&self.theme.accent_dim)
                    } else {
                        Color32::GRAY
                    });
                    if ui
                        .button(label)
                        .on_hover_text(match sc {
                            ScaleType::Major => "Major scale — bright, happy feel.",
                            _ => "Minor scale — dark, moody feel.",
                        })
                        .clicked()
                    {
                        self.seq.chord_seq.lock().unwrap().scale = sc;
                    }
                }

                ui.separator();

                // Voice lead toggle
                let vl = self.seq.chord_seq.lock().unwrap().voice_lead;
                let vl_label = egui::RichText::new("Voice Lead")
                    .color(if vl { self.theme.c(&self.theme.accent) } else { Color32::GRAY });
                if ui
                    .button(vl_label)
                    .on_hover_text(
                        "Auto-pick the inversion that minimises voice movement between steps. \
                         Overrides per-step voicing.",
                    )
                    .clicked()
                {
                    self.seq.chord_seq.lock().unwrap().voice_lead = !vl;
                }
            }
        });

        ui.add_space(4.0);

        match seq_mode {
            SeqMode::NoteSeq => self.ui_note_seq(ui),
            SeqMode::ChordSeq => self.ui_chord_seq(ui),
            SeqMode::ChordKb => {} // handled in keyboard strip
        }

        // Library popups (floating windows; must be called with ctx, not ui)
        if self.show_harmony_library {
            self.ui_harmony_library_window(ui.ctx());
        }
        if self.show_melody_library {
            self.ui_melody_library_window(ui.ctx());
        }
    }

    fn ui_note_seq(&mut self, ui: &mut egui::Ui) {
        let bar_area_h = 64.0;
        let seq_playing = self.seq.playing.load(Ordering::Relaxed);
        let seq_current_step = self.seq.current_step.load(Ordering::Relaxed);
        let recording = self.seq.recording.load(Ordering::Relaxed);
        let rec_step = self.seq.rec_step.load(Ordering::Relaxed);

        let (length, midi_min, midi_max) = {
            let ns = self.seq.note_seq.lock().unwrap();
            (
                ns.length,
                *SEQ_CHROMATIC.first().unwrap() as f32,
                *SEQ_CHROMATIC.last().unwrap() as f32,
            )
        };

        // Step pads use a tighter gap than the global item_spacing —
        // per the design system spec (04-components.md §StepPad).
        const MIN_STEP_W: f32 = 18.0;
        let step_gap = self.theme.sp_xxs;
        let n = length as f32;
        let natural_w = (ui.available_width() - step_gap * (n - 1.0)) / n;
        let step_w = natural_w.max(MIN_STEP_W);
        let needs_scroll = natural_w < MIN_STEP_W;

        // Render closure — captures `self`, called from exactly one branch
        // below. Lets us mount the ScrollArea conditionally so the wide-window
        // path stays free of any scroll-area machinery.
        let render = |this: &mut Self, ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = step_gap;
            for i in 0..length {
                // Snapshot the step's mutable fields out of the lock so the
                // design-system pattern can drive them without holding the
                // mutex across painter calls. Writes go back in one short
                // critical section after the pattern returns.
                let (mut is_on, mut note, mut drag_accum, mut velocity, mut probability) = {
                    let ns = this.seq.note_seq.lock().unwrap();
                    (
                        ns.steps[i],
                        ns.notes[i],
                        ns.drag_accum[i],
                        ns.velocities[i],
                        ns.probabilities[i],
                    )
                };
                let is_current = seq_playing && seq_current_step == i;
                let is_rec_cursor = recording && !seq_playing && rec_step == i;
                let note_label = super::midi_note_name(note).to_string();

                let events = note_seq_step(
                    ui,
                    NoteSeqStepState {
                        is_on: &mut is_on,
                        note: &mut note,
                        drag_accum: &mut drag_accum,
                        velocity: &mut velocity,
                        probability: &mut probability,
                        midi_min,
                        midi_max,
                        is_current,
                        is_rec_cursor,
                    },
                    &note_label,
                    step_w,
                    bar_area_h,
                    &this.theme,
                );

                if events.pad_clicked {
                    is_on = !is_on;
                }

                // Snap `note` to the nearest legal chromatic index so the
                // pitch bar's continuous range still produces the same
                // discrete pitches the engine expects.
                let nearest = SEQ_CHROMATIC
                    .iter()
                    .min_by_key(|&&c| ((c as i32) - (note as i32)).abs())
                    .copied()
                    .unwrap_or(note);
                note = nearest;

                // Write back any deltas in a single lock acquisition.
                if events.pad_clicked || events.changed || drag_accum != 0.0 {
                    let mut ns = this.seq.note_seq.lock().unwrap();
                    ns.steps[i] = is_on;
                    ns.notes[i] = note;
                    ns.drag_accum[i] = drag_accum;
                    ns.velocities[i] = velocity;
                    ns.probabilities[i] = probability;
                }
            }
        });
        };

        if needs_scroll {
            egui::ScrollArea::horizontal()
                .id_salt("note_seq_h_scroll")
                // Step cells own click+drag — wheel and scrollbar drag still scroll.
                .drag_to_scroll(false)
                .show(ui, |ui| render(self, ui));
        } else {
            render(self, ui);
        }
    }

    fn ui_chord_seq(&mut self, ui: &mut egui::Ui) {
        let bar_area_h = 64.0;
        let seq_playing = self.seq.playing.load(Ordering::Relaxed);
        let seq_current_step = self.seq.current_step.load(Ordering::Relaxed);
        let recording = self.seq.recording.load(Ordering::Relaxed);
        let rec_step = self.seq.rec_step.load(Ordering::Relaxed);

        let (length, scale, root) = {
            let cs = self.seq.chord_seq.lock().unwrap();
            (cs.length, cs.scale, cs.root)
        };

        const MIN_STEP_W: f32 = 18.0;
        let step_gap = self.theme.sp_xxs;
        let n = length as f32;
        let natural_w = (ui.available_width() - step_gap * (n - 1.0)) / n;
        let step_w = natural_w.max(MIN_STEP_W);
        let needs_scroll = natural_w < MIN_STEP_W;

        let render = |this: &mut Self, ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = step_gap;
            for i in 0..length {
                ui.vertical(|ui| {
                    ui.set_width(step_w);
                    let (is_on, degree, chord_type, oct_off) = {
                        let cs = this.seq.chord_seq.lock().unwrap();
                        (
                            cs.steps[i],
                            cs.degrees[i],
                            cs.chord_types[i],
                            cs.octave_offsets[i],
                        )
                    };
                    let is_current = seq_playing && seq_current_step == i;
                    let is_rec_cursor = recording && !seq_playing && rec_step == i;

                    let (bar_resp, painter) =
                        ui.allocate_painter(Vec2::new(step_w, bar_area_h), Sense::click_and_drag());
                    let r = bar_resp.rect;
                    painter.rect_filled(
                        r,
                        CornerRadius::same(this.theme.rounding_sm as u8),
                        this.theme.c(&this.theme.bg_seq_bar),
                    );
                    let t = degree as f32 / 6.0;
                    let bar_h = (t * (bar_area_h - 4.0)).max(4.0);
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(r.min.x + 2.0, r.max.y - bar_h - 2.0),
                        Vec2::new(step_w - 4.0, bar_h),
                    );
                    let quality = chord_quality(scale, degree);
                    let bar_color = if is_current {
                        this.theme.c(&this.theme.seq_current)
                    } else if !is_on {
                        this.theme.c(&this.theme.seq_note_bar_off)
                    } else if quality == "m" {
                        this.theme.c(&this.theme.seq_chord_minor)
                    } else if quality == "°" {
                        this.theme.c(&this.theme.seq_chord_dim)
                    } else {
                        this.theme.c(&this.theme.seq_chord_major)
                    };
                    painter.rect_filled(bar_rect, CornerRadius::same(this.theme.rounding_xs as u8), bar_color);
                    // Rec cursor border.
                    if is_rec_cursor {
                        painter.rect_stroke(
                            r,
                            CornerRadius::same(this.theme.rounding_sm as u8),
                            Stroke::new(this.theme.stroke_active, this.theme.c(&this.theme.seq_rec_cursor)),
                            StrokeKind::Middle,
                        );
                    }

                    let cname = chord_name(root, scale, degree);
                    let primary = this.theme.c(&this.theme.text_primary);
                    let secondary = this.theme.c(&this.theme.text_secondary);
                    let disabled = this.theme.c(&this.theme.text_disabled);
                    painter.text(
                        egui::pos2(r.center().x, r.min.y + 10.0),
                        egui::Align2::CENTER_CENTER,
                        &cname,
                        this.theme.font_value(),
                        if is_on { primary } else { secondary },
                    );
                    painter.text(
                        egui::pos2(r.center().x, r.min.y + 22.0),
                        egui::Align2::CENTER_CENTER,
                        DEGREE_LABELS[degree],
                        this.theme.font_micro(),
                        if is_on { secondary } else { disabled },
                    );
                    // Chord type label (bottom of bar). Uses the accent
                    // family to differentiate from the chord-name and
                    // degree labels stacked above.
                    let chord_type_color = if is_on {
                        this.theme.c(&this.theme.accent_dim)
                    } else {
                        disabled
                    };
                    painter.text(
                        egui::pos2(r.center().x, r.max.y - 8.0),
                        egui::Align2::CENTER_CENTER,
                        chord_type.label(),
                        this.theme.font_micro(),
                        chord_type_color,
                    );

                    // Left drag: change degree.
                    if bar_resp.dragged() {
                        let mut cs = this.seq.chord_seq.lock().unwrap();
                        cs.drag_accum[i] -= bar_resp.drag_delta().y * 0.6;
                        let steps = cs.drag_accum[i] as i32;
                        if steps != 0 {
                            cs.drag_accum[i] -= steps as f32;
                            cs.degrees[i] = (degree as i32 + steps).clamp(0, 6) as usize;
                        }
                    }
                    if bar_resp.drag_stopped() {
                        this.seq.chord_seq.lock().unwrap().drag_accum[i] = 0.0;
                    }
                    // Scroll wheel: change degree.
                    if bar_resp.hovered() {
                        let scroll = ui.input(|inp| inp.smooth_scroll_delta.y);
                        if scroll != 0.0 {
                            let delta = if scroll > 0.0 { 1i32 } else { -1 };
                            let mut cs = this.seq.chord_seq.lock().unwrap();
                            cs.degrees[i] = (degree as i32 + delta).clamp(0, 6) as usize;
                        }
                    }
                    // Right-click: cycle chord type.
                    if bar_resp.secondary_clicked() {
                        use crate::sequencer::ChordType;
                        let all = ChordType::all();
                        let cur_idx = all.iter().position(|&t| t == chord_type).unwrap_or(0);
                        let next = all[(cur_idx + 1) % all.len()];
                        this.seq.chord_seq.lock().unwrap().chord_types[i] = next;
                    }

                    // ── Step pad (on/off) ────────────────────────────────
                    let fill = if is_current {
                        this.theme.c(&this.theme.seq_current)
                    } else if is_on {
                        this.theme.c(&this.theme.seq_step_on)
                    } else {
                        this.theme.c(&this.theme.seq_step_off)
                    };
                    let pad_border = if is_current {
                        this.theme.c(&this.theme.text_primary)
                    } else {
                        this.theme.c(&this.theme.border)
                    };
                    let (pad_resp, painter) = ui.allocate_painter(Vec2::new(step_w, 28.0), Sense::click());
                    painter.rect_filled(pad_resp.rect, CornerRadius::same(this.theme.rounding_sm as u8), fill);
                    painter.rect_stroke(
                        pad_resp.rect,
                        CornerRadius::same(this.theme.rounding_sm as u8),
                        Stroke::new(this.theme.stroke_ui, pad_border),
                        StrokeKind::Middle,
                    );
                    if pad_resp.clicked() {
                        let mut cs = this.seq.chord_seq.lock().unwrap();
                        cs.steps[i] = !cs.steps[i];
                    }

                    // ── Velocity MiniBar ─────────────────────────────────
                    let mut velocity = this.seq.chord_seq.lock().unwrap().velocities[i];
                    let mut vel_f = velocity as f32;
                    let vel_label = format!("{velocity}");
                    MiniBar::new(
                        &mut vel_f,
                        0.0..=127.0,
                        MiniBarOrientation::Horizontal,
                        Vec2::new(step_w, 14.0),
                    )
                    .fill(this.theme.c(&this.theme.seq_velocity_bar))
                    .label(vel_label, this.theme.font_micro(), this.theme.c(&this.theme.text_primary))
                    .show(ui, &this.theme);
                    let new_vel = vel_f.round().clamp(0.0, 127.0) as u8;
                    if new_vel != velocity {
                        this.seq.chord_seq.lock().unwrap().velocities[i] = new_vel;
                        velocity = new_vel;
                    }
                    let _ = velocity;

                    // ── Probability MiniBar ──────────────────────────────
                    let probability = this.seq.chord_seq.lock().unwrap().probabilities[i];
                    let mut prob_f = probability as f32;
                    MiniBar::new(
                        &mut prob_f,
                        0.0..=100.0,
                        MiniBarOrientation::Horizontal,
                        Vec2::new(step_w, 10.0),
                    )
                    .zoned(
                        50.0,
                        100.0,
                        this.theme.c(&this.theme.seq_prob_low),
                        this.theme.c(&this.theme.seq_prob_mid),
                        this.theme.c(&this.theme.seq_prob_high),
                    )
                    .show(ui, &this.theme);
                    let new_prob = prob_f.round().clamp(0.0, 100.0) as u8;
                    if new_prob != probability {
                        this.seq.chord_seq.lock().unwrap().probabilities[i] = new_prob;
                    }

                    // ── Octave offset row ────────────────────────────────
                    // Click left half = -1, right half = +1.
                    let oct_h = 14.0;
                    let (oct_resp, painter) =
                        ui.allocate_painter(Vec2::new(step_w, oct_h), Sense::click());
                    let or_ = oct_resp.rect;
                    let oct_t = (oct_off + 2) as f32 / 4.0;
                    painter.rect_filled(or_, CornerRadius::same(this.theme.rounding_xs as u8), this.theme.c(&this.theme.bg_sunken));
                    let oct_fill_w = oct_t * or_.width();
                    painter.rect_filled(
                        egui::Rect::from_min_size(or_.min, Vec2::new(oct_fill_w, oct_h)),
                        CornerRadius::same(this.theme.rounding_xs as u8),
                        this.theme.c(&this.theme.seq_octave_bar),
                    );
                    let oct_label = match oct_off {
                        0 => "oct".to_string(),
                        n if n > 0 => format!("+{n}"),
                        n => format!("{n}"),
                    };
                    painter.text(
                        or_.center(),
                        egui::Align2::CENTER_CENTER,
                        oct_label,
                        this.theme.font_micro(),
                        this.theme.c(&this.theme.text_primary),
                    );
                    if oct_resp.clicked() {
                        if let Some(pos) = oct_resp.interact_pointer_pos() {
                            let mid = or_.center().x;
                            let new_off = if pos.x > mid {
                                (oct_off + 1).min(2)
                            } else {
                                (oct_off - 1).max(-2)
                            };
                            this.seq.chord_seq.lock().unwrap().octave_offsets[i] = new_off;
                        }
                    }

                    // ── Voicing row ──────────────────────────────────────
                    // Click left = prev inversion, right = next inversion.
                    // Voice-lead-active variant uses chord_major (green hue)
                    // for the label; non-VL uses the velocity-bar color so
                    // the row reads as a velocity sibling.
                    let voicing_h = 12.0;
                    let voicing = this.seq.chord_seq.lock().unwrap().voicings[i];
                    let voice_lead = this.seq.chord_seq.lock().unwrap().voice_lead;
                    let (v_resp, painter) =
                        ui.allocate_painter(Vec2::new(step_w, voicing_h), Sense::click());
                    let v_rect = v_resp.rect;
                    painter.rect_filled(v_rect, CornerRadius::same(this.theme.rounding_xs as u8), this.theme.c(&this.theme.bg_sunken));
                    let vcolor = if voice_lead {
                        this.theme.c(&this.theme.seq_chord_major)
                    } else {
                        this.theme.c(&this.theme.seq_velocity_bar)
                    };
                    let vlabel = if voice_lead { "VL" } else { voicing.short() };
                    painter.text(
                        v_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        vlabel,
                        this.theme.font_micro(),
                        vcolor,
                    );
                    if v_resp.clicked() && !voice_lead {
                        if let Some(pos) = v_resp.interact_pointer_pos() {
                            let next = if pos.x > v_rect.center().x {
                                voicing.next()
                            } else {
                                voicing.prev()
                            };
                            this.seq.chord_seq.lock().unwrap().voicings[i] = next;
                        }
                    }
                });
            }
        });
        };

        if needs_scroll {
            egui::ScrollArea::horizontal()
                .id_salt("chord_seq_h_scroll")
                .drag_to_scroll(false)
                .show(ui, |ui| render(self, ui));
        } else {
            render(self, ui);
        }
    }

    // -------------------------------------------------------------------------
    // Pattern library windows
    // -------------------------------------------------------------------------

    fn ui_harmony_library_window(&mut self, ctx: &egui::Context) {
        use crate::ui::pattern_library::{apply_harmony, harmony_categories, HARMONY_PRESETS};

        let mut open = self.show_harmony_library;
        egui::Window::new("Harmony Library")
            .id(egui::Id::new("harmony_lib_window"))
            .open(&mut open)
            .resizable(true)
            .min_width(340.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Click a preset to preview, then Load to apply.")
                        .weak()
                        .small(),
                );
                ui.separator();

                let categories = harmony_categories();

                // Category filter chips
                ui.horizontal_wrapped(|ui| {
                    let all_active = self.pattern_lib_category.is_none();
                    let all_label = egui::RichText::new("All").small().color(if all_active {
                        self.theme.c(&self.theme.accent)
                    } else {
                        Color32::GRAY
                    });
                    if ui.button(all_label).clicked() {
                        self.pattern_lib_category = None;
                    }
                    for &cat in &categories {
                        let active = self.pattern_lib_category == Some(cat);
                        let label = egui::RichText::new(cat).small().color(if active {
                            self.theme.c(&self.theme.accent)
                        } else {
                            Color32::GRAY
                        });
                        if ui.button(label).clicked() {
                            self.pattern_lib_category = if active { None } else { Some(cat) };
                        }
                    }
                });
                ui.separator();

                // Preset list — data-driven (HARMONY_PRESETS table).
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for (idx, preset) in HARMONY_PRESETS.iter().enumerate() {
                            if let Some(cat) = self.pattern_lib_category {
                                if preset.category != cat {
                                    continue;
                                }
                            }
                            let selected = self.harmony_lib_selected == Some(idx);
                            ui.horizontal(|ui| {
                                // Category tag
                                ui.label(
                                    egui::RichText::new(preset.category)
                                        .small()
                                        .weak()
                                        .color(Color32::from_gray(100)),
                                );
                                // Name
                                let name_label =
                                    egui::RichText::new(preset.name).color(if selected {
                                        self.theme.c(&self.theme.accent)
                                    } else {
                                        Color32::WHITE
                                    });
                                if ui.selectable_label(selected, name_label).clicked() {
                                    self.harmony_lib_selected = Some(idx);
                                }
                                // Step count badge
                                ui.label(
                                    egui::RichText::new(format!("{}s", preset.length))
                                        .small()
                                        .weak(),
                                );
                            });

                            // Preview row: degree pills
                            if selected {
                                ui.horizontal_wrapped(|ui| {
                                    for &d in preset.degrees {
                                        ui.label(
                                            egui::RichText::new(
                                                crate::sequencer::DEGREE_LABELS[d % 7],
                                            )
                                            .monospace()
                                            .small()
                                            .color(self.theme.c(&self.theme.accent_dim)),
                                        );
                                    }
                                });
                            }
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    let can_load = self.harmony_lib_selected.is_some();
                    if ui
                        .add_enabled(can_load, egui::Button::new("Load"))
                        .on_hover_text("Load the selected progression into the Chord Sequencer.")
                        .clicked()
                    {
                        if let Some(idx) = self.harmony_lib_selected {
                            let preset = &HARMONY_PRESETS[idx];
                            apply_harmony(&mut self.seq.chord_seq.lock().unwrap(), preset);
                            // Clamp current step
                            let cur = self.seq.current_step.load(Ordering::Relaxed);
                            if cur >= preset.length {
                                self.seq.current_step.store(0, Ordering::Relaxed);
                            }
                            self.show_harmony_library = false;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_harmony_library = false;
                    }
                });
            });
        self.show_harmony_library = open;
    }

    fn ui_melody_library_window(&mut self, ctx: &egui::Context) {
        use crate::ui::midi_note_full;
        use crate::ui::pattern_library::{apply_melody, melody_categories, MELODY_PRESETS};

        let mut open = self.show_melody_library;
        egui::Window::new("Melody Library")
            .id(egui::Id::new("melody_lib_window"))
            .open(&mut open)
            .resizable(true)
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Presets are transposed to your current keyboard octave on load.")
                        .weak()
                        .small(),
                );
                ui.separator();

                let categories = melody_categories();

                ui.horizontal_wrapped(|ui| {
                    let all_active = self.pattern_lib_category.is_none();
                    let all_label = egui::RichText::new("All")
                        .small()
                        .color(if all_active { self.theme.c(&self.theme.accent) } else { Color32::GRAY });
                    if ui.button(all_label).clicked() {
                        self.pattern_lib_category = None;
                    }
                    for &cat in &categories {
                        let active = self.pattern_lib_category == Some(cat);
                        let label = egui::RichText::new(cat)
                            .small()
                            .color(if active { self.theme.c(&self.theme.accent) } else { Color32::GRAY });
                        if ui.button(label).clicked() {
                            self.pattern_lib_category = if active { None } else { Some(cat) };
                        }
                    }
                });
                ui.separator();

                // Base MIDI: C in the user's current piano octave
                let base_midi = ((self.piano_octave * 12) + 12).clamp(21, 108) as u8;

                // Data-driven: MELODY_PRESETS table.
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for (idx, preset) in MELODY_PRESETS.iter().enumerate() {
                        if let Some(cat) = self.pattern_lib_category {
                            if preset.category != cat {
                                continue;
                            }
                        }
                        let selected = self.melody_lib_selected == Some(idx);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(preset.category)
                                    .small()
                                    .weak()
                                    .color(Color32::from_gray(100)),
                            );
                            let name_label = egui::RichText::new(preset.name).color(
                                if selected { self.theme.c(&self.theme.accent) } else { Color32::WHITE },
                            );
                            if ui.selectable_label(selected, name_label).clicked() {
                                self.melody_lib_selected = Some(idx);
                            }
                            ui.label(
                                egui::RichText::new(format!("{}s", preset.length))
                                    .small()
                                    .weak(),
                            );
                        });

                        // Preview: note names at current octave
                        if selected {
                            ui.horizontal_wrapped(|ui| {
                                for (&offset, &active) in preset.notes.iter().zip(preset.active.iter()) {
                                    let midi = (base_midi as i32 + offset as i32).clamp(21, 108) as u8;
                                    let name = if active {
                                        midi_note_full(midi)
                                    } else {
                                        "—".to_string()
                                    };
                                    ui.label(
                                        egui::RichText::new(name)
                                            .monospace()
                                            .small()
                                            .color(if active {
                                                self.theme.c(&self.theme.accent_dim)
                                            } else {
                                                Color32::from_gray(80)
                                            }),
                                    );
                                }
                            });
                        }
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    let can_load = self.melody_lib_selected.is_some();
                    if ui
                        .add_enabled(can_load, egui::Button::new("Load"))
                        .on_hover_text("Load the selected melody into the Note Sequencer, transposed to your keyboard octave.")
                        .clicked()
                    {
                        if let Some(idx) = self.melody_lib_selected {
                            let preset = &MELODY_PRESETS[idx];
                            apply_melody(&mut self.seq.note_seq.lock().unwrap(), preset, base_midi);
                            let cur = self.seq.current_step.load(Ordering::Relaxed);
                            if cur >= preset.length {
                                self.seq.current_step.store(0, Ordering::Relaxed);
                            }
                            self.show_melody_library = false;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_melody_library = false;
                    }
                });
            });
        self.show_melody_library = open;
    }
}
