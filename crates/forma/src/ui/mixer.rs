use crate::audio::TRACK_COUNT;
use crate::ui::design::level_meter::{level_meter, LevelMeterOrientation, LevelMeterSize};
use crate::SynthApp;
use eframe::egui;
use egui::{Sense, Vec2};
use std::sync::Arc;

const FADER_HEIGHT: f32 = 120.0;
const CHANNEL_WIDTH: f32 = 56.0;

impl SynthApp {
    pub fn ui_mix_board(&mut self, ui: &mut egui::Ui) {
        let accent = self.theme.c(&self.theme.accent);
        let text_sec = self.theme.c(&self.theme.text_secondary);
        let text_dis = self.theme.c(&self.theme.text_disabled);
        let border = self.theme.c(&self.theme.border);
        let seq_rec = self.theme.c(&self.theme.seq_rec_cursor);
        let sp_xs = self.theme.sp_xs;
        let sp_xxs = self.theme.sp_xxs;

        // Header
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("MIXER").font(self.theme.font_heading()).color(accent));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Label::new(
                            egui::RichText::new("✕").font(self.theme.font_heading()).color(text_dis),
                        )
                        .sense(Sense::click()),
                    )
                    .clicked()
                {
                    self.show_mixer = false;
                }
            });
        });

        ui.add_space(sp_xs);

        // Check if any track is soloed
        let any_solo = (0..TRACK_COUNT).any(|t| self.track_mixer[t].solo());

        ui.horizontal(|ui| {
            // ── Synth track channels ──────────────────────────────────────────
            for t in 0..TRACK_COUNT {
                let focused = self.focused_track == t;
                let mixer = Arc::clone(&self.track_mixer[t]);

                let mut vol = mixer.volume();
                let mut pan = mixer.pan();
                let muted = mixer.muted();
                let solo = mixer.solo();
                let peak = mixer.peak();

                // A track is effectively silent if muted or if another track is soloed
                let silenced = muted || (any_solo && !solo);

                ui.vertical(|ui| {
                    ui.set_min_width(CHANNEL_WIDTH);

                    // Track name
                    let name_col = if focused { accent } else { text_sec };
                    ui.label(
                        egui::RichText::new(format!("T{}", t + 1))
                            .font(self.theme.font_body())
                            .color(name_col),
                    );
                    ui.label(
                        egui::RichText::new(&self.track_names[t])
                            .font(self.theme.font_small())
                            .color(text_dis),
                    );

                    ui.add_space(sp_xs);

                    // VU + fader together in a horizontal strip
                    ui.horizontal(|ui| {
                        let display_level = if silenced { 0.0 } else { peak.min(1.0) };
                        level_meter(
                            ui,
                            display_level,
                            0.0,
                            LevelMeterOrientation::Vertical,
                            LevelMeterSize::Large,
                            &self.theme,
                        );

                        ui.add_space(sp_xxs);

                        let fader_resp = ui.add(
                            egui::Slider::new(&mut vol, 0.0..=1.0)
                                .vertical()
                                .show_value(false)
                                .text("")
                                .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 2.0 }),
                        );
                        if fader_resp.changed() {
                            mixer.set_volume(vol);
                        }
                    });

                    ui.add_space(sp_xxs);

                    // Volume readout
                    ui.label(
                        egui::RichText::new(format!("{:.0}%", vol * 100.0))
                            .font(self.theme.font_small())
                            .color(text_dis),
                    );

                    ui.add_space(sp_xxs);

                    // Pan slider
                    let pan_resp = ui.add(
                        egui::Slider::new(&mut pan, -1.0..=1.0)
                            .show_value(false)
                            .text("")
                            .fixed_decimals(2),
                    );
                    if pan_resp.changed() {
                        mixer.set_pan(pan);
                    }
                    // Pan readout
                    let pan_label = if pan.abs() < 0.02 {
                        "C".to_string()
                    } else if pan < 0.0 {
                        format!("L{:.0}", pan.abs() * 100.0)
                    } else {
                        format!("R{:.0}", pan * 100.0)
                    };
                    ui.label(egui::RichText::new(pan_label).font(self.theme.font_small()).color(text_dis));

                    ui.add_space(sp_xs);

                    // M / S buttons
                    ui.horizontal(|ui| {
                        let m_col = if muted { seq_rec } else { text_dis };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("M").font(self.theme.font_body()).color(m_col),
                                )
                                .frame(muted)
                                .min_size(Vec2::new(20.0, 14.0)),
                            )
                            .on_hover_text("Mute")
                            .clicked()
                        {
                            mixer.set_muted(!muted);
                        }
                        let s_col = if solo { accent } else { text_dis };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("S").font(self.theme.font_body()).color(s_col),
                                )
                                .frame(solo)
                                .min_size(Vec2::new(20.0, 14.0)),
                            )
                            .on_hover_text("Solo")
                            .clicked()
                        {
                            mixer.set_solo(!solo);
                        }
                    });

                    // Focused indicator
                    if focused {
                        let (r, _) = ui.allocate_exact_size(
                            Vec2::new(CHANNEL_WIDTH - 4.0, 2.0),
                            Sense::hover(),
                        );
                        ui.painter_at(r).rect_filled(r, 0.0, accent);
                    }
                });

                // Divider between channels
                if t < TRACK_COUNT - 1 {
                    ui.add_space(sp_xxs);
                    let (line_rect, _) =
                        ui.allocate_exact_size(Vec2::new(1.0, FADER_HEIGHT + 80.0), Sense::hover());
                    ui.painter_at(line_rect).rect_filled(line_rect, 0.0, border);
                    ui.add_space(sp_xxs);
                }
            }

            // ── Drum bus channel ─────────────────────────────────────────────
            ui.add_space(sp_xxs);
            let (line_rect, _) =
                ui.allocate_exact_size(Vec2::new(1.0, FADER_HEIGHT + 80.0), Sense::hover());
            ui.painter_at(line_rect).rect_filled(line_rect, 0.0, border);
            ui.add_space(sp_xxs);

            ui.vertical(|ui| {
                ui.set_min_width(CHANNEL_WIDTH);

                let drums_col = if self.app_mode == crate::ui::layout::AppMode::DrumMachine {
                    accent
                } else {
                    text_sec
                };
                ui.label(egui::RichText::new("DRUMS").font(self.theme.font_body()).color(drums_col));
                ui.label(egui::RichText::new("step seq").font(self.theme.font_small()).color(text_dis));

                ui.add_space(sp_xs);

                let drum_engine = std::sync::Arc::clone(&self.drum_engine);
                let mut dvol = drum_engine.volume();
                let mut dpan = drum_engine.pan();
                let drum_muted = drum_engine.muted.load(std::sync::atomic::Ordering::Relaxed);
                let dpeak = drum_engine.peak();

                // VU + volume fader
                ui.horizontal(|ui| {
                    let display_level = if drum_muted { 0.0 } else { dpeak.min(1.0) };
                    level_meter(
                        ui,
                        display_level,
                        0.0,
                        LevelMeterOrientation::Vertical,
                        LevelMeterSize::Large,
                        &self.theme,
                    );
                    ui.add_space(sp_xxs);
                    if ui
                        .add(
                            egui::Slider::new(&mut dvol, 0.0..=1.0)
                                .vertical()
                                .show_value(false)
                                .text("")
                                .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: 2.0 }),
                        )
                        .changed()
                    {
                        drum_engine.set_volume(dvol);
                    }
                });

                ui.label(
                    egui::RichText::new(format!("{:.0}%", dvol * 100.0))
                        .font(self.theme.font_small())
                        .color(text_dis),
                );
                ui.add_space(sp_xxs);

                let pan_resp = ui.add(
                    egui::Slider::new(&mut dpan, -1.0..=1.0)
                        .show_value(false)
                        .text(""),
                );
                if pan_resp.changed() {
                    drum_engine.set_pan(dpan);
                }
                let pan_label = if dpan.abs() < 0.02 {
                    "C".to_string()
                } else if dpan < 0.0 {
                    format!("L{:.0}", dpan.abs() * 100.0)
                } else {
                    format!("R{:.0}", dpan * 100.0)
                };
                ui.label(egui::RichText::new(pan_label).font(self.theme.font_small()).color(text_dis));
                ui.add_space(sp_xs);

                let m_col = if drum_muted { seq_rec } else { text_dis };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("M").font(self.theme.font_body()).color(m_col),
                        )
                        .frame(drum_muted)
                        .min_size(Vec2::new(20.0, 14.0)),
                    )
                    .on_hover_text("Mute drum bus")
                    .clicked()
                {
                    drum_engine
                        .muted
                        .store(!drum_muted, std::sync::atomic::Ordering::Relaxed);
                }
            });
        });

        let _ = (seq_rec, sp_xs, sp_xxs, border);
    }
}
