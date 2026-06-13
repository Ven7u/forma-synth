use crate::ui::design::{toggle::ToggleSize, KnobSize, SynthUi, Tier};
use crate::ui::frame::SynthFrame;
use crate::SynthApp;
use eframe::egui;
use egui::{Color32, CornerRadius, Pos2, Rect, RichText, Sense, Stroke, Vec2};

const WAVE_OPTIONS: &[(usize, &str)] =
    &[(0, "Sin"), (1, "Saw"), (2, "Sqr"), (3, "Tri")];

/// Shared fixed height for all dock cards (inner content, before section margins).
/// Raise or lower this to resize all four cards together.
const CARD_H: f32 = 260.0;

impl SynthApp {
    pub fn ui_osc_panel(&mut self, ui: &mut egui::Ui, i: usize) {
        let theme = self.theme.clone();
        let is_osc1 = i == 0;
        let flip = is_osc1 && self.osc1_mod_view;

        // Tier 1 frame: this is a major sound-shaping zone even though the
        // controls themselves are Tier 2/3.
        let frame = SynthFrame::tier1(&theme);

        // Track header interactions inside the closure; apply after the frame
        // returns to avoid borrow-checker contortions.
        let mut new_enabled = self.osc_enabled[i];
        let mut flip_clicked = false;

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            // ── Header ────────────────────────────────────────────────────
            let title = if flip {
                format!("OSC {} · MOD", i + 1)
            } else {
                format!("OSC {}", i + 1)
            };
            ui.horizontal(|ui| {
                ui.synth_toggle(
                    &mut new_enabled,
                    &title,
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    None,
                )
                .on_hover_text("Toggle oscillator on/off");

                if is_osc1 {
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let flip_label = if flip { "‹ back" } else { "mod ›" };
                            let flip_col = theme.c(&theme.text_secondary);
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(flip_label)
                                            .font(theme.font_body())
                                            .color(flip_col),
                                    )
                                    .sense(Sense::click()),
                                )
                                .on_hover_text(if flip {
                                    "Back to main controls"
                                } else {
                                    "Sync / FM / Ring mod"
                                })
                                .clicked()
                            {
                                flip_clicked = true;
                            }
                        },
                    );
                }
            });

            ui.add_space(theme.sp_xs);

            if flip {
                self.ui_osc1_mod_back(ui);
            } else {
                self.ui_osc_front(ui, i);
            }
            // Pad to shared fixed card height so all dock cards are the same size.
            ui.add_space((CARD_H - ui.min_rect().height()).max(0.0));
        });

        if new_enabled != self.osc_enabled[i] {
            self.osc_enabled[i] = new_enabled;
            let vol = if new_enabled { self.osc_vol[i] } else { 0.0 };
            self.engine.set_osc_vol(i as u8, vol);
        }
        if flip_clicked {
            self.osc1_mod_view = !self.osc1_mod_view;
        }
    }

    // ── Front face (identical for all 3 OSCs) ────────────────────────────────

    fn ui_osc_front(&mut self, ui: &mut egui::Ui, i: usize) {
        let theme = self.theme.clone();
        let on = self.osc_enabled[i];

        ui.add_enabled_ui(on, |ui| {
            // ── Waveform chips (Tier 3 — configuration) ────────────────────
            let mut wave_choice = self.osc_wave[i];
            let chip_resp = ui.chip_selector(
                &mut wave_choice,
                WAVE_OPTIONS,
                &theme,
                Some(ui.available_width()),
            );
            chip_resp.on_hover_text("Waveform: sine, sawtooth, square (PW), triangle");
            if wave_choice != self.osc_wave[i] {
                self.osc_wave[i] = wave_choice;
                self.engine.set_osc_wave(i as u8, wave_choice as u8);
            }

            ui.add_space(theme.sp_sm);

            // ── Knob row: OCT · DET · PW (Tier 2/3 — sound design + config) ─
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme.sp_md;

                // OCT — integer DragValue. Tier 3 (rarely touched mid-perf).
                ui.vertical(|ui| {
                    let col_w = KnobSize::Standard.rect().x;
                    ui.set_width(col_w);
                    ui.add_space(theme.sp_xs);
                    if ui
                        .add_sized(
                            [col_w, 28.0],
                            egui::DragValue::new(&mut self.osc_octave[i])
                                .range(-2..=2)
                                .prefix("Oct "),
                        )
                        .on_hover_text("Octave shift (−2 … +2)")
                        .changed()
                    {
                        self.update_freq_mult(i);
                    }
                    ui.add_space(theme.sp_xxs);
                    ui.label(
                        RichText::new("OCT")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                });

                // DET knob (±100 ¢) — Tier 2.
                if ui
                    .synth_knob(
                        &mut self.osc_detune[i],
                        -100.0..=100.0,
                        "DET",
                        &theme,
                        false,
                        KnobSize::Standard,
                        Tier::Secondary,
                    )
                    .on_hover_text("Detune ±100 ¢. Shift+drag for fine control.")
                    .changed()
                {
                    self.update_freq_mult(i);
                }

                // PW knob — Tier 2, only active for square waveform.
                let pw_enabled = self.osc_wave[i] == 2;
                ui.add_enabled_ui(pw_enabled, |ui| {
                    if ui
                        .synth_knob(
                            &mut self.osc_pulse_width[i],
                            0.01..=0.99,
                            "PW",
                            &theme,
                            false,
                            KnobSize::Standard,
                            Tier::Secondary,
                        )
                        .on_hover_text("Pulse Width — duty cycle of the square wave.\n0.5 = symmetric square (hollow/woody).\nLower or higher = thin, nasal tone.\nModulate with LFO for classic PWM sweep.\nOnly active on Sqr waveform.")
                        .changed()
                    {
                        self.engine.set_osc_pulse_width(i as u8, self.osc_pulse_width[i]);
                    }
                });
            });

            ui.add_space(theme.sp_xs);

            // ── Unison row (Tier 2) ───────────────────────────────────────
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = theme.sp_xs;
                let mut uni_on = self.osc_unison_enabled[i];
                if ui
                    .synth_toggle(
                        &mut uni_on,
                        "UNI",
                        ToggleSize::Standard,
                        Tier::Secondary,
                        &theme,
                        None,
                    )
                    .on_hover_text("Stack detuned voices for a thick, wide sound")
                    .clicked()
                {
                    self.osc_unison_enabled[i] = uni_on;
                    self.update_unison(i);
                }

                if uni_on {
                    let mut changed = false;
                    changed |= ui
                        .add_sized(
                            [40.0, 24.0],
                            egui::DragValue::new(&mut self.osc_unison_count[i])
                                .range(2..=5)
                                .prefix("×"),
                        )
                        .on_hover_text("Number of unison voices (2–5)")
                        .changed();
                    changed |= ui
                        .synth_knob(
                            &mut self.osc_unison_spread[i],
                            0.0..=50.0,
                            "SPRD",
                            &theme,
                            false,
                            KnobSize::Standard,
                            Tier::Secondary,
                        )
                        .on_hover_text("Total pitch spread across unison voices (cents)")
                        .changed();
                    if changed {
                        self.update_unison(i);
                    }
                }
            });

            ui.add_space(theme.sp_sm);

            // ── Mini waveform preview ─────────────────────────────────────
            let notes_held = !self.piano_held_midi.is_empty()
                || self.seq.playing.load(std::sync::atomic::Ordering::Relaxed);
            let active = on && notes_held;

            let preview_h = 36.0_f32;
            let (rect, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), preview_h),
                Sense::hover(),
            );
            if ui.is_rect_visible(rect) {
                let line_color = if active {
                    theme.c(&theme.accent)
                } else {
                    theme.c(&theme.accent).linear_multiply(0.3)
                };
                draw_wave_preview(
                    ui.painter(),
                    rect,
                    self.osc_wave[i],
                    self.osc_pulse_width[i],
                    theme.c(&theme.scope_bg),
                    line_color,
                    theme.rounding_sm,
                    theme.stroke_focus,
                );
            }
        });
    }

    // ── Back face: OSC 1 mod controls ────────────────────────────────────────

    fn ui_osc1_mod_back(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let sync_accent = theme.c(&theme.accent_hard_sync);
        let fm_accent = theme.c(&theme.accent_fm);
        let ring_accent = theme.c(&theme.accent_ring);

        // SYNC toggle + → OSC 2 label
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xs;
            let mut on = self.hard_sync;
            if ui
                .synth_toggle(
                    &mut on,
                    "SYNC",
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    Some(sync_accent),
                )
                .on_hover_text("Hard Sync — OSC 1 resets OSC 2 phase each cycle")
                .clicked()
            {
                self.hard_sync = on;
                self.engine.set_hard_sync_enabled(self.hard_sync);
            }
            ui.label(
                RichText::new("→ OSC 2")
                    .font(theme.font_body())
                    .color(theme.c(&theme.text_disabled)),
            );
        });

        ui.add_space(theme.sp_sm);

        // FM toggle + depth slider
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xs;
            let mut on = self.fm_enabled;
            if ui
                .synth_toggle(
                    &mut on,
                    "FM",
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    Some(fm_accent),
                )
                .on_hover_text("Frequency Modulation — OSC 2 modulates OSC 1 pitch at audio rate")
                .clicked()
            {
                self.fm_enabled = on;
                self.engine
                    .set_fm_depth(if self.fm_enabled { self.fm_depth } else { 0.0 });
            }
            ui.add_enabled_ui(self.fm_enabled, |ui| {
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::Slider::new(&mut self.fm_depth, 0.0..=10.0).fixed_decimals(1),
                    )
                    .on_hover_text("FM depth — ~1 subtle, 3–5 bells, 8+ chaotic sidebands")
                    .changed()
                {
                    self.engine.set_fm_depth(self.fm_depth);
                }
            });
        });

        ui.add_space(theme.sp_xs);

        // RING toggle + depth slider
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xs;
            let mut on = self.ring_enabled;
            if ui
                .synth_toggle(
                    &mut on,
                    "RING",
                    ToggleSize::Standard,
                    Tier::Secondary,
                    &theme,
                    Some(ring_accent),
                )
                .on_hover_text("Ring Mod — OSC 1 × OSC 2: metallic, bell-like textures")
                .clicked()
            {
                self.ring_enabled = on;
                self.engine.set_ring_depth(if self.ring_enabled {
                    self.ring_depth
                } else {
                    0.0
                });
            }
            ui.add_enabled_ui(self.ring_enabled, |ui| {
                if ui
                    .add_sized(
                        [ui.available_width(), 24.0],
                        egui::Slider::new(&mut self.ring_depth, 0.0..=2.0).fixed_decimals(2),
                    )
                    .on_hover_text("Ring mod depth — mute OSC 1 and 2 in mixer for pure ring mod")
                    .changed()
                {
                    self.engine.set_ring_depth(self.ring_depth);
                }
            });
        });
    }

    // ── Audio helpers ─────────────────────────────────────────────────────────

    pub fn update_freq_mult(&self, i: usize) {
        let oct = self.osc_octave[i] as f32;
        let cents = self.osc_detune[i];
        let mult = 2_f32.powf(oct + cents / 1200.0);
        self.engine.set_osc_freq_mult(i as u8, mult);
    }

    pub fn update_unison(&self, i: usize) {
        let count = self.osc_unison_count[i];
        let spread = self.osc_unison_spread[i];
        let osc = i as u8;

        if !self.osc_unison_enabled[i] || count <= 1 {
            for c in 0..5 {
                self.engine.set_osc_unison_detune(osc, c as u8, 1.0);
                self.engine
                    .set_osc_unison_vol(osc, c as u8, if c == 0 { 1.0 } else { 0.0 });
            }
            return;
        }

        let vol = 1.0 / count as f32;
        for c in 0..5 {
            if c < count {
                let t = if count > 1 {
                    c as f32 / (count - 1) as f32
                } else {
                    0.5
                };
                let cents = -spread * 0.5 + t * spread;
                let detune = 2_f32.powf(cents / 1200.0);
                self.engine.set_osc_unison_detune(osc, c as u8, detune);
                self.engine.set_osc_unison_vol(osc, c as u8, vol);
            } else {
                self.engine.set_osc_unison_detune(osc, c as u8, 1.0);
                self.engine.set_osc_unison_vol(osc, c as u8, 0.0);
            }
        }
    }

    pub fn ui_mixer_panel(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let limiter_accent = theme.c(&theme.accent_limiter);

        SynthFrame::section(&theme).show(ui, |ui| {
            let total_h = CARD_H;
            const FADER_COL_W: f32 = 20.0;
            const SLIDER_W: f32 = 8.0;
            const CH_W_CONST: f32 = 5.0;
            const CH_GAP_CONST: f32 = 1.0;
            const METER_TOTAL_W_CONST: f32 = CH_W_CONST * 2.0 + CH_GAP_CONST + 4.0;

            // Cap the section's inner width so ui.horizontal() doesn't grab the
            // full column width (which would leave empty space between controls
            // and meters and cause overflow on small screens).
            let sp = ui.spacing().item_spacing.x;
            let controls_w = FADER_COL_W * 4.0 + sp * 3.0;
            let meter_w = METER_TOTAL_W_CONST + theme.sp_xs * 2.0; // +frame inner margins
            ui.set_max_width(controls_w + sp + meter_w);

            ui.horizontal(|ui| {
                // ── Left: all mixer controls ────────────────────────────────
                ui.vertical(|ui| {
                    let max_w = controls_w;
                    ui.set_max_width(max_w);
                    ui.label(
                        RichText::new("MIX")
                            .font(theme.font_heading())
                            .italics()
                            .color(theme.c(&theme.text_primary)),
                    );
                    ui.add_space(theme.sp_xs);

                    // Per-channel volume faders for OSC 1/2/3 + Noise.
                    // egui::Slider until the design-system Fader component
                    // lands (04-components.md §Fader, Phase 6+).
                    ui.horizontal(|ui| {
                        for i in 0..3 {
                            ui.vertical(|ui| {
                                ui.set_width(FADER_COL_W);
                                ui.label(
                                    RichText::new(format!("O{}", i + 1))
                                        .font(theme.font_body())
                                        .color(if self.osc_enabled[i] {
                                            theme.c(&theme.text_primary)
                                        } else {
                                            theme.c(&theme.text_disabled)
                                        }),
                                );
                                if ui
                                    .add_sized(
                                        [SLIDER_W, 80.0],
                                        egui::Slider::new(&mut self.osc_vol[i], 0.0..=1.0)
                                            .vertical()
                                            .fixed_decimals(2),
                                    )
                                    .on_hover_text(format!("OSC {} volume in the mix", i + 1))
                                    .changed()
                                    && self.osc_enabled[i]
                                {
                                    self.engine.set_osc_vol(i as u8, self.osc_vol[i]);
                                }
                            });
                        }

                        ui.vertical(|ui| {
                            ui.set_width(FADER_COL_W);
                            ui.label(
                                RichText::new("N")
                                    .font(theme.font_body())
                                    .color(theme.c(&theme.text_secondary)),
                            );
                            let mut noise_vol = self.engine.noise_vol();
                            if ui
                                .add_sized(
                                    [SLIDER_W, 80.0],
                                    egui::Slider::new(&mut noise_vol, 0.0..=1.0)
                                        .vertical()
                                        .fixed_decimals(2),
                                )
                                .on_hover_text("White noise volume")
                                .changed()
                            {
                                self.engine.set_noise_vol(noise_vol);
                            }
                        });
                    });

                    ui.add_space(theme.sp_xs);
                    ui.separator();
                    ui.add_space(theme.sp_xs);

                    ui.horizontal(|ui| {
                        // MAST — Master volume is Tier 1 per the philosophy
                        // doc ("performance controls"). Standard size to fit
                        // the existing mixer column width; Tier::Primary gives
                        // it the full accent arc.
                        let mut master = self.engine.master_volume();
                        if ui
                            .synth_knob(
                                &mut master,
                                0.0..=1.0,
                                "MAST",
                                &theme,
                                false,
                                KnobSize::Standard,
                                Tier::Primary,
                            )
                            .on_hover_text("Master output volume — applied after all FX")
                            .changed()
                        {
                            self.engine.set_master_volume(master);
                        }
                        // GLIDE — Tier 2 sound design.
                        let mut glide = self.engine.glide_time();
                        if ui
                            .synth_knob(
                                &mut glide,
                                0.0..=0.5,
                                "GLIDE",
                                &theme,
                                false,
                                KnobSize::Standard,
                                Tier::Secondary,
                            )
                            .on_hover_text("Pitch slide time between notes (seconds)")
                            .changed()
                        {
                            self.engine.set_glide_time(glide);
                        }

                        // VOICE — chip selector for POLY / MONO / LEGATO.
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("VOICE")
                                    .weak()
                                    .font(theme.font_body())
                                    .color(theme.c(&theme.text_secondary)),
                            );
                            let mut mode = self.engine.mono_mode();
                            let prev = mode;
                            ui.chip_selector(
                                &mut mode,
                                &[
                                    (0u8, "POLY"),
                                    (1u8, "MONO"),
                                    (2u8, "LEG"),
                                ],
                                &theme,
                                None,
                            )
                            .on_hover_text(
                                "POLY: multiple voices · MONO: single voice retriggered · LEG: single voice with glide",
                            );
                            if mode != prev {
                                self.engine.set_mono_mode(mode);
                            }
                        });
                    });

                    ui.add_space(theme.sp_xs);

                    ui.horizontal(|ui| {
                        let mut lim_on = self.limiter_enabled;
                        if ui
                            .synth_toggle(
                                &mut lim_on,
                                "LIM",
                                ToggleSize::Standard,
                                Tier::Secondary,
                                &theme,
                                Some(limiter_accent),
                            )
                            .on_hover_text("Limiter — prevents output clipping")
                            .clicked()
                        {
                            self.limiter_enabled = lim_on;
                            self.engine.set_limiter_enabled(self.limiter_enabled);
                        }
                        ui.add_enabled_ui(self.limiter_enabled, |ui| {
                            let mut thr = self.engine.limiter_threshold();
                            if ui
                                .add(
                                    egui::DragValue::new(&mut thr)
                                        .range(0.5..=1.0)
                                        .speed(0.005)
                                        .fixed_decimals(2),
                                )
                                .on_hover_text("Threshold — lower = more compression")
                                .changed()
                                && self.limiter_enabled
                            {
                                self.engine.set_limiter_threshold(thr);
                            }
                        });
                    });
                });

                // ── Right: L/R peak meters, full card height ─────────────────
                let peak_raw_l = self.engine.peak_l();
                let peak_raw_r = self.engine.peak_r();
                let dt = 1.0 / 60.0_f32;
                self.peak_display =
                    (self.peak_display * 0.85 + peak_raw_l * 0.15).max(peak_raw_l * 0.3);
                let peak_raw_max = peak_raw_l.max(peak_raw_r);
                if peak_raw_max > self.peak_hold {
                    self.peak_hold = peak_raw_max;
                    self.peak_hold_timer = 0.0;
                } else {
                    self.peak_hold_timer += dt;
                    if self.peak_hold_timer > 1.5 {
                        self.peak_hold *= 0.97;
                    }
                }

                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(theme.sp_xs as i8, 0))
                    .show(ui, |ui| {
                        let (resp, painter) = ui.allocate_painter(
                            Vec2::new(METER_TOTAL_W_CONST, total_h),
                            Sense::hover(),
                        );
                        let mr = resp.rect;

                        painter.rect_filled(
                            mr,
                            CornerRadius::same(theme.rounding_xs as u8),
                            theme.c(&theme.meter_bg),
                        );

                        for (ci, &peak_raw) in [peak_raw_l, peak_raw_r].iter().enumerate() {
                            let x_left = mr.left() + 2.0 + ci as f32 * (CH_W_CONST + CH_GAP_CONST);
                            let ch_rect = Rect::from_min_size(
                                Pos2::new(x_left, mr.top() + 2.0),
                                Vec2::new(CH_W_CONST, mr.height() - 14.0),
                            );
                            let level = peak_raw.clamp(0.0, 1.0);
                            let bar_h = ch_rect.height() * level;
                            if bar_h > 0.5 {
                                let color = if peak_raw < 0.7 {
                                    theme.c(&theme.meter_green)
                                } else if peak_raw < 1.0 {
                                    // Token-derived: interpolation between
                                    // theme.meter_green and theme.meter_clip.
                                    let t = (peak_raw - 0.7) / 0.3;
                                    let g = theme.meter_green;
                                    let c = theme.meter_clip;
                                    Color32::from_rgb(
                                        (g[0] as f32 + (c[0] as f32 - g[0] as f32) * t) as u8,
                                        (g[1] as f32 + (c[1] as f32 - g[1] as f32) * t) as u8,
                                        (g[2] as f32 + (c[2] as f32 - g[2] as f32) * t) as u8,
                                    )
                                } else {
                                    theme.c(&theme.meter_clip)
                                };
                                let bar_rect = Rect::from_min_size(
                                    Pos2::new(ch_rect.left(), ch_rect.bottom() - bar_h),
                                    Vec2::new(CH_W_CONST, bar_h),
                                );
                                painter.rect_filled(bar_rect, CornerRadius::ZERO, color);
                            }

                            let hold_frac = self.peak_hold.clamp(0.0, 1.0);
                            let hold_y = ch_rect.bottom() - ch_rect.height() * hold_frac;
                            let hold_color = if self.peak_hold >= 1.0 {
                                theme.c(&theme.meter_clip)
                            } else {
                                theme.c(&theme.text_primary)
                            };
                            painter.line_segment(
                                [
                                    Pos2::new(ch_rect.left(), hold_y),
                                    Pos2::new(ch_rect.right(), hold_y),
                                ],
                                Stroke::new(theme.stroke_focus, hold_color),
                            );

                            painter.text(
                                Pos2::new(ch_rect.center_top().x, mr.bottom() - 2.0),
                                egui::Align2::CENTER_BOTTOM,
                                if ci == 0 { "L" } else { "R" },
                                theme.font_micro(),
                                theme.c(&theme.text_secondary),
                            );
                        }
                    }); // Frame::new inner_margin
            });
            // Pad to shared fixed card height.
            ui.add_space((CARD_H - ui.min_rect().height()).max(0.0));
        });
    }
}

// ── Waveform preview painter ──────────────────────────────────────────────────

fn draw_wave_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    wave: usize,
    pulse_width: f32,
    bg: Color32,
    line_color: Color32,
    rounding: f32,
    stroke_w: f32,
) {
    painter.rect_filled(rect, egui::CornerRadius::same(rounding as u8), bg);

    let w = rect.width();
    let h = rect.height();
    let cx = rect.left();
    let cy = rect.center().y;
    let amp = h * 0.38;
    let cycles = 2.0_f32;
    let steps = 80usize;

    let points: Vec<Pos2> = (0..=steps)
        .map(|s| {
            let t = s as f32 / steps as f32;
            let norm_phase = (t * cycles).fract();
            let phase_rad = t * cycles * std::f32::consts::TAU;

            let y = match wave {
                0 => phase_rad.sin(),
                1 => 1.0 - 2.0 * norm_phase,
                2 => {
                    if norm_phase < pulse_width {
                        1.0
                    } else {
                        -1.0
                    }
                }
                3 => {
                    if norm_phase < 0.5 {
                        4.0 * norm_phase - 1.0
                    } else {
                        3.0 - 4.0 * norm_phase
                    }
                }
                _ => 0.0,
            };

            Pos2::new(cx + t * w, cy - y * amp)
        })
        .collect();

    let clip = painter.clip_rect();
    let painter = painter.with_clip_rect(clip.intersect(rect));
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(stroke_w, line_color));
    }
}
