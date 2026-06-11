use crate::midi_presets;
use crate::SynthApp;
use eframe::egui;
use egui::Color32;

impl SynthApp {
    pub fn ui_midi_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("MIDI").strong().small());

        // Refresh port list button
        if ui
            .small_button("⟳")
            .on_hover_text("Refresh MIDI device list")
            .clicked()
        {
            self.midi.list_ports();
        }

        if self.midi.port_names.is_empty() {
            ui.label(egui::RichText::new("No MIDI devices found").weak().small());
            return;
        }

        // Device selector
        let connected = self.midi.connected_port;
        let current_label = connected
            .and_then(|i| self.midi.port_names.get(i))
            .map(|s| s.as_str())
            .unwrap_or("— disconnected —");

        egui::ComboBox::from_id_salt("midi_port")
            .selected_text(egui::RichText::new(current_label).small())
            .show_ui(ui, |ui| {
                let selected = connected.is_none();
                if ui
                    .selectable_label(selected, "— disconnected —")
                    .on_hover_text("Disconnect from all MIDI devices.")
                    .clicked()
                {
                    self.midi.disconnect();
                }
                let names: Vec<String> = self.midi.port_names.clone();
                for (i, name) in names.iter().enumerate() {
                    let selected = connected == Some(i);
                    if ui
                        .selectable_label(selected, name)
                        .on_hover_text(format!("Connect to MIDI device: {name}"))
                        .clicked()
                        && !selected
                    {
                        match self.midi.connect(i) {
                            Ok(()) => {
                                let name = self.midi.port_names[i].clone();
                                self.midi_bindings =
                                    crate::midi_mapping_store::load_for_device(&name);
                            }
                            Err(e) => eprintln!("MIDI connect error: {e}"),
                        }
                    }
                }
            });

        // Status dot
        let (color, label) = if connected.is_some() {
            (self.theme.c(&self.theme.midi_connected), "●")
        } else {
            (Color32::from_gray(80), "○")
        };
        ui.label(egui::RichText::new(label).color(color).small());

        ui.separator();

        // ── Keyboard presets ─────────────────────────────────────────────
        let text_sec = self.theme.c(&self.theme.text_secondary);
        let text_dis = self.theme.c(&self.theme.text_disabled);
        let accent = self.theme.c(&self.theme.accent);

        ui.label(
            egui::RichText::new("Keyboard Presets")
                .small()
                .strong()
                .color(text_sec),
        );
        ui.label(
            egui::RichText::new(
                "Load a factory CC mapping for your keyboard, then fine-tune with MIDI Learn.",
            )
            .small()
            .color(text_dis),
        );
        ui.add_space(4.0);

        for preset in midi_presets::PRESETS {
            ui.horizontal(|ui| {
                let btn = egui::Button::new(egui::RichText::new(preset.name).small().color(accent));
                if ui.add(btn).on_hover_text(preset.description).clicked() {
                    self.midi_bindings = midi_presets::preset_bindings(preset);
                    self.save_active_bindings();
                }
            });
        }

        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Note: loading a preset replaces all current bindings.")
                .small()
                .italics()
                .color(text_dis),
        );

        ui.separator();

        // ── Layouts ───────────────────────────────────────────────────────
        ui.label(
            egui::RichText::new("Layouts")
                .small()
                .strong()
                .color(text_sec),
        );

        // Save current layout
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.layout_save_name)
                    .hint_text("Layout name…")
                    .desired_width(110.0)
                    .font(egui::TextStyle::Small),
            );
            let can_save = !self.layout_save_name.trim().is_empty();
            let save_btn = egui::Button::new(egui::RichText::new("Save").small());
            if ui
                .add_enabled(can_save, save_btn)
                .on_hover_text("Save current layout under this name.")
                .clicked()
            {
                let name = self.layout_save_name.trim().to_string();
                let state = self.capture_layout_state();
                crate::layout_store::save_named(&name, &state);
            }
        });

        ui.add_space(2.0);

        // Factory layouts
        ui.label(egui::RichText::new("Factory").font(self.theme.font_body()).color(text_dis));
        use crate::ui::layout::builtin_presets;
        for preset in builtin_presets() {
            ui.horizontal(|ui| {
                let btn = egui::Button::new(egui::RichText::new(preset.name).small().color(accent));
                if ui.add(btn).on_hover_text(preset.description).clicked() {
                    self.apply_panel_visibility(&preset.panels);
                    let state = self.capture_layout_state();
                    crate::ui::layout::save_layout(&state);
                }
            });
        }

        // User-saved layouts
        let saved = crate::layout_store::list_user_layouts();
        if !saved.is_empty() {
            ui.add_space(2.0);
            ui.label(egui::RichText::new("Saved").font(self.theme.font_body()).color(text_dis));
            for name in saved {
                ui.horizontal(|ui| {
                    let btn = egui::Button::new(egui::RichText::new(&name).small().color(accent));
                    if ui
                        .add(btn)
                        .on_hover_text(format!("Load layout \"{name}\""))
                        .clicked()
                    {
                        if let Some(state) = crate::layout_store::load_named(&name) {
                            self.apply_layout_state(&state);
                        }
                    }
                    let del =
                        egui::Button::new(egui::RichText::new("✕").font(self.theme.font_body()).color(text_dis));
                    if ui
                        .add(del)
                        .on_hover_text(format!("Delete layout \"{name}\""))
                        .clicked()
                    {
                        crate::layout_store::delete_named(&name);
                    }
                });
            }
        }

        ui.separator();

        // ── MIDI Monitor ─────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("MIDI Monitor")
                    .small()
                    .strong()
                    .color(text_sec),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Clear").clicked() {
                    self.midi_monitor.clear();
                }
            });
        });
        ui.label(
            egui::RichText::new("Press any button/knob on your keyboard to see its message here.")
                .small()
                .color(text_dis),
        );
        ui.add_space(2.0);

        let monitor_h = (ui.available_height() - 4.0).clamp(60.0, 200.0);
        egui::ScrollArea::vertical()
            .max_height(monitor_h)
            .show(ui, |ui| {
                if self.midi_monitor.is_empty() {
                    ui.label(
                        egui::RichText::new("— no messages yet —")
                            .small()
                            .color(text_dis),
                    );
                }
                for msg in &self.midi_monitor {
                    // Highlight CC and Program Change rows in accent color.
                    let col = if msg.starts_with("CC") || msg.starts_with("Prog") {
                        accent
                    } else {
                        text_sec
                    };
                    ui.label(egui::RichText::new(msg).monospace().font(self.theme.font_body()).color(col));
                }
            });
    }
}
