use crate::audio::TRACK_COUNT;
use crate::ui::design::level_meter::{level_meter, LevelMeterOrientation, LevelMeterSize};
use crate::ui::layout::AppMode;
use crate::SynthApp;
use eframe::egui;
use egui::{Color32, Sense, Stroke, Vec2};

impl SynthApp {
    /// Rig strip panel — rendered as a TopBottomPanel above the synth editor in LIVE mode.
    pub fn ui_rig_strip(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.c(&self.theme.accent);
        let text_sec = self.theme.c(&self.theme.text_secondary);
        let text_dis = self.theme.c(&self.theme.text_disabled);
        let bg_surface = self.theme.c(&self.theme.bg_surface);
        let border = self.theme.c(&self.theme.border);
        let seq_rec = self.theme.c(&self.theme.seq_rec_cursor);
        let sp_xxs = self.theme.sp_xxs;
        let sp_sm = self.theme.sp_sm;
        let rounding_sm = egui::CornerRadius::same(self.theme.rounding_sm as u8);

        ui.horizontal(|ui| {
            // ── Synth track cells ────────────────────────────────────────────
            for t in 0..TRACK_COUNT {
                let focused = self.focused_track == t;
                let muted = self.track_mixer[t].muted();
                let solo = self.track_mixer[t].solo();
                let peak = self.track_mixer[t].peak();

                // Cell frame
                let cell_fill = if focused { bg_surface } else { Color32::TRANSPARENT };
                let cell_stroke = if focused {
                    Stroke::new(self.theme.stroke_ui, accent)
                } else {
                    Stroke::new(self.theme.stroke_ui * 0.5, border)
                };

                egui::Frame::new()
                    .fill(cell_fill)
                    .stroke(cell_stroke)
                    .corner_radius(rounding_sm)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.set_min_width(90.0);

                        // Track name + playing indicator
                        ui.horizontal(|ui| {
                            let dot_col = if !muted { accent } else { border };
                            ui.label(
                                egui::RichText::new("●").font(self.theme.font_small()).color(dot_col),
                            );
                            let name_col = if focused { accent } else { text_sec };
                            let label = egui::RichText::new(
                                format!("T{}  {}", t + 1, self.track_names[t]),
                            )
                            .font(self.theme.font_body())
                            .color(name_col);
                            if ui
                                .add(egui::Label::new(label).sense(Sense::click()))
                                .clicked()
                            {
                                self.switch_focused_track(t);
                                // Switch to LIVE mode if in STUDIO
                                if self.app_mode == AppMode::Studio {
                                    self.app_mode = AppMode::Live;
                                }
                            }
                        });

                        // VU meter bar — horizontal, compact
                        level_meter(
                            ui,
                            peak.min(1.0),
                            0.0,
                            LevelMeterOrientation::Horizontal,
                            LevelMeterSize::Standard,
                            &self.theme,
                        );

                        // Patch name
                        ui.label(
                            egui::RichText::new(&self.track_patches[t].name)
                                .font(self.theme.font_body())
                                .color(text_dis),
                        );

                        // M / S buttons
                        ui.horizontal(|ui| {
                            let m_col = if muted { seq_rec } else { text_dis };
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("M")
                                            .font(self.theme.font_body())
                                            .color(m_col),
                                    )
                                    .frame(muted)
                                    .min_size(Vec2::new(18.0, 14.0)),
                                )
                                .on_hover_text("Mute this track")
                                .clicked()
                            {
                                self.track_mixer[t].set_muted(!muted);
                            }

                            let s_col = if solo { accent } else { text_dis };
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("S")
                                            .font(self.theme.font_body())
                                            .color(s_col),
                                    )
                                    .frame(solo)
                                    .min_size(Vec2::new(18.0, 14.0)),
                                )
                                .on_hover_text("Solo this track")
                                .clicked()
                            {
                                self.track_mixer[t].set_solo(!solo);
                            }
                        });
                    });

                ui.add_space(sp_xxs);
            }

            // ── Drums cell ────────────────────────────────────────────────────
            let drums_focused = self.app_mode == AppMode::DrumMachine;
            let drums_cell_stroke = if drums_focused {
                Stroke::new(self.theme.stroke_ui, accent)
            } else {
                Stroke::new(self.theme.stroke_ui * 0.5, border)
            };
            egui::Frame::new()
                .fill(if drums_focused { bg_surface } else { Color32::TRANSPARENT })
                .stroke(drums_cell_stroke)
                .corner_radius(rounding_sm)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.set_min_width(64.0);
                    ui.horizontal(|ui| {
                        let dot_col = if self.drums.enabled { accent } else { border };
                        ui.label(
                            egui::RichText::new("●").font(self.theme.font_small()).color(dot_col),
                        );
                        let col = if drums_focused { accent } else { text_sec };
                        if ui
                            .add(
                                egui::Label::new(
                                    egui::RichText::new("DRUMS")
                                        .font(self.theme.font_body())
                                        .color(col),
                                )
                                .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            self.app_mode = AppMode::DrumMachine;
                        }
                    });
                    // Placeholder VU (drum engine not yet wired to rig strip)
                    level_meter(
                        ui,
                        0.0,
                        0.0,
                        LevelMeterOrientation::Horizontal,
                        LevelMeterSize::Small,
                        &self.theme,
                    );
                    ui.label(
                        egui::RichText::new("Phase 5")
                            .font(self.theme.font_body())
                            .color(text_dis),
                    );
                    let drum_muted = !self.drums.enabled;
                    let m_col = if drum_muted { seq_rec } else { text_dis };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("M").font(self.theme.font_body()).color(m_col),
                            )
                            .frame(drum_muted)
                            .min_size(Vec2::new(18.0, 14.0)),
                        )
                        .on_hover_text("Mute drums")
                        .clicked()
                    {
                        self.drums.enabled = !self.drums.enabled;
                    }
                });

            ui.add_space(sp_sm);
            ui.separator();
            ui.add_space(sp_xxs + sp_xxs);

            // ── Transport + MIX▸ ─────────────────────────────────────────────
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(format!("♩ {} BPM", self.global_bpm))
                        .font(self.theme.font_body())
                        .color(accent),
                );
                let mix_col = if self.show_mixer { accent } else { text_dis };
                if ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new("MIX▸").font(self.theme.font_body()).color(mix_col),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .on_hover_text("Toggle mixer panel")
                    .clicked()
                {
                    self.show_mixer = !self.show_mixer;
                }
            });
        });

        let _ = (seq_rec, sp_xxs, sp_sm, rounding_sm, border);
    }
}
