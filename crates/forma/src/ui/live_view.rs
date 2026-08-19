use crate::audio::TRACK_COUNT;
use crate::ui::layout::AppMode;
use crate::SynthApp;
use eframe::egui;
use egui::{Color32, Rect, Sense, Vec2};
use std::sync::Arc;

const STRIP_H: f32 = 48.0;

impl SynthApp {
    pub fn ui_live_view(&mut self, ui: &mut egui::Ui) {
        // Compact track strip — shows VU, mute, patch name for each track.
        egui::TopBottomPanel::top("live_strip")
            .exact_height(STRIP_H)
            .frame(
                egui::Frame::new()
                    .fill(self.theme.c(&self.theme.bg_bar))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .stroke(egui::Stroke::new(0.5, self.theme.c(&self.theme.border))),
            )
            .show_inside(ui, |ui| {
                self.ui_loop_track_strip(ui);
            });

        // Full-height dark grid surface.
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(self.theme.c(&self.theme.bg_app)))
            .show_inside(ui, |ui| {
                // Recording / playback / timing all run on the audio thread; the
                // UI only reads back state and posts requests.
                self.ui_loop_grid(ui);
            });
    }

    fn ui_loop_track_strip(&mut self, ui: &mut egui::Ui) {
        let text_sec = self.theme.c(&self.theme.text_secondary);
        let text_dis = self.theme.c(&self.theme.text_disabled);
        let border = self.theme.c(&self.theme.border);
        let accent = self.theme.c(&self.theme.accent);
        let meter_bg = self.theme.c(&self.theme.meter_bg);
        let meter_green = self.theme.c(&self.theme.meter_green);
        let meter_clip = self.theme.c(&self.theme.meter_clip);

        ui.horizontal(|ui| {
            let mut switch_to: Option<usize> = None;

            #[allow(clippy::needless_range_loop)]
            for t in 0..TRACK_COUNT {
                let mixer = Arc::clone(&self.track_mixer[t]);
                let peak = mixer.peak();
                let muted = mixer.muted();
                let solo = mixer.solo();
                let patch_name = self.track_patches[t].name.clone();
                let track_name = self.track_names[t].clone();
                let focused = self.focused_track == t && self.app_mode == AppMode::Live;

                let card_stroke = egui::Stroke::new(
                    if focused {
                        self.theme.stroke_focus
                    } else {
                        self.theme.stroke_ui
                    },
                    if focused { accent } else { border },
                );
                let card_fill = if focused {
                    accent.gamma_multiply(0.10)
                } else {
                    Color32::TRANSPARENT
                };

                let resp = egui::Frame::new()
                    .fill(card_fill)
                    .stroke(card_stroke)
                    .inner_margin(egui::Margin::symmetric(6, 3))
                    .corner_radius(egui::CornerRadius::same(self.theme.rounding_sm as u8))
                    .show(ui, |ui| {
                        ui.set_min_width(120.0);
                        ui.set_max_width(160.0);

                        // Track name + patch
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("T{}  {}", t + 1, track_name))
                                    .font(self.theme.font_body())
                                    .color(if focused { accent } else { text_sec }),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // M button
                                    let m_col = if muted { meter_clip } else { text_dis };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("M")
                                                    .font(self.theme.font_small())
                                                    .color(m_col),
                                            )
                                            .frame(muted)
                                            .min_size(Vec2::new(16.0, 14.0)),
                                        )
                                        .clicked()
                                    {
                                        mixer.set_muted(!muted);
                                    }

                                    let s_col = if solo { accent } else { text_dis };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new("S")
                                                    .font(self.theme.font_small())
                                                    .color(s_col),
                                            )
                                            .frame(solo)
                                            .min_size(Vec2::new(16.0, 14.0)),
                                        )
                                        .clicked()
                                    {
                                        mixer.set_solo(!solo);
                                    }
                                },
                            );
                        });

                        // VU + patch name row
                        ui.horizontal(|ui| {
                            let vu_w = 80.0_f32;
                            let (vu_rect, _) =
                                ui.allocate_exact_size(Vec2::new(vu_w, 4.0), Sense::hover());
                            let painter = ui.painter_at(vu_rect);
                            painter.rect_filled(vu_rect, 1.0, meter_bg);
                            let fill_w = (peak.min(1.0) * vu_w).max(0.0);
                            if fill_w > 0.5 {
                                let bar_col = if muted {
                                    border
                                } else if peak > 0.9 {
                                    meter_clip
                                } else {
                                    meter_green
                                };
                                painter.rect_filled(
                                    Rect::from_min_size(vu_rect.min, Vec2::new(fill_w, 4.0)),
                                    1.0,
                                    bar_col,
                                );
                            }
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(&patch_name)
                                    .font(self.theme.font_small())
                                    .color(text_dis),
                            );
                        });
                    })
                    .response;

                if resp.clicked() {
                    switch_to = Some(t);
                }
                ui.add_space(4.0);
            }

            if let Some(t) = switch_to {
                self.switch_focused_track(t);
                self.app_mode = AppMode::Live;
            }
        });
    }
}
