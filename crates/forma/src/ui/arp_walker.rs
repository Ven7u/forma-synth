use crate::ui::frame::SynthFrame;
use crate::SynthApp;
use eframe::egui;
use egui::{Pos2, RichText, Sense, Stroke, Vec2};
use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::atomic::Ordering;

impl SynthApp {
    pub fn ui_arp_panel(&mut self, ui: &mut egui::Ui) {
        use forma_engine::arp::{ArpMode, ClockDiv};

        let theme = self.theme.clone();
        let enabled = self.engine.arp_enabled();

        // ── ARP card ─────────────────────────────────────────────────────────
        SynthFrame::section(&theme).show(ui, |ui| {
            ui.label(
                RichText::new("ARP")
                    .font(theme.font_heading())
                    .color(theme.c(&theme.text_primary)),
            );
            ui.add_space(theme.sp_xs);

            // Transport row — ARP / RST / HOLD
            ui.horizontal(|ui| {
                let bar_quantize = self.seq.bar_quantize.load(Ordering::Relaxed);
                let arp_label = if self.arp_pending_start {
                    RichText::new("… Bar")
                        .strong()
                        .color(theme.c(&theme.accent_hold))
                } else {
                    RichText::new("ARP").strong().color(if enabled {
                        theme.c(&theme.accent)
                    } else {
                        theme.c(&theme.text_disabled)
                    })
                };
                let arp_tip = if self.arp_pending_start {
                    "Waiting for the next bar boundary — click to cancel."
                } else if bar_quantize && self.arp_sync_active() {
                    "Toggle arp — start is quantized to the next bar (BAR is on)."
                } else {
                    "Toggle arp on/off."
                };
                if ui.button(arp_label).on_hover_text(arp_tip).clicked() {
                    if self.arp_pending_start {
                        self.arp_pending_start = false;
                    } else {
                        let new_enabled = !enabled;
                        let frozen_chord: Vec<u8> = if new_enabled && self.kb_freeze {
                            self.frozen_notes.iter().copied().collect()
                        } else {
                            vec![]
                        };
                        self.all_notes_off();
                        self.engine.set_arp_enabled(new_enabled);
                        if new_enabled {
                            if self.kb_freeze {
                                self.kb_freeze = false;
                            }
                            if !frozen_chord.is_empty() {
                                self.engine.chord_hold(&frozen_chord);
                            }
                            if self.arp_sync_active() {
                                self.apply_clock_sync();
                                self.schedule_or_restart_arp();
                            }
                        }
                        if !new_enabled {
                            self.arp_pending_start = false;
                            self.engine.chord_hold(&[]);
                        }
                    }
                }

                let rst_tip =
                    if self.seq.bar_quantize.load(Ordering::Relaxed) && self.arp_sync_active() {
                        "Restart arp at the next bar boundary (BAR is on)."
                    } else {
                        "Restart arp phase/step from beginning."
                    };
                if ui.button("RST").on_hover_text(rst_tip).clicked() {
                    self.schedule_or_restart_arp();
                }

                let hold = self.engine.arp_hold();
                let hold_label = RichText::new("HOLD").color(if hold {
                    theme.c(&theme.accent_hold)
                } else {
                    theme.c(&theme.text_disabled)
                });
                if ui.button(hold_label).clicked() {
                    let new_hold = !hold;
                    self.engine.set_arp_hold(new_hold);
                    if !new_hold {
                        self.engine.chord_hold(&[]);
                    }
                }
            });

            // BPM + Sync
            ui.horizontal(|ui| {
                ui.label("BPM:");
                let sync_active = self.arp_sync_active();
                if sync_active {
                    self.engine
                        .set_arp_bpm(self.seq.bpm.load(Ordering::Relaxed) as f32);
                }
                let mut bpm = self.engine.arp_bpm();
                ui.add_enabled_ui(!sync_active, |ui| {
                    if ui.add(egui::Slider::new(&mut bpm, 20.0..=300.0)).changed() {
                        self.engine.set_arp_bpm(bpm);
                    }
                });
                ui.add_enabled_ui(!self.global_sync, |ui| {
                    let sync_label = RichText::new("Sync").color(if self.arp_sync_active() {
                        theme.c(&theme.accent)
                    } else {
                        theme.c(&theme.text_disabled)
                    });
                    if ui
                        .button(sync_label)
                        .on_hover_text("Lock Arp BPM to the Global BPM.")
                        .clicked()
                    {
                        self.arp_sync = !self.arp_sync;
                        if self.arp_sync {
                            self.apply_clock_sync();
                            self.schedule_or_restart_arp();
                        } else {
                            self.seq.arp_restart.store(false, Ordering::Relaxed);
                        }
                    }
                });
            });

            ui.add_enabled_ui(enabled, |ui| {
                // Div
                let current_div = self.engine.arp_division();
                ui.horizontal(|ui| {
                    ui.label("Div:");
                    for (i, &label) in ClockDiv::LABELS.iter().enumerate() {
                        let active = current_div == i as u8;
                        let col = if active {
                            theme.c(&theme.accent_dim)
                        } else {
                            theme.c(&theme.text_disabled)
                        };
                        if ui.button(RichText::new(label).color(col)).clicked() && !active {
                            self.engine.set_arp_division(i as u8);
                        }
                    }
                });

                // Mode
                let current_mode = self.engine.arp_mode();
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    for (i, &label) in ArpMode::LABELS.iter().enumerate() {
                        let active = current_mode == i as u8;
                        let col = if active {
                            theme.c(&theme.accent_dim)
                        } else {
                            theme.c(&theme.text_disabled)
                        };
                        if ui.button(RichText::new(label).color(col)).clicked() && !active {
                            self.engine.set_arp_mode(i as u8);
                        }
                    }
                });

                // Oct + Gate
                let current_oct = self.engine.arp_octave_range();
                ui.horizontal(|ui| {
                    ui.label("Oct:");
                    for oct in 1u8..=4 {
                        let active = current_oct == oct;
                        let col = if active {
                            theme.c(&theme.accent_dim)
                        } else {
                            theme.c(&theme.text_disabled)
                        };
                        if ui
                            .button(RichText::new(oct.to_string()).color(col))
                            .clicked()
                            && !active
                        {
                            self.engine.set_arp_octave_range(oct);
                        }
                    }
                    ui.separator();
                    ui.label("Gate:");
                    let mut gate = self.engine.arp_gate();
                    if ui.add(egui::Slider::new(&mut gate, 0.05..=1.0)).changed() {
                        self.engine.set_arp_gate(gate);
                    }
                });
            });
        });

        ui.add_space(theme.sp_xs);

        // ── RING card ─────────────────────────────────────────────────────────
        SynthFrame::section(&theme).show(ui, |ui| {
            ui.label(
                RichText::new("RING")
                    .font(theme.font_heading())
                    .color(theme.c(&theme.text_primary)),
            );
            ui.add_space(theme.sp_xs);
            ui.add_enabled_ui(enabled, |ui| {
                self.ui_arp_ring(ui);
            });
        });
    }

    fn ui_arp_ring(&mut self, ui: &mut egui::Ui) {
        use forma_engine::arp::{euclidean_pattern, RING_PRESETS};

        let theme = self.theme.clone();
        let accent = theme.c(&theme.accent);

        // Controls row — RING toggle / N / K / Gen
        ui.horizontal(|ui| {
            let ring_col = if self.arp_ring_enabled {
                theme.c(&theme.accent_ring)
            } else {
                theme.c(&theme.text_disabled)
            };
            if ui
                .button(RichText::new("RING").color(ring_col))
                .on_hover_text("Euclidean ring gate — controls which arp steps fire a note.")
                .clicked()
            {
                self.arp_ring_enabled = !self.arp_ring_enabled;
                self.engine.set_arp_ring_enabled(self.arp_ring_enabled);
            }
            ui.label("N:");
            let mut n = self.arp_ring_steps as usize;
            if ui
                .add(egui::DragValue::new(&mut n).range(2..=16))
                .on_hover_text("Number of steps in the ring (2–16).")
                .changed()
            {
                self.arp_ring_steps = n as u8;
                self.engine.set_arp_ring_steps(n as u8);
                let mask = if n >= 32 { u32::MAX } else { (1u32 << n) - 1 };
                self.arp_ring_pattern &= mask;
                self.engine.set_arp_ring_pattern(self.arp_ring_pattern);
            }
            ui.label("K:");
            let max_k = self.arp_ring_steps as usize;
            let mut k = (self.arp_ring_k as usize).min(max_k);
            if ui
                .add(egui::DragValue::new(&mut k).range(1..=max_k))
                .on_hover_text("Hits to distribute (euclidean generator).")
                .changed()
            {
                self.arp_ring_k = k as u8;
            }
            if ui
                .button("Gen")
                .on_hover_text("Fill ring with K hits spread as evenly as possible (Euclidean).")
                .clicked()
            {
                self.arp_ring_pattern = euclidean_pattern(k, self.arp_ring_steps as usize, 0);
                self.engine.set_arp_ring_pattern(self.arp_ring_pattern);
            }
        });

        // Preset chips
        ui.horizontal_wrapped(|ui| {
            for &(name, k, n, rot) in RING_PRESETS {
                if ui
                    .small_button(name)
                    .on_hover_text(format!("E({k},{n}) rot={rot}"))
                    .clicked()
                {
                    self.arp_ring_steps = n;
                    self.arp_ring_pattern = euclidean_pattern(k as usize, n as usize, rot as usize);
                    self.arp_ring_k = k;
                    self.engine.set_arp_ring_steps(n);
                    self.engine.set_arp_ring_pattern(self.arp_ring_pattern);
                }
            }
        });

        // Circular ring canvas
        let ring_size = 120.0;
        let (ring_rect, response) = ui.allocate_exact_size(Vec2::splat(ring_size), Sense::click());
        if ui.is_rect_visible(ring_rect) {
            let painter = ui.painter_at(ring_rect);
            let center = ring_rect.center();
            let ring_r = ring_size * 0.38;
            let n = self.arp_ring_steps as usize;
            let pattern = self.arp_ring_pattern;
            let ring_pos = self.engine.arp_ring_pos() as usize;

            // Background guide circle
            painter.circle_stroke(
                center,
                ring_r,
                Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
            );

            for i in 0..n {
                let angle = i as f32 * TAU / n as f32 - FRAC_PI_2;
                let dot = Pos2::new(
                    center.x + ring_r * angle.cos(),
                    center.y + ring_r * angle.sin(),
                );
                let active = (pattern >> i) & 1 == 1;
                let is_head = self.arp_ring_enabled && ring_pos == i;

                if is_head {
                    painter.circle_filled(dot, 9.0, accent.gamma_multiply(0.25));
                }
                if active {
                    painter.circle_filled(
                        dot,
                        6.0,
                        if is_head {
                            accent
                        } else {
                            accent.gamma_multiply(0.75)
                        },
                    );
                } else {
                    painter.circle_stroke(
                        dot,
                        5.0,
                        Stroke::new(theme.stroke_ui, theme.c(&theme.text_disabled)),
                    );
                    if is_head {
                        painter.circle_filled(dot, 3.5, theme.c(&theme.text_secondary));
                    }
                }
            }

            // Click: toggle nearest step
            if response.clicked() {
                if let Some(click_pos) = response.interact_pointer_pos() {
                    let mut best = 0;
                    let mut best_d = f32::MAX;
                    for i in 0..n {
                        let angle = i as f32 * TAU / n as f32 - FRAC_PI_2;
                        let dot = Pos2::new(
                            center.x + ring_r * angle.cos(),
                            center.y + ring_r * angle.sin(),
                        );
                        let d = (dot - click_pos).length();
                        if d < best_d {
                            best_d = d;
                            best = i;
                        }
                    }
                    if best_d < 14.0 {
                        self.arp_ring_pattern ^= 1 << best;
                        self.engine.set_arp_ring_pattern(self.arp_ring_pattern);
                    }
                }
            }
        }
    }

    pub fn ui_walker_panel(&mut self, ui: &mut egui::Ui) {
        use forma_engine::arp::{ClockDiv, Scale};

        let theme = self.theme.clone();
        let enabled = self.engine.walker_enabled();

        // ── WALKER card ───────────────────────────────────────────────────────
        SynthFrame::section(&theme).show(ui, |ui| {
            ui.label(RichText::new("WALKER").font(theme.font_heading()).color(theme.c(&theme.text_primary)));
            ui.add_space(theme.sp_xs);

            // Transport row — WALKER toggle / RST
            ui.horizontal(|ui| {
                let label = RichText::new("WALKER").strong().color(if enabled {
                    theme.c(&theme.accent_walker)
                } else {
                    theme.c(&theme.text_disabled)
                });
                if ui.button(label)
                    .on_hover_text("Scale Walker — autonomous random walk within a scale. Generates notes independently of keyboard input.")
                    .clicked()
                {
                    let new_enabled = !enabled;
                    self.engine.set_walker_enabled(new_enabled);
                    if new_enabled && self.walker_sync_active() {
                        self.apply_clock_sync();
                        self.schedule_or_restart_walker();
                    }
                }
                if ui.button("RST").on_hover_text("Restart walker phase/index from beginning.").clicked() {
                    self.engine.walker_restart();
                }
            });

            // BPM + Sync
            ui.horizontal(|ui| {
                ui.label("BPM:");
                let sync_active = self.walker_sync_active();
                if sync_active {
                    self.engine.set_walker_bpm(self.seq.bpm.load(Ordering::Relaxed) as f32);
                }
                let mut bpm = self.engine.walker_bpm();
                ui.add_enabled_ui(!sync_active, |ui| {
                    if ui.add(egui::Slider::new(&mut bpm, 20.0..=300.0)).changed() {
                        self.engine.set_walker_bpm(bpm);
                    }
                });
                ui.add_enabled_ui(!self.global_sync, |ui| {
                    let sync_label = RichText::new("Sync").color(if self.walker_sync_active() {
                        theme.c(&theme.accent)
                    } else {
                        theme.c(&theme.text_disabled)
                    });
                    if ui.button(sync_label).on_hover_text("Lock Walker BPM to the Global BPM.").clicked() {
                        self.walker_sync = !self.walker_sync;
                        if self.walker_sync {
                            self.apply_clock_sync();
                            self.schedule_or_restart_walker();
                        } else {
                            self.seq.walker_restart.store(false, Ordering::Relaxed);
                        }
                    }
                });
            });

            ui.add_enabled_ui(enabled, |ui| {
                // Div
                let current_div = self.engine.walker_division();
                ui.horizontal(|ui| {
                    ui.label("Div:");
                    for (i, &label) in ClockDiv::LABELS.iter().enumerate() {
                        let active = current_div == i as u8;
                        let col = if active { theme.c(&theme.accent_dim) } else { theme.c(&theme.text_disabled) };
                        if ui.button(RichText::new(label).color(col)).clicked() && !active {
                            self.engine.set_walker_division(i as u8);
                        }
                    }
                });

                // Scale
                let current_scale = self.engine.walker_scale();
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    for (i, &label) in Scale::LABELS.iter().enumerate() {
                        let active = current_scale == i as u8;
                        let col = if active { theme.c(&theme.accent_dim) } else { theme.c(&theme.text_disabled) };
                        if ui.button(RichText::new(label).color(col)).clicked() && !active {
                            self.engine.set_walker_scale(i as u8);
                        }
                    }
                });

                // Root + Oct
                ui.horizontal(|ui| {
                    ui.label("Root:");
                    let mut root = self.engine.walker_root();
                    let note_names = ["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"];
                    let name = note_names[(root % 12) as usize];
                    let octave = (root as i32 / 12) - 1;
                    ui.label(RichText::new(format!("{}{}", name, octave)).color(theme.c(&theme.text_secondary)));
                    if ui.add(egui::Slider::new(&mut root, 36u8..=84)).changed() {
                        self.engine.set_walker_root(root);
                    }
                    ui.separator();
                    ui.label("Oct:");
                    let current_oct = self.engine.walker_octave_range();
                    for oct in 1u8..=3 {
                        let active = current_oct == oct;
                        let col = if active { theme.c(&theme.accent_dim) } else { theme.c(&theme.text_disabled) };
                        if ui.button(RichText::new(oct.to_string()).color(col)).clicked() && !active {
                            self.engine.set_walker_octave_range(oct);
                        }
                    }
                });

                // Gate
                ui.horizontal(|ui| {
                    ui.label("Gate:");
                    let mut gate = self.engine.walker_gate();
                    if ui.add(egui::Slider::new(&mut gate, 0.05..=1.0)).changed() {
                        self.engine.set_walker_gate(gate);
                    }
                });
            });
        });
    }
}
