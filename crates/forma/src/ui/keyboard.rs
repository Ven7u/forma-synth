use crate::sequencer::{
    apply_voicing, chord_name, chord_quality, scale_pitch_classes, ChordType, ScaleType, SeqMode,
    VoicingType, CHORD_KB_COLS, CHORD_KB_ROWS, DEGREE_LABELS, NOTE_NAMES,
};
use crate::ui::design::chip::chip_selector;
use crate::ui::design::chord_pad::{chord_pad, parse_quality, ChordPadState};
use crate::ui::design::piano::{piano, KeyVisualState, PianoConfig, PianoSize};
use crate::ui::design::toggle::{toggle_button, ToggleSize};
use crate::ui::design::Tier;
use crate::ui::layout::AppMode;
use crate::SynthApp;
use eframe::egui;
use egui::Vec2;

#[allow(dead_code)]
const WHITE_SEMITONES: &[i32] = &[0, 2, 4, 5, 7, 9, 11];
#[allow(dead_code)]
const BLACK_SEMITONES: &[Option<i32>] = &[Some(1), Some(3), None, Some(6), Some(8), Some(10), None];

const KEY_MAP: &[(egui::Key, i32)] = &[
    (egui::Key::A, 0),          // C
    (egui::Key::W, 1),          // C#
    (egui::Key::S, 2),          // D
    (egui::Key::E, 3),          // D#
    (egui::Key::D, 4),          // E
    (egui::Key::F, 5),          // F
    (egui::Key::T, 6),          // F#
    (egui::Key::G, 7),          // G
    (egui::Key::Y, 8),          // G#
    (egui::Key::H, 9),          // A
    (egui::Key::U, 10),         // A#
    (egui::Key::J, 11),         // B
    (egui::Key::K, 12),         // C
    (egui::Key::O, 13),         // C#
    (egui::Key::L, 14),         // D
    (egui::Key::P, 15),         // D#
    (egui::Key::Semicolon, 16), // E
    (egui::Key::Quote, 17),     // F
];

/// 88-key piano: A0 (MIDI 21) to C8 (MIDI 108).
const PIANO_FIRST_MIDI: u8 = 21;
const PIANO_LAST_MIDI: u8 = 108;

impl SynthApp {
    /// Process keyboard input every frame regardless of which tab is visible.
    /// Call this from the main update loop, not from a tab panel.
    pub fn tick_keyboard_input(&mut self, ctx: &egui::Context) {
        #[allow(dead_code)]
        const WHITE_KEYS: &[egui::Key] = &[
            egui::Key::A,
            egui::Key::S,
            egui::Key::D,
            egui::Key::F,
            egui::Key::G,
            egui::Key::H,
            egui::Key::J,
        ];

        // F1–F4 → switch focused synth track; F5 → Drum Machine mode.
        for (key, track) in [
            (egui::Key::F1, 0usize),
            (egui::Key::F2, 1),
            (egui::Key::F3, 2),
            (egui::Key::F4, 3),
        ] {
            if ctx.input(|i| i.key_pressed(key)) {
                self.switch_focused_track(track);
                // Bring up LIVE mode so the rig strip is visible.
                if self.app_mode == AppMode::DrumMachine || self.app_mode == AppMode::Studio {
                    self.app_mode = AppMode::Live;
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
            self.app_mode = AppMode::DrumMachine;
        }

        // Space bar toggles freeze (when no text widget has focus).
        let space_pressed = ctx.input(|inp| inp.key_pressed(egui::Key::Space));
        if space_pressed && !ctx.memory(|m| m.focused().is_some()) {
            self.kb_freeze = !self.kb_freeze;
            if !self.kb_freeze {
                let frozen: Vec<u8> = self.frozen_notes.drain().collect();
                for n in frozen {
                    self.push_note_off(n);
                }
            }
        }

        // Retrigger held chord-KB pads if voicing changed since last note-on.
        if self.kb_chord_mode && self.kb_voicing != self.kb_voicing_applied {
            let old_v = self.kb_voicing_applied;
            let new_v = self.kb_voicing;
            // Collect (old_notes, new_notes) for every held pad before any mutable borrows.
            let mut retriggers: Vec<(Vec<u8>, Vec<u8>)> = self
                .chord_kb
                .kb_held
                .iter()
                .map(|&(row, col)| {
                    (
                        apply_voicing(self.chord_kb.chord_notes_for(row, col), old_v),
                        apply_voicing(self.chord_kb.chord_notes_for(row, col), new_v),
                    )
                })
                .collect();
            if let Some((row, col)) = self.chord_kb.held_pad {
                retriggers.push((
                    apply_voicing(self.chord_kb.chord_notes_for(row, col), old_v),
                    apply_voicing(self.chord_kb.chord_notes_for(row, col), new_v),
                ));
            }
            for (old_notes, new_notes) in retriggers {
                for n in &old_notes {
                    self.engine.note_off(*n);
                }
                for n in &new_notes {
                    self.engine.note_on(*n, self.piano_velocity);
                }
            }
            // Update frozen notes to the new voicing so freeze-held chords retrigger cleanly.
            let old_frozen: Vec<u8> = self.frozen_notes.iter().copied().collect();
            if !old_frozen.is_empty() {
                // frozen_notes are individual MIDI values — swap any that match old voiced notes.
                // Rebuild the set with updated values.
                let mut new_frozen = std::collections::HashSet::new();
                for n in old_frozen {
                    // Try to find which pad this note belonged to and remap it.
                    let mut remapped = false;
                    'outer: for row in 0..crate::sequencer::CHORD_KB_ROWS {
                        for col in 0..crate::sequencer::CHORD_KB_COLS {
                            let old_voiced =
                                apply_voicing(self.chord_kb.chord_notes_for(row, col), old_v);
                            if old_voiced.contains(&n) {
                                for m in
                                    apply_voicing(self.chord_kb.chord_notes_for(row, col), new_v)
                                {
                                    self.engine.note_off(n);
                                    self.engine.note_on(m, self.piano_velocity);
                                    new_frozen.insert(m);
                                }
                                remapped = true;
                                break 'outer;
                            }
                        }
                    }
                    if !remapped {
                        new_frozen.insert(n);
                    }
                }
                self.frozen_notes = new_frozen;
            }
            self.kb_voicing_applied = new_v;
        }

        // Arrow keys = momentary voicing (chord mode only); release to return to Root.
        // No arrow = Root  ↑ = 1st  ↓ = 2nd  → = Open  ← = Full
        if self.kb_chord_mode && !ctx.memory(|m| m.focused().is_some()) {
            self.kb_voicing = ctx.input(|inp| {
                if inp.key_down(egui::Key::ArrowUp) {
                    VoicingType::First
                } else if inp.key_down(egui::Key::ArrowDown) {
                    VoicingType::Second
                } else if inp.key_down(egui::Key::ArrowRight) {
                    VoicingType::Open
                } else if inp.key_down(egui::Key::ArrowLeft) {
                    VoicingType::Full
                } else {
                    VoicingType::Root
                }
            });
        } else {
            self.kb_voicing = VoicingType::Root;
        }

        if self.kb_chord_mode {
            // 3 rows × 7 cols grid:
            //   Row 0 (triads):  A S D F G H J
            //   Row 1 (7ths):    Q W E R T Y U
            //   Row 2 (sus/add): Z X C V B N M
            const ROW_KEYS: [[egui::Key; 7]; 3] = [
                [
                    egui::Key::Q,
                    egui::Key::W,
                    egui::Key::E,
                    egui::Key::R,
                    egui::Key::T,
                    egui::Key::Y,
                    egui::Key::U,
                ],
                [
                    egui::Key::A,
                    egui::Key::S,
                    egui::Key::D,
                    egui::Key::F,
                    egui::Key::G,
                    egui::Key::H,
                    egui::Key::J,
                ],
                [
                    egui::Key::Z,
                    egui::Key::X,
                    egui::Key::C,
                    egui::Key::V,
                    egui::Key::B,
                    egui::Key::N,
                    egui::Key::M,
                ],
            ];

            let mut current_pads = std::collections::HashSet::<(usize, usize)>::new();
            ctx.input(|inp| {
                for (row, keys) in ROW_KEYS.iter().enumerate() {
                    for (col, &key) in keys.iter().enumerate() {
                        if inp.key_down(key) {
                            current_pads.insert((row, col));
                        }
                    }
                }
            });

            for &pad in &current_pads {
                if !self.chord_kb.kb_held.contains(&pad) {
                    let (row, col) = pad;
                    self.seq_record_chord_pad(row, col);
                    if self.kb_freeze {
                        let frozen: Vec<u8> = self.frozen_notes.drain().collect();
                        for n in frozen {
                            self.push_note_off(n);
                        }
                        for m in
                            apply_voicing(self.chord_kb.chord_notes_for(row, col), self.kb_voicing)
                        {
                            self.push_note_on(m);
                        }
                    } else {
                        for m in
                            apply_voicing(self.chord_kb.chord_notes_for(row, col), self.kb_voicing)
                        {
                            self.push_note_on(m);
                        }
                    }
                }
            }
            let released: Vec<(usize, usize)> = self
                .chord_kb
                .kb_held
                .iter()
                .filter(|p| !current_pads.contains(p))
                .copied()
                .collect();
            for (row, col) in released {
                let notes = apply_voicing(self.chord_kb.chord_notes_for(row, col), self.kb_voicing);
                if self.kb_freeze {
                    for m in notes {
                        self.frozen_notes.insert(m);
                    }
                } else {
                    for m in notes {
                        self.push_note_off(m);
                    }
                }
            }
            self.chord_kb.kb_held = current_pads;
            let prev_midi: Vec<u8> = self.piano_held_midi.drain().collect();
            for m in prev_midi {
                self.push_note_off(m);
            }
        } else {
            // Piano mode — release chord notes if mode just switched
            if !self.chord_kb.kb_held.is_empty() {
                let held: Vec<(usize, usize)> = self.chord_kb.kb_held.drain().collect();
                for (row, col) in held {
                    for m in apply_voicing(self.chord_kb.chord_notes_for(row, col), self.kb_voicing)
                    {
                        self.push_note_off(m);
                    }
                }
            }
            // ChordSeq live transpose: any key press sets the root note
            if SeqMode::from_u8(self.seq.mode.load(std::sync::atomic::Ordering::Relaxed))
                == SeqMode::ChordSeq
                && self.seq.playing.load(std::sync::atomic::Ordering::Relaxed)
            {
                let mut pressed_semitone: Option<u8> = None;
                ctx.input(|inp| {
                    for &(key, semitone) in KEY_MAP {
                        if inp.key_pressed(key) {
                            pressed_semitone = Some((semitone % 12) as u8);
                        }
                    }
                });
                if let Some(semi) = pressed_semitone {
                    self.seq.chord_seq.lock().unwrap().root = semi;
                }
                let prev: Vec<u8> = self.piano_held_midi.drain().collect();
                for m in prev {
                    self.push_note_off(m);
                }
            } else if !ctx.memory(|m| m.focused().is_some()) {
                // Z/X = octave, C/V = velocity, 1/2 = pitch bend, 3–8 = mod wheel → filter
                let prev_pitch_bend = self.piano_pitch_bend;
                let prev_mod = self.piano_mod_wheel;
                ctx.input(|inp| {
                    if inp.key_pressed(egui::Key::Z) && self.piano_octave > 1 {
                        self.piano_octave -= 1;
                    }
                    if inp.key_pressed(egui::Key::X) && self.piano_octave < 7 {
                        self.piano_octave += 1;
                    }
                    if inp.key_pressed(egui::Key::C) {
                        self.piano_velocity = self.piano_velocity.saturating_sub(10).max(10);
                    }
                    if inp.key_pressed(egui::Key::V) {
                        self.piano_velocity = self.piano_velocity.saturating_add(10).min(127);
                    }
                    // 1 = pitch bend down, 2 = pitch bend up (hold), release = reset
                    let bend_down = inp.key_down(egui::Key::Num1);
                    let bend_up = inp.key_down(egui::Key::Num2);
                    self.piano_pitch_bend = if bend_down && !bend_up {
                        -2
                    } else if bend_up && !bend_down {
                        2
                    } else {
                        0
                    };
                    // 3=off, 4=20%, 5=40%, 6=60%, 7=80%, 8=100% filter offset
                    let mod_keys = [
                        (egui::Key::Num3, 0u8),
                        (egui::Key::Num4, 1),
                        (egui::Key::Num5, 2),
                        (egui::Key::Num6, 3),
                        (egui::Key::Num7, 4),
                        (egui::Key::Num8, 5),
                    ];
                    for (key, level) in mod_keys {
                        if inp.key_pressed(key) {
                            self.piano_mod_wheel = level;
                        }
                    }
                });
                if self.piano_pitch_bend != prev_pitch_bend {
                    let semitones = self.piano_pitch_bend as f32;
                    self.engine.set_lfo_pitch_mult(2_f32.powf(semitones / 12.0));
                }
                if self.piano_mod_wheel != prev_mod {
                    self.engine.set_mod_wheel(self.piano_mod_wheel as f32 / 5.0);
                }
                let mut current_held = std::collections::HashSet::<u8>::new();
                ctx.input(|inp| {
                    for &(key, semitone) in KEY_MAP {
                        if inp.key_down(key) {
                            current_held.insert((self.piano_octave * 12 + semitone) as u8);
                        }
                    }
                });
                for &midi in &current_held {
                    if !self.piano_held_midi.contains(&midi) {
                        // New note: release frozen set now, defer NoteOn to next frame.
                        if self.kb_freeze {
                            let frozen: Vec<u8> = self.frozen_notes.drain().collect();
                            for n in frozen {
                                self.push_note_off(n);
                            }
                            self.push_note_on(midi);
                        } else {
                            self.push_note_on(midi);
                        }
                    }
                }
                let released: Vec<u8> = self
                    .piano_held_midi
                    .iter()
                    .filter(|&&m| !current_held.contains(&m))
                    .copied()
                    .collect();
                for midi in released {
                    if self.kb_freeze {
                        self.frozen_notes.insert(midi);
                    } else {
                        self.push_note_off(midi);
                    }
                }
                self.piano_held_midi = current_held;
            } else {
                // A text widget has focus — release any notes that were held before focus was taken.
                let held: Vec<u8> = self.piano_held_midi.drain().collect();
                for midi in held {
                    if self.kb_freeze {
                        self.frozen_notes.insert(midi);
                    } else {
                        self.push_note_off(midi);
                    }
                }
            }
        }
    }

    /// Render the persistent bottom keyboard strip.
    pub fn ui_keyboard_panel(&mut self, ui: &mut egui::Ui) {
        // Pre-resolve tokens.
        let accent     = self.theme.c(&self.theme.accent);
        let accent_fm  = self.theme.c(&self.theme.accent_fm);
        let accent_hold = self.theme.c(&self.theme.accent_hold);
        let text_dis   = self.theme.c(&self.theme.text_disabled);

        ui.horizontal(|ui| {
            // ── Mode selector: Piano / Chord KB ──────────────────────────
            let mut mode = self.kb_chord_mode as usize;
            chip_selector(
                ui, &mut mode,
                &[(0, "Piano"), (1, "Chord KB")],
                &self.theme, None,
            )
            .on_hover_text(
                "Piano — individual notes  |  Chord KB — A–G trigger diatonic chords I–VII.",
            );
            self.kb_chord_mode = mode != 0;

            ui.separator();

            // ── Freeze toggle ─────────────────────────────────────────────
            let prev_freeze = self.kb_freeze;
            toggle_button(
                ui, &mut self.kb_freeze, "❄ Freeze",
                ToggleSize::Standard, Tier::Secondary, &self.theme, Some(accent_fm),
            )
            .on_hover_text(
                "Freeze held notes — they keep sounding until a new chord/note is played.\nSpace bar toggles. MIDI CC 64 (sustain pedal) also works.",
            );
            if prev_freeze && !self.kb_freeze {
                let frozen: Vec<u8> = self.frozen_notes.drain().collect();
                for n in frozen {
                    self.push_note_off(n);
                }
            }

            ui.separator();

            if self.kb_chord_mode {
                // ── Root note ─────────────────────────────────────────────
                ui.label("Key:");
                egui::ComboBox::from_id_salt("chord_kb_root")
                    .selected_text(NOTE_NAMES[self.chord_kb.root as usize])
                    .show_ui(ui, |ui| {
                        for (i, name) in NOTE_NAMES.iter().enumerate() {
                            ui.selectable_value(&mut self.chord_kb.root, i as u8, *name);
                        }
                    });

                // ── Scale selector (Major / Minor) ────────────────────────
                ui.label("Scale:");
                chip_selector(
                    ui, &mut self.chord_kb.scale,
                    &[(ScaleType::Major, "Major"), (ScaleType::Minor, "Minor")],
                    &self.theme, None,
                );

                ui.separator();

                // ── Edit mode toggle ──────────────────────────────────────
                let prev_edit = self.chord_kb.edit_mode;
                toggle_button(
                    ui, &mut self.chord_kb.edit_mode, "✏ Edit",
                    ToggleSize::Standard, Tier::Tertiary, &self.theme, Some(accent_hold),
                )
                .on_hover_text("Edit mode: click any pad to change its chord type.");
                if prev_edit && !self.chord_kb.edit_mode {
                    self.chord_kb.editing_pad = None;
                }

                // ── Piano preview toggle ──────────────────────────────────
                toggle_button(
                    ui, &mut self.chord_kb.show_piano_preview, "KEYS",
                    ToggleSize::Small, Tier::Tertiary, &self.theme, None,
                )
                .on_hover_text("Toggle piano preview");

                ui.separator();

                // Voicing indicator (controlled by arrow keys)
                ui.label(egui::RichText::new("Voicing:").weak().small());
                for &v in VoicingType::all() {
                    let col = if self.kb_voicing == v { accent } else { text_dis };
                    ui.label(egui::RichText::new(v.label()).small().color(col))
                        .on_hover_text(
                            "Hold arrow keys: no arrow = Root  ↑ = 1st  ↓ = 2nd  → = Open  ← = Full",
                        );
                }
                ui.label(egui::RichText::new("  A–J / Q–U / Z–M = rows 1–3").weak().small());
            } else {
                // ── Octave controls ───────────────────────────────────────
                ui.label("Oct:").on_hover_text("Keyboard octave (1–7).  Z = down, X = up");
                if ui
                    .button("−")
                    .on_hover_text("One octave down  [Z]")
                    .clicked()
                    && self.piano_octave > 1
                {
                    self.piano_octave -= 1;
                }
                ui.label(format!("C{}", self.piano_octave));
                if ui
                    .button("+")
                    .on_hover_text("One octave up  [X]")
                    .clicked()
                    && self.piano_octave < 7
                {
                    self.piano_octave += 1;
                }

                ui.separator();

                // ── Velocity controls ─────────────────────────────────────
                ui.label("Vel:").on_hover_text("Note velocity (10–127).  C = down, V = up");
                if ui.button("−").on_hover_text("Velocity −10  [C]").clicked() {
                    self.piano_velocity = self.piano_velocity.saturating_sub(10).max(10);
                }
                ui.label(format!("{}", self.piano_velocity));
                if ui.button("+").on_hover_text("Velocity +10  [V]").clicked() {
                    self.piano_velocity = self.piano_velocity.saturating_add(10).min(127);
                }

                ui.separator();

                // Pitch bend indicator (keys 1/2)
                let bend_col = if self.piano_pitch_bend != 0 {
                    self.theme.c(&self.theme.accent_fm)
                } else {
                    text_dis
                };
                let bend_text = match self.piano_pitch_bend {
                    -2 => "Bend ▼",
                    2  => "Bend ▲",
                    _  => "Bend",
                };
                ui.label(egui::RichText::new(bend_text).color(bend_col))
                    .on_hover_text("Pitch bend ±2 semitones.  Hold 1 = down,  Hold 2 = up");

                ui.separator();

                // Mod wheel indicator (keys 3–8 → filter cutoff offset)
                let mod_col = if self.piano_mod_wheel > 0 {
                    self.theme.c(&self.theme.fx_reverb)
                } else {
                    text_dis
                };
                let mod_bars = ["▁", "▃", "▅", "▆", "█"];
                let mod_label = if self.piano_mod_wheel == 0 {
                    "Mod: off".to_string()
                } else {
                    format!(
                        "Mod: {}",
                        mod_bars[(self.piano_mod_wheel as usize).saturating_sub(1).min(4)]
                    )
                };
                ui.label(egui::RichText::new(mod_label).color(mod_col))
                    .on_hover_text(
                        "Modulation wheel → filter cutoff.  3=off, 4–8=levels  (up to +8 kHz)",
                    );

                ui.separator();

                // ── Scale highlight selector ──────────────────────────────
                ui.label(egui::RichText::new("Scale:").weak().small());
                let mut scale_opts: Vec<(Option<ScaleType>, &str)> =
                    vec![(None, "Off")];
                for &sc in ScaleType::all_highlight() {
                    scale_opts.push((Some(sc), sc.label()));
                }
                chip_selector(
                    ui, &mut self.piano_scale_highlight,
                    &scale_opts, &self.theme, None,
                );

                if self.piano_scale_highlight.is_some() {
                    ui.label(egui::RichText::new("Root:").weak().small());
                    egui::ComboBox::from_id_salt("piano_scale_root")
                        .selected_text(NOTE_NAMES[self.piano_scale_root as usize])
                        .width(48.0)
                        .show_ui(ui, |ui| {
                            for (i, name) in NOTE_NAMES.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.piano_scale_root, i as u8, *name,
                                );
                            }
                        });
                }

                let hint = if SeqMode::from_u8(
                    self.seq.mode.load(std::sync::atomic::Ordering::Relaxed),
                ) == SeqMode::ChordSeq
                    && self.seq.playing.load(std::sync::atomic::Ordering::Relaxed)
                {
                    "  any key = set root note (live transpose)"
                } else {
                    "  a–' = notes  |  z/x = oct  |  c/v = vel  |  1/2 = bend  |  3–8 = mod"
                };
                ui.label(egui::RichText::new(hint).weak().small());
            }
        });

        if self.kb_chord_mode {
            self.draw_chord_pads(ui);
            if self.chord_kb.show_piano_preview {
                self.draw_chord_piano_preview(ui);
            }
        } else {
            self.draw_piano_88(ui);
        }

        let _ = (accent, accent_fm, accent_hold, text_dis);
    }

    fn draw_chord_pads(&mut self, ui: &mut egui::Ui) {
        const KEY_HINTS: [[&str; CHORD_KB_COLS]; CHORD_KB_ROWS] = [
            ["Q", "W", "E", "R", "T", "Y", "U"],
            ["A", "S", "D", "F", "G", "H", "J"],
            ["Z", "X", "C", "V", "B", "N", "M"],
        ];
        const ROW_LABELS: [&str; CHORD_KB_ROWS] = ["7ths", "Triads", "Sus/Add"];

        let sp_xxs = self.theme.sp_xxs;
        let label_w = 48.0_f32;
        let spacing = ui.spacing().item_spacing.x;
        let btn_w = ((ui.available_width() - label_w - spacing * 7.0) / 7.0).max(40.0);
        let btn_h = 52.0;

        // Collect held state snapshot before mutable borrows below.
        let held_pad    = self.chord_kb.held_pad;
        let edit_mode   = self.chord_kb.edit_mode;
        let editing_pad = self.chord_kb.editing_pad;
        let text_disabled = self.theme.c(&self.theme.text_disabled);

        egui::Grid::new("chord_kb_grid")
            .num_columns(CHORD_KB_COLS + 1)
            .spacing([spacing, sp_xxs])
            .show(ui, |ui| {
                for row in 0..CHORD_KB_ROWS {
                    // Fixed-width row label cell
                    ui.allocate_ui_with_layout(
                        Vec2::new(label_w, btn_h),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(ROW_LABELS[row])
                                    .weak()
                                    .small()
                                    .color(text_disabled),
                            );
                        },
                    );

                    for col in 0..CHORD_KB_COLS {
                        let is_held_mouse = held_pad == Some((row, col));
                        let is_held_kb   = self.chord_kb.kb_held.contains(&(row, col));
                        let is_held      = is_held_mouse || is_held_kb;
                        let is_editing   = editing_pad == Some((row, col));

                        let quality = parse_quality(chord_quality(self.chord_kb.scale, col));
                        let cname = chord_name(self.chord_kb.root, self.chord_kb.scale, col);
                        let chord_type = self.chord_kb.pads[row][col].chord_type;
                        let display = if chord_type == ChordType::Triad {
                            cname
                        } else {
                            format!("{} {}", cname, chord_type.label())
                        };

                        let resp = chord_pad(
                            ui,
                            ChordPadState {
                                quality,
                                chord_name: &display,
                                degree: DEGREE_LABELS[col],
                                key_hint: KEY_HINTS[row][col],
                                held: is_held,
                                editing: is_editing,
                            },
                            Vec2::new(btn_w, btn_h),
                            &self.theme,
                        );

                        // Interaction
                        if edit_mode {
                            if resp.clicked() {
                                self.chord_kb.editing_pad =
                                    if is_editing { None } else { Some((row, col)) };
                            }
                        } else {
                            if resp.is_pointer_button_down_on() && !is_held_mouse {
                                self.seq_record_chord_pad(row, col);
                                if self.kb_freeze {
                                    let frozen: Vec<u8> = self.frozen_notes.drain().collect();
                                    for n in frozen {
                                        self.push_note_off(n);
                                    }
                                    if let Some((pr, pc)) = self.chord_kb.held_pad {
                                        for m in apply_voicing(
                                            self.chord_kb.chord_notes_for(pr, pc),
                                            self.kb_voicing,
                                        ) {
                                            self.frozen_notes.insert(m);
                                        }
                                    }
                                    for m in apply_voicing(
                                        self.chord_kb.chord_notes_for(row, col),
                                        self.kb_voicing,
                                    ) {
                                        self.push_note_on(m);
                                    }
                                } else {
                                    if let Some((pr, pc)) = self.chord_kb.held_pad {
                                        for m in apply_voicing(
                                            self.chord_kb.chord_notes_for(pr, pc),
                                            self.kb_voicing,
                                        ) {
                                            self.push_note_off(m);
                                        }
                                    }
                                    for m in apply_voicing(
                                        self.chord_kb.chord_notes_for(row, col),
                                        self.kb_voicing,
                                    ) {
                                        self.push_note_on(m);
                                    }
                                }
                                self.chord_kb.held_pad = Some((row, col));
                            }
                            if !resp.is_pointer_button_down_on() && is_held_mouse {
                                self.chord_kb.held_pad = None;
                                let notes = apply_voicing(
                                    self.chord_kb.chord_notes_for(row, col),
                                    self.kb_voicing,
                                );
                                if self.kb_freeze {
                                    for m in notes {
                                        self.frozen_notes.insert(m);
                                    }
                                } else {
                                    for m in notes {
                                        self.push_note_off(m);
                                    }
                                }
                            }
                        }
                    }
                    ui.end_row();
                }
            }); // end Grid

        // Edit popover
        if let Some((row, col)) = self.chord_kb.editing_pad {
            let accent = self.theme.c(&self.theme.accent);
            let text_dis = self.theme.c(&self.theme.text_disabled);
            egui::Window::new("Chord type")
                .id(egui::Id::new("chord_kb_edit_popover"))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("{} — {}", DEGREE_LABELS[col], ROW_LABELS[row]));
                    ui.separator();
                    for &ct in ChordType::all() {
                        let active = self.chord_kb.pads[row][col].chord_type == ct;
                        let label = egui::RichText::new(ct.label()).color(if active {
                            accent
                        } else {
                            text_dis
                        });
                        if ui.button(label).clicked() {
                            self.chord_kb.pads[row][col].chord_type = ct;
                            self.chord_kb.editing_pad = None;
                            self.chord_kb.edit_mode = false;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.chord_kb.editing_pad = None;
                        self.chord_kb.edit_mode = false;
                    }
                });
        }

        let _ = text_disabled;
    }

    /// Read-only piano strip showing which MIDI notes are currently active in chord KB mode.
    fn draw_chord_piano_preview(&mut self, ui: &mut egui::Ui) {
        // Collect active notes: frozen + currently held pads.
        let mut active: std::collections::HashSet<u8> = self.frozen_notes.clone();
        if let Some((row, col)) = self.chord_kb.held_pad {
            for m in apply_voicing(self.chord_kb.chord_notes_for(row, col), self.kb_voicing) {
                active.insert(m);
            }
        }
        for &(row, col) in &self.chord_kb.kb_held.clone() {
            for m in apply_voicing(self.chord_kb.chord_notes_for(row, col), self.kb_voicing) {
                active.insert(m);
            }
        }

        piano(
            ui,
            &PianoConfig {
                first_midi: PIANO_FIRST_MIDI,
                last_midi:  PIANO_LAST_MIDI,
                size: PianoSize::Preview,
                interactive: false,
                show_labels: true,
                range_bar: None,
            },
            &|midi| KeyVisualState {
                pressed: active.contains(&midi),
                ..Default::default()
            },
            &self.theme,
        );
    }

    /// Draw a full 88-key piano (A0–C8) with active range and scale highlighting.
    fn draw_piano_88(&mut self, ui: &mut egui::Ui) {
        let scale_pcs: Option<[bool; 12]> = self
            .piano_scale_highlight
            .map(|sc| scale_pitch_classes(sc, self.piano_scale_root));

        let kb_max_semitone = KEY_MAP.iter().map(|&(_, s)| s).max().unwrap_or(14);
        let kb_range_start = (self.piano_octave * 12) as u8;
        let kb_range_end   = kb_range_start + kb_max_semitone as u8 + 1;

        let result = piano(
            ui,
            &PianoConfig {
                first_midi: PIANO_FIRST_MIDI,
                last_midi:  PIANO_LAST_MIDI,
                size: PianoSize::Full,
                interactive: true,
                show_labels: true,
                range_bar: Some((kb_range_start, kb_range_end)),
            },
            &|midi| {
                let pressed = self.piano_held_midi.contains(&midi)
                    || self.piano_mouse_midi == Some(midi);
                let pitch_class = (midi % 12) as usize;
                KeyVisualState {
                    pressed,
                    in_kb_range:   midi >= kb_range_start && midi < kb_range_end,
                    is_scale_root: scale_pcs.is_some()
                        && pitch_class == self.piano_scale_root as usize,
                    in_scale: scale_pcs.is_some_and(|pcs| pcs[pitch_class]),
                }
            },
            &self.theme,
        );

        // Mouse interaction — play the note under the pointer.
        if result.response.is_pointer_button_down_on() {
            if let Some(midi) = result.pointer_midi {
                if self.piano_mouse_midi != Some(midi) {
                    if let Some(old) = self.piano_mouse_midi {
                        self.push_note_off(old);
                    }
                    self.piano_mouse_midi = Some(midi);
                    self.push_note_on(midi);
                }
            }
        } else if let Some(midi) = self.piano_mouse_midi.take() {
            self.push_note_off(midi);
        }
    }
}
