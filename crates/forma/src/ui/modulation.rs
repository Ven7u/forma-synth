use crate::ui::design::{
    adsr_display::draw_adsr_visualizer,
    fader::{fader, FaderOrientation, FaderSize},
    filter_display::draw_lp_response_curve,
    toggle::ToggleSize,
    KnobSize, SynthUi, Tier,
};
use crate::ui::frame::SynthFrame;
use crate::SynthApp;
use eframe::egui;
use egui::{RichText, Stroke};

/// Identifies which LFO panel hosts the gate-lane sub-row. Used by
/// `ui_lfo_gate_row` to dispatch reads/writes to the right SynthApp fields.
#[derive(Clone, Copy)]
enum LfoGate {
    L1,
    L2,
}

/// (label, beats_per_cycle) — beats relative to a quarter note.
/// rate_hz = bpm / 60.0 / beats_per_cycle
pub const LFO_SYNC_DIVISIONS: &[(&str, f32)] = &[
    ("4", 16.0), // 4 bars
    ("2", 8.0),  // 2 bars
    ("1", 4.0),  // 1 bar
    ("1/2", 2.0),
    ("1/4", 1.0),
    ("1/8", 0.5),
    ("1/16", 0.25),
    ("1/4T", 2.0 / 3.0), // quarter triplet
    ("1/8T", 1.0 / 3.0), // eighth triplet
];

pub fn lfo_synced_rate(bpm: f32, division: usize) -> f32 {
    let beats = LFO_SYNC_DIVISIONS[division.min(LFO_SYNC_DIVISIONS.len() - 1)].1;
    (bpm / 60.0 / beats).clamp(0.01, 20.0)
}

impl SynthApp {
    pub fn ui_lfo_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();

        SynthFrame::section(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Header — LFO 1 enable toggle.
            let mut lfo_on = self.lfo_enabled;
            if ui
                .synth_toggle(
                    &mut lfo_on,
                    "LFO 1",
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    None,
                )
                .on_hover_text(
                    "Low Frequency Oscillator — modulates pitch, filter cutoff, or amplitude",
                )
                .clicked()
            {
                self.lfo_enabled = lfo_on;
                self.engine.set_lfo_depth(if self.lfo_enabled {
                    self.lfo_depth
                } else {
                    0.0
                });
            }

            ui.add_space(theme.sp_xs);

            ui.add_enabled_ui(self.lfo_enabled, |ui| {
                // Rate + Depth knobs + SYNC toggle.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = theme.sp_md;
                    let sync_on = self.lfo_sync_active();
                    if !sync_on
                        && ui
                            .synth_knob(
                                &mut self.lfo_rate,
                                0.1..=20.0,
                                "RATE",
                                &theme,
                                false,
                                KnobSize::Standard,
                                Tier::Secondary,
                            )
                            .on_hover_text(
                                "LFO speed in Hz. 0.1 = very slow, 5 = fast vibrato, 20 = audio range.",
                            )
                            .changed()
                    {
                        self.engine.set_lfo_rate(self.lfo_rate);
                    }
                    if ui
                        .synth_knob(
                            &mut self.lfo_depth,
                            0.0..=1.0,
                            "DEPTH",
                            &theme,
                            false,
                            KnobSize::Standard,
                            Tier::Secondary,
                        )
                        .on_hover_text("Modulation depth. 0 = off, 1 = full.")
                        .changed()
                    {
                        self.engine.set_lfo_depth(self.lfo_depth);
                    }

                    ui.add_enabled_ui(!self.global_sync, |ui| {
                        let mut sync_on = self.lfo_sync_active();
                        if ui
                            .synth_toggle(
                                &mut sync_on,
                                "SYNC",
                                ToggleSize::Standard,
                                Tier::Secondary,
                                &theme,
                                None,
                            )
                            .on_hover_text("Lock LFO rate to a note division of the Global BPM")
                            .clicked()
                        {
                            self.lfo_sync = !self.lfo_sync;
                            if self.lfo_sync_active() {
                                let rate =
                                    lfo_synced_rate(self.global_bpm as f32, self.lfo_division);
                                self.lfo_rate = rate;
                                self.engine.set_lfo_rate(rate);
                            }
                        }
                    });
                });

                // Division selector when synced — kept as wrapped
                // selectable_label row because chip_selector doesn't
                // currently support `horizontal_wrapped` overflow.
                if self.lfo_sync_active() {
                    ui.add_space(theme.sp_xs);
                    ui.horizontal_wrapped(|ui| {
                        for (i, &(label, _)) in LFO_SYNC_DIVISIONS.iter().enumerate() {
                            let active = self.lfo_division == i;
                            let rate = lfo_synced_rate(self.global_bpm as f32, i);
                            if ui
                                .selectable_label(
                                    active,
                                    RichText::new(label).small().color(if active {
                                        theme.c(&theme.accent)
                                    } else {
                                        theme.c(&theme.text_secondary)
                                    }),
                                )
                                .on_hover_text(format!(
                                    "{} → {:.3} Hz @ {} BPM",
                                    label, rate, self.global_bpm
                                ))
                                .clicked()
                            {
                                self.lfo_division = i;
                                self.lfo_rate = rate;
                                self.engine.set_lfo_rate(rate);
                            }
                        }
                    });
                }

                ui.add_space(theme.sp_xs);

                // SHAPE — chip selector for Sin / Tri / Saw.
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("SHAPE")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    let mut shape = self.lfo_shape;
                    let prev = shape;
                    ui.chip_selector(
                        &mut shape,
                        &[(0usize, "Sin"), (1, "Tri"), (2, "Saw")],
                        &theme,
                        None,
                    )
                    .on_hover_text(
                        "Sine: smooth · Triangle: linear ramp · Saw: ramp + reset",
                    );
                    if shape != prev {
                        self.lfo_shape = shape;
                        self.engine.set_lfo_shape(shape as u8);
                    }
                });

                // Destination — chip selector for Pitch / Filter / Amp.
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("→")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    let mut dest = self.lfo_dest;
                    let prev = dest;
                    ui.chip_selector(
                        &mut dest,
                        &[(0usize, "Pitch"), (1, "Filter"), (2, "Amp")],
                        &theme,
                        None,
                    )
                    .on_hover_text("Pitch: vibrato · Filter: wobble/wah · Amp: tremolo");
                    if dest != prev {
                        self.lfo_dest = dest;
                        self.engine.set_lfo_dest(dest as u8);
                    }
                });

                ui.add_space(theme.sp_xs);
                ui.separator();
                self.ui_lfo_gate_row(ui, LfoGate::L1);
            });
        });
    }

    pub fn ui_lfo2_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();

        SynthFrame::section(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Header — LFO 2 enable toggle.
            let mut lfo2_on = self.lfo2_enabled;
            if ui
                .synth_toggle(
                    &mut lfo2_on,
                    "LFO 2",
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    None,
                )
                .on_hover_text("Second LFO — runs independently of LFO 1")
                .clicked()
            {
                self.lfo2_enabled = lfo2_on;
                self.engine.set_lfo2_depth(if self.lfo2_enabled {
                    self.lfo2_depth
                } else {
                    0.0
                });
            }

            ui.add_space(theme.sp_xs);

            ui.add_enabled_ui(self.lfo2_enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = theme.sp_md;
                    if ui
                        .synth_knob(
                            &mut self.lfo2_rate,
                            0.01..=20.0,
                            "RATE",
                            &theme,
                            true,
                            KnobSize::Standard,
                            Tier::Secondary,
                        )
                        .on_hover_text("LFO 2 rate in Hz — as slow as 0.01 Hz for breathing swells")
                        .changed()
                    {
                        self.engine.set_lfo2_rate(self.lfo2_rate);
                    }
                    if ui
                        .synth_knob(
                            &mut self.lfo2_depth,
                            0.0..=1.0,
                            "DEPTH",
                            &theme,
                            false,
                            KnobSize::Standard,
                            Tier::Secondary,
                        )
                        .on_hover_text("LFO 2 modulation depth")
                        .changed()
                    {
                        self.engine.set_lfo2_depth(self.lfo2_depth);
                    }
                });

                ui.add_space(theme.sp_xs);

                // SHAPE — chip selector.
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("SHAPE")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    let mut shape = self.lfo2_shape;
                    let prev = shape;
                    ui.chip_selector(
                        &mut shape,
                        &[(0usize, "Sin"), (1, "Tri"), (2, "Saw")],
                        &theme,
                        None,
                    );
                    if shape != prev {
                        self.lfo2_shape = shape;
                        self.engine.set_lfo2_shape(shape as u8);
                    }
                });

                // Destination — chip selector.
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("→")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    let mut dest = self.lfo2_dest;
                    let prev = dest;
                    ui.chip_selector(
                        &mut dest,
                        &[(0usize, "Pitch"), (1, "Filter"), (2, "Amp")],
                        &theme,
                        None,
                    );
                    if dest != prev {
                        self.lfo2_dest = dest;
                        self.engine.set_lfo2_dest(dest as u8);
                    }
                });

                ui.add_space(theme.sp_xs);
                ui.separator();
                self.ui_lfo_gate_row(ui, LfoGate::L2);
            });
        });
    }

    /// "PULSE" — gate-lane sequencer that ducks the master output on each "on" step,
    /// tempo-synced to the global BPM. Visual style mirrors `ui_lfo_panel` /
    /// `ui_lfo2_panel`: a `SynthFrame::section` with header, knob row, division row,
    /// and a 16-cell step row below.
    pub fn ui_pulse_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();

        SynthFrame::section(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Header — PULSE enable toggle.
            let mut pulse_on = self.pulse_enabled;
            if ui
                .synth_toggle(
                    &mut pulse_on,
                    "PULSE",
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    None,
                )
                .on_hover_text(
                    "Tempo-synced sidechain ducker — every \"on\" step dips the master output",
                )
                .clicked()
            {
                self.pulse_enabled = pulse_on;
                self.engine.set_gate_aenv_enabled(self.pulse_enabled);
            }

            ui.add_space(theme.sp_xs);

            ui.add_enabled_ui(self.pulse_enabled, |ui| {
                // Depth knob + length stepper.
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = theme.sp_md;
                    if ui
                        .synth_knob(
                            &mut self.pulse_depth,
                            0.0..=1.0,
                            "DEPTH",
                            &theme,
                            false,
                            KnobSize::Standard,
                            Tier::Secondary,
                        )
                        .on_hover_text("How hard each step ducks the master output")
                        .changed()
                    {
                        self.engine.set_gate_aenv_depth(self.pulse_depth);
                    }

                    ui.label(
                        RichText::new("LEN")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    let mut len = self.pulse_length as i32;
                    if ui
                        .add(egui::DragValue::new(&mut len).range(1..=16))
                        .on_hover_text("Number of active steps before the pattern repeats")
                        .changed()
                    {
                        self.pulse_length = len.clamp(1, 16) as u8;
                        self.engine.set_gate_aenv_length(self.pulse_length);
                    }
                });

                ui.add_space(theme.sp_xs);

                // Division selector — kept as wrapped selectable_label row.
                ui.horizontal_wrapped(|ui| {
                    for div in forma_common::ClockDivision::ALL {
                        let div_u8 = div.to_u8();
                        let active = self.pulse_division as u8 == div_u8;
                        let label = div.label();
                        let rate = div.hz(self.global_bpm as f32);
                        if ui
                            .selectable_label(
                                active,
                                RichText::new(label).small().color(if active {
                                    theme.c(&theme.accent)
                                } else {
                                    theme.c(&theme.text_secondary)
                                }),
                            )
                            .on_hover_text(format!(
                                "{} → {:.3} Hz @ {} BPM",
                                label, rate, self.global_bpm
                            ))
                            .clicked()
                        {
                            self.pulse_division = div_u8 as usize;
                            self.engine.set_gate_aenv_division(div_u8);
                            self.engine.set_gate_aenv_rate(rate);
                        }
                    }
                });

                ui.add_space(theme.sp_xs);

                // 16-cell step row. Same length-aware alpha treatment as
                // `ui_lfo_gate_row` — see that comment.
                ui.label(
                    RichText::new("STEPS")
                        .font(theme.font_body())
                        .color(theme.c(&theme.text_secondary)),
                );
                ui.horizontal(|ui| {
                    let total_w = ui.available_width();
                    let spacing = ui.spacing().item_spacing.x;
                    let step_w = ((total_w - spacing * 15.0) / 16.0).max(14.0);
                    let cell_h = 26.0;
                    let active_col = theme.c(&theme.accent);
                    let inactive_col = theme.c(&theme.bg_sunken);
                    let edge = theme.c(&theme.text_disabled);
                    for i in 0..16u8 {
                        let on_step = (self.pulse_pattern >> i) & 1 != 0;
                        let in_active_len = i < self.pulse_length;
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::Vec2::new(step_w, cell_h),
                            egui::Sense::click(),
                        );
                        let painter = ui.painter_at(rect);
                        let fill = if on_step { active_col } else { inactive_col };
                        // Token-derived: alpha encodes pattern-length state.
                        let alpha = if in_active_len { 255 } else { 90 };
                        let fill = egui::Color32::from_rgba_unmultiplied(
                            fill.r(),
                            fill.g(),
                            fill.b(),
                            alpha,
                        );
                        painter.rect_filled(rect, egui::CornerRadius::same(theme.rounding_xs as u8), fill);
                        painter.rect_stroke(
                            rect,
                            egui::CornerRadius::same(theme.rounding_xs as u8),
                            Stroke::new(theme.stroke_ui, edge),
                            egui::StrokeKind::Middle,
                        );
                        if resp.clicked() {
                            self.pulse_pattern ^= 1u16 << i;
                            self.engine.set_gate_aenv_pattern(self.pulse_pattern);
                        }
                    }
                });
            });
        });
    }

    /// Compact RETRIG sub-row appended inside `ui_lfo_panel` / `ui_lfo2_panel`.
    /// Renders a small enable toggle, a division combobox, and a 16-cell step row.
    /// Routes reads/writes through `which` so the same code drives both LFOs.
    fn ui_lfo_gate_row(&mut self, ui: &mut egui::Ui, which: LfoGate) {
        // Snapshot current state for the requested lane (avoids &mut self field aliasing).
        let mut enabled = match which {
            LfoGate::L1 => self.lfo1_gate_enabled,
            LfoGate::L2 => self.lfo2_gate_enabled,
        };
        let mut pattern = match which {
            LfoGate::L1 => self.lfo1_gate_pattern,
            LfoGate::L2 => self.lfo2_gate_pattern,
        };
        let length = match which {
            LfoGate::L1 => self.lfo1_gate_length,
            LfoGate::L2 => self.lfo2_gate_length,
        };
        let mut division = match which {
            LfoGate::L1 => self.lfo1_gate_division,
            LfoGate::L2 => self.lfo2_gate_division,
        };

        let theme = self.theme.clone();

        // Header row: RETRIG toggle + division dropdown.
        ui.horizontal(|ui| {
            if ui
                .synth_toggle(
                    &mut enabled,
                    "RETRIG",
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    None,
                )
                .on_hover_text(
                    "Resets the LFO's phase to 0 on each \"on\" step (tempo-synced).\n\
                     \n\
                     The LFO keeps running at its own RATE — retrigger only restarts the cycle.\n\
                     Set the LFO rate SLOWER than the retrigger rate so each gated step plays\n\
                     just the start of the LFO shape, like a tempo-locked envelope:\n\
                       • LFO 0.3–1 Hz + RETRIG 1/4 → rhythmic filter/amp sweeps.\n\
                       • LFO at the same rate as RETRIG → no audible effect (already aligned).\n\
                       • LFO faster than RETRIG → barely audible (only tiny phase glitches).",
                )
                .clicked()
            {
                // synth_toggle has already flipped `enabled` via its &mut.
            }

            let div = forma_common::ClockDivision::from_u8(division as u8);
            egui::ComboBox::from_id_salt(("lfo_gate_div", which as u8))
                .selected_text(div.label())
                .show_ui(ui, |ui| {
                    for d in forma_common::ClockDivision::ALL {
                        if ui
                            .selectable_label(div == *d, d.label())
                            .on_hover_text(format!(
                                "{} → {:.2} Hz @ {} BPM",
                                d.label(),
                                d.hz(self.global_bpm as f32),
                                self.global_bpm
                            ))
                            .clicked()
                        {
                            division = d.to_u8() as usize;
                        }
                    }
                });
        });

        // 16-cell step row. Custom painter rather than StepPad component
        // because the active-length alpha treatment (steps beyond the
        // pattern length render at 35% opacity to signal "ignored") doesn't
        // map cleanly to StepPad's binary state model. All visual values
        // are token-derived.
        let mut pattern_changed = false;
        ui.add_enabled_ui(enabled, |ui| {
            ui.horizontal(|ui| {
                let total_w = ui.available_width();
                let spacing = ui.spacing().item_spacing.x;
                let step_w = ((total_w - spacing * 15.0) / 16.0).max(10.0);
                let cell_h = 18.0;
                let active_col = theme.c(&theme.accent);
                let inactive_col = theme.c(&theme.bg_sunken);
                let edge = theme.c(&theme.text_disabled);
                for i in 0..16u8 {
                    let on_step = (pattern >> i) & 1 != 0;
                    let in_active_len = i < length;
                    let (rect, resp) = ui
                        .allocate_exact_size(egui::Vec2::new(step_w, cell_h), egui::Sense::click());
                    let painter = ui.painter_at(rect);
                    let fill = if on_step { active_col } else { inactive_col };
                    // Token-derived: alpha encodes whether the step falls
                    // inside the current pattern length.
                    let alpha = if in_active_len { 255 } else { 90 };
                    let fill =
                        egui::Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), alpha);
                    painter.rect_filled(rect, egui::CornerRadius::same(theme.rounding_xs as u8), fill);
                    painter.rect_stroke(
                        rect,
                        egui::CornerRadius::same(theme.rounding_xs as u8),
                        Stroke::new(theme.stroke_ui, edge),
                        egui::StrokeKind::Middle,
                    );
                    if resp.clicked() {
                        pattern ^= 1u16 << i;
                        pattern_changed = true;
                    }
                }
            });
        });

        // Push changes back to SynthApp + engine.
        match which {
            LfoGate::L1 => {
                if self.lfo1_gate_enabled != enabled {
                    self.lfo1_gate_enabled = enabled;
                    self.engine.set_gate_lfo1_enabled(enabled);
                }
                if self.lfo1_gate_division != division {
                    self.lfo1_gate_division = division;
                    self.engine.set_gate_lfo1_division(division as u8);
                    let rate = forma_common::ClockDivision::from_u8(division as u8)
                        .hz(self.global_bpm as f32);
                    self.engine.set_gate_lfo1_rate(rate);
                }
                if pattern_changed {
                    self.lfo1_gate_pattern = pattern;
                    self.engine.set_gate_lfo1_pattern(pattern);
                }
            }
            LfoGate::L2 => {
                if self.lfo2_gate_enabled != enabled {
                    self.lfo2_gate_enabled = enabled;
                    self.engine.set_gate_lfo2_enabled(enabled);
                }
                if self.lfo2_gate_division != division {
                    self.lfo2_gate_division = division;
                    self.engine.set_gate_lfo2_division(division as u8);
                    let rate = forma_common::ClockDivision::from_u8(division as u8)
                        .hz(self.global_bpm as f32);
                    self.engine.set_gate_lfo2_rate(rate);
                }
                if pattern_changed {
                    self.lfo2_gate_pattern = pattern;
                    self.engine.set_gate_lfo2_pattern(pattern);
                }
            }
        }
    }

    pub fn ui_filter_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();

        // Tier 1 frame — Filter cutoff / resonance are the canonical
        // performance controls per `01-philosophy.md`. Accent border earns
        // the elevation.
        SynthFrame::tier1(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // ── Header row: FILTER toggle | mode chips | LP24 badge ───────
            let mut filter_on = self.filter_enabled;
            ui.horizontal(|ui| {
                if ui
                    .synth_toggle(
                        &mut filter_on,
                        "FILTER",
                        ToggleSize::Large,
                        Tier::Primary,
                        &theme,
                        None,
                    )
                    .on_hover_text("Moog-style 4-pole lowpass filter")
                    .clicked()
                {
                    self.filter_enabled = filter_on;
                    if self.filter_enabled {
                        self.engine.set_filter_cutoff(self.filter_cutoff);
                        self.engine.set_filter_resonance(self.filter_q);
                    } else {
                        self.engine.set_filter_cutoff(18000.0);
                        self.engine.set_filter_resonance(0.0);
                    }
                }
                ui.add_space(theme.sp_sm);
                // Mode chips — LP active, others disabled until multi-mode lands
                let accent = theme.c(&theme.accent);
                let disabled = theme.c(&theme.text_disabled);
                ui.add(egui::Button::selectable(
                    true,
                    RichText::new("LP").font(theme.font_body()).strong().color(accent),
                ));
                for label in ["BP", "HP", "NOTCH"] {
                    ui.add_enabled(
                        false,
                        egui::Button::selectable(
                            false,
                            RichText::new(label).font(theme.font_body()).color(disabled),
                        ),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("LP24")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                });
            });

            ui.add_space(theme.sp_xs);

            ui.add_enabled_ui(self.filter_enabled, |ui| {
                let mod_offset = if self.mod_wheel_dest == 1 {
                    self.piano_mod_wheel as f32 / 5.0 * self.mod_wheel_depth * self.filter_cutoff * 0.5
                } else {
                    0.0
                };
                let effective_cutoff = (self.filter_cutoff + mod_offset).clamp(80.0, 18000.0);

                // ── Curve (left) + Knob column (right) side by side ───────
                // Knob column: 2 Large knobs (row 1) + 4 Standard knobs (row 2)
                // Large rect = 64px, Standard = 44px; give the column 240px.
                let knob_col_w = 240.0_f32;
                let curve_w = (ui.available_width() - knob_col_w - theme.sp_md).max(160.0);

                ui.horizontal(|ui| {
                    // ── Filter response curve ─────────────────────────────
                    let curve_size = egui::Vec2::new(curve_w, 160.0);
                    let (rect, response) =
                        ui.allocate_exact_size(curve_size, egui::Sense::click_and_drag());

                    if response.dragged() {
                        let fine = ui.input(|i| i.modifiers.shift);
                        if fine {
                            let delta = response.drag_delta();
                            let log_min = 80.0_f32.ln();
                            let log_max = 18000.0_f32.ln();
                            self.filter_cutoff = ((self.filter_cutoff.ln()
                                + delta.x / rect.width() * (log_max - log_min) * 0.15)
                                .clamp(log_min, log_max))
                            .exp();
                            self.filter_q = (self.filter_q - delta.y / rect.height() * 0.95 * 0.15)
                                .clamp(0.0, 0.95);
                        } else if let Some(pos) = response.interact_pointer_pos() {
                            let x = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                            let y = ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                            let log_min = 80.0_f32.ln();
                            let log_max = 18000.0_f32.ln();
                            self.filter_cutoff = (log_min + x * (log_max - log_min)).exp();
                            self.filter_q = (1.0 - y) * 0.95;
                        }
                        self.engine.set_filter_cutoff(self.filter_cutoff);
                        self.engine.set_filter_resonance(self.filter_q);
                    }
                    if response.double_clicked() {
                        self.filter_cutoff = 3000.0;
                        self.filter_q = 0.0;
                        self.engine.set_filter_cutoff(self.filter_cutoff);
                        self.engine.set_filter_resonance(self.filter_q);
                    }

                    if ui.is_rect_visible(rect) {
                        draw_lp_response_curve(
                            ui.painter(),
                            rect,
                            effective_cutoff,
                            self.filter_q,
                            response.hovered() || response.dragged(),
                            &theme,
                        );
                    }

                    ui.add_space(theme.sp_md);

                    // ── Knob column ───────────────────────────────────────
                    ui.vertical(|ui| {
                        // Row 1: Tier 1 — CUTOFF + RES (Large)
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = theme.sp_md;

                            ui.vertical(|ui| {
                                let mut display_cutoff = effective_cutoff;
                                let knob_resp = ui
                                    .synth_knob(
                                        &mut display_cutoff,
                                        80.0..=18000.0,
                                        "CUTOFF",
                                        &theme,
                                        true,
                                        KnobSize::Large,
                                        Tier::Primary,
                                    )
                                    .on_hover_text("Cutoff frequency. 80 Hz = dark, 18 kHz = fully open.");
                                if knob_resp.changed() {
                                    self.filter_cutoff =
                                        (display_cutoff - mod_offset).clamp(80.0, 18000.0);
                                    self.engine.set_filter_cutoff(self.filter_cutoff);
                                }
                                let hz_str = if effective_cutoff >= 1000.0 {
                                    format!("{:.1}k", effective_cutoff / 1000.0)
                                } else {
                                    format!("{:.0}", effective_cutoff)
                                };
                                let (prefix, color) = if mod_offset > 0.0 {
                                    ("▲ ", theme.c(&theme.accent))
                                } else {
                                    ("", theme.c(&theme.text_secondary))
                                };
                                ui.label(
                                    RichText::new(format!("{prefix}{hz_str}"))
                                        .font(theme.font_small())
                                        .color(color),
                                )
                                .on_hover_text("Effective cutoff frequency");
                            });

                            if ui
                                .synth_knob(
                                    &mut self.filter_q,
                                    0.0..=0.95,
                                    "RES",
                                    &theme,
                                    false,
                                    KnobSize::Large,
                                    Tier::Primary,
                                )
                                .on_hover_text("Resonance — 0 = flat, 0.9+ = self-oscillation.")
                                .changed()
                            {
                                self.engine.set_filter_resonance(self.filter_q);
                            }
                        });

                        ui.add_space(theme.sp_xs);

                        // Row 2: Tier 2 — DRIVE, KEY, VEL>AMP, VEL>FLT (Standard)
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = theme.sp_sm;

                            let mut drive = self.engine.filter_drive();
                            if ui
                                .synth_knob(&mut drive, 1.0..=10.0, "DRIVE", &theme, false, KnobSize::Standard, Tier::Secondary)
                                .on_hover_text("Input drive — saturates before the filter. 1 = clean, 10 = heavy.")
                                .changed()
                            {
                                self.engine.set_filter_drive(drive);
                            }

                            let mut key_track = self.engine.filter_key_track();
                            if ui
                                .synth_knob(&mut key_track, 0.0..=1.0, "KEY", &theme, false, KnobSize::Standard, Tier::Secondary)
                                .on_hover_text("Keyboard tracking — cutoff follows pitch. 0 = off, 1 = full.")
                                .changed()
                            {
                                self.engine.set_filter_key_track(key_track);
                            }

                            let mut vel_amp = self.engine.vel_amp();
                            if ui
                                .synth_knob(&mut vel_amp, 0.0..=1.0, "VEL>AMP", &theme, false, KnobSize::Standard, Tier::Secondary)
                                .on_hover_text("Velocity → amplitude. 0 = ignored, 1 = full sensitivity.")
                                .changed()
                            {
                                self.engine.set_vel_amp(vel_amp);
                            }

                            let mut vel_filter = self.engine.vel_filter();
                            if ui
                                .synth_knob(&mut vel_filter, 0.0..=1.0, "VEL>FLT", &theme, false, KnobSize::Standard, Tier::Secondary)
                                .on_hover_text("Velocity → filter cutoff. 0 = off, 1 = adds up to 8 kHz.")
                                .changed()
                            {
                                self.engine.set_vel_filter(vel_filter);
                            }
                        });
                    });
                });
            });
        });
    }

    pub fn ui_mod_matrix_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        const SOURCES: &[&str] = &["Off", "LFO 1", "LFO 2", "Mod Wheel", "Aftertouch"];
        const DESTS: &[&str] = &["Off", "Filter", "Amp", "Pitch"];

        SynthFrame::section(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("MOD MATRIX")
                    .font(theme.font_heading())
                    .color(theme.c(&theme.text_primary)),
            );
            ui.add_space(theme.sp_xs);

            egui::Grid::new("mod_matrix_grid")
                .num_columns(4)
                .spacing([theme.sp_sm, theme.sp_xs])
                .show(ui, |ui| {
                    for label in ["#", "SOURCE", "→ DEST", "DEPTH"] {
                        ui.label(
                            RichText::new(label)
                                .font(theme.font_body())
                                .color(theme.c(&theme.text_secondary)),
                        );
                    }
                    ui.end_row();

                    for slot in 0..4 {
                        ui.label(
                            RichText::new(format!("{}", slot + 1))
                                .font(theme.font_body())
                                .color(theme.c(&theme.text_disabled)),
                        );

                        egui::ComboBox::from_id_salt(format!("mat_src_{slot}"))
                            .selected_text(SOURCES[self.mat_src[slot].min(4)])
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for (i, label) in SOURCES.iter().enumerate() {
                                    if ui
                                        .selectable_label(self.mat_src[slot] == i, *label)
                                        .clicked()
                                    {
                                        self.mat_src[slot] = i;
                                        self.engine.set_mat_src(slot, i as u8);
                                    }
                                }
                            });

                        egui::ComboBox::from_id_salt(format!("mat_dst_{slot}"))
                            .selected_text(DESTS[self.mat_dst[slot].min(3)])
                            .width(70.0)
                            .show_ui(ui, |ui| {
                                for (i, label) in DESTS.iter().enumerate() {
                                    if ui
                                        .selectable_label(self.mat_dst[slot] == i, *label)
                                        .clicked()
                                    {
                                        self.mat_dst[slot] = i;
                                        self.engine.set_mat_dst(slot, i as u8);
                                    }
                                }
                            });

                        let mut d = self.mat_depth[slot];
                        let active = self.mat_src[slot] != 0 && self.mat_dst[slot] != 0;
                        let resp = ui.add_enabled(
                            active,
                            egui::DragValue::new(&mut d)
                                .range(-1.0_f32..=1.0)
                                .speed(0.01)
                                .fixed_decimals(2),
                        );
                        if resp.changed() {
                            self.mat_depth[slot] = d;
                            self.engine.set_mat_depth(slot, d);
                        }

                        ui.end_row();
                    }
                });
        });
    }

    pub fn ui_mod_wheel_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        SynthFrame::section(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("MOD WHEEL")
                    .font(theme.font_heading())
                    .color(theme.c(&theme.text_primary)),
            );
            ui.add_space(theme.sp_xs);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme.sp_md;
                let mut depth = self.mod_wheel_depth;
                if ui
                    .synth_knob(
                        &mut depth,
                        0.0..=1.0,
                        "DEPTH",
                        &theme,
                        false,
                        KnobSize::Standard,
                        Tier::Secondary,
                    )
                    .on_hover_text("How much the mod wheel affects the selected destination.")
                    .changed()
                {
                    self.mod_wheel_depth = depth;
                    self.engine.set_mod_wheel_depth(depth);
                }
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("→")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    for (d, label, tip) in [
                        (0usize, "Off", "Mod wheel has no effect."),
                        (1, "Filter", "Opens the filter as you push the wheel."),
                        (
                            2,
                            "LFO Depth",
                            "Scales LFO 1 depth — classic vibrato/wah control.",
                        ),
                        (3, "Amp", "Reduces amplitude — expression/swell."),
                    ] {
                        if ui
                            .selectable_label(self.mod_wheel_dest == d, label)
                            .on_hover_text(tip)
                            .clicked()
                        {
                            self.mod_wheel_dest = d;
                            self.engine.set_mod_wheel_dest(d as u8);
                        }
                    }
                });
            });
        });
    }

    pub fn ui_aftertouch_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        SynthFrame::section(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("AFTERTOUCH")
                    .font(theme.font_heading())
                    .color(theme.c(&theme.text_primary)),
            );
            ui.add_space(theme.sp_xs);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme.sp_md;
                let mut depth = self.aftertouch_depth;
                if ui
                    .synth_knob(
                        &mut depth,
                        0.0..=1.0,
                        "DEPTH",
                        &theme,
                        false,
                        KnobSize::Standard,
                        Tier::Secondary,
                    )
                    .on_hover_text(
                        "How much channel pressure (aftertouch) affects the selected destination.",
                    )
                    .changed()
                {
                    self.aftertouch_depth = depth;
                    self.engine.set_aftertouch_depth(depth);
                }
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("→")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    for (d, label, tip) in [
                        (0usize, "Off", "Aftertouch has no effect."),
                        (1, "Filter", "Pressing harder opens the filter."),
                        (2, "LFO Depth", "Pressing harder increases LFO 1 depth."),
                        (3, "Amp", "Pressing harder reduces volume — swell effect."),
                    ] {
                        if ui
                            .selectable_label(self.aftertouch_dest == d, label)
                            .on_hover_text(tip)
                            .clicked()
                        {
                            self.aftertouch_dest = d;
                            self.engine.set_aftertouch_dest(d as u8);
                        }
                    }
                });
            });
        });
    }

    pub fn ui_adsr_panel(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        _slots: &mut [usize; 4],
        is_filter: bool,
    ) {
        let theme = self.theme.clone();
        let sp_xs = theme.sp_xs;
        let sp_sm = theme.sp_sm;

        SynthFrame::section(&theme).show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // Snapshot ADSR from engine
            let mut adsr: [f32; 4] = if is_filter {
                [
                    self.engine.fenv_attack(),
                    self.engine.fenv_decay(),
                    self.engine.fenv_sustain(),
                    self.engine.fenv_release(),
                ]
            } else {
                [
                    self.engine.amp_attack(),
                    self.engine.amp_decay(),
                    self.engine.amp_sustain(),
                    self.engine.amp_release(),
                ]
            };
            let labels = ["A", "D", "S", "R"];
            let tips = [
                "Attack — time to reach full level after a note is pressed.",
                "Decay — time to fall from peak to sustain level.",
                "Sustain — level held while key is held (0 = silent, 1 = full).",
                "Release — time to fade out after key is released.",
            ];
            let ranges: [std::ops::RangeInclusive<f32>; 4] =
                [0.001..=10.0, 0.001..=5.0, 0.0..=1.0, 0.001..=15.0];

            let cursors: Vec<f32> = if is_filter {
                self.engine.fenv_cursors()
            } else {
                self.engine.amp_cursors()
            };

            // ── Header inline with fader row ──────────────────────────────
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .font(theme.font_heading())
                        .strong()
                        .color(theme.c(&theme.text_primary)),
                );
            });
            ui.add_space(sp_xs);

            // ── Faders (Large) on left, envelope plot on right ────────────
            let fader_h = FaderSize::Large.length();
            let val_label_h = theme.font_small().size + 2.0;
            let stage_label_h = theme.font_body().size + sp_xs;
            let total_h = stage_label_h + fader_h + val_label_h;

            ui.horizontal(|ui| {
                for i in 0..4 {
                    ui.vertical(|ui| {
                        // Stage label
                        ui.label(
                            RichText::new(labels[i])
                                .font(theme.font_body())
                                .color(theme.c(&theme.text_secondary)),
                        );

                        // A/D/R: log scale → normalize to 0..1 for fader
                        let log = i != 2;
                        let (lo, hi) = (*ranges[i].start(), *ranges[i].end());
                        let mut norm = if log {
                            (adsr[i].max(lo).ln() - lo.ln()) / (hi.ln() - lo.ln())
                        } else {
                            (adsr[i] - lo) / (hi - lo)
                        };
                        norm = norm.clamp(0.0, 1.0);
                        let resp = fader(
                            ui,
                            &mut norm,
                            0.0..=1.0,
                            FaderOrientation::Vertical,
                            FaderSize::Large,
                            &theme,
                        )
                        .on_hover_text(tips[i]);
                        if resp.changed() {
                            let v = if log {
                                (lo.ln() + norm * (hi.ln() - lo.ln())).exp()
                            } else {
                                lo + norm * (hi - lo)
                            }
                            .clamp(lo, hi);
                            adsr[i] = v;
                            if is_filter {
                                match i {
                                    0 => self.engine.set_fenv_attack(v),
                                    1 => self.engine.set_fenv_decay(v),
                                    2 => self.engine.set_fenv_sustain(v),
                                    _ => self.engine.set_fenv_release(v),
                                }
                            } else {
                                match i {
                                    0 => self.engine.set_amp_attack(v),
                                    1 => self.engine.set_amp_decay(v),
                                    2 => self.engine.set_amp_sustain(v),
                                    _ => self.engine.set_amp_release(v),
                                }
                            }
                        }

                        // Value readout below fader
                        let val_str = if i == 2 {
                            format!("{:.2}", adsr[i])
                        } else if adsr[i] < 1.0 {
                            format!("{:.0}ms", adsr[i] * 1000.0)
                        } else {
                            format!("{:.1}s", adsr[i])
                        };
                        ui.label(
                            RichText::new(val_str)
                                .font(theme.font_small())
                                .color(theme.c(&theme.text_secondary)),
                        );
                    });
                }

                ui.add_space(sp_sm);
                // Envelope plot fills remaining width at full fader column height
                draw_adsr_visualizer(ui, &adsr, &cursors, &theme, total_h);
            });
        });
    }
}
