use crate::ui::design::{layout::fx_module, slider::Slider};
use crate::SynthApp;
use eframe::egui;

/// Phaser stage options shown in the UI.
const PHASER_STAGE_OPTIONS: &[(usize, &str)] = &[(4, "4"), (6, "6"), (8, "8")];

/// Delay note divisions: (label, beats relative to a quarter-note pulse).
pub const DELAY_DIVISIONS: &[(&str, f32)] = &[
    ("1/1", 4.0),
    ("1/2", 2.0),
    ("1/4", 1.0),
    ("1/8", 0.5),
    ("1/16", 0.25),
    ("3/8", 1.5),   // dotted quarter
    ("3/16", 0.75), // dotted eighth
];

impl SynthApp {
    pub fn ui_fx_chain(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let col_od = theme.c(&theme.fx_overdrive);
        let col_dist = theme.c(&theme.fx_distortion);
        let col_cho = theme.c(&theme.fx_chorus);
        let col_dly = theme.c(&theme.fx_delay);
        let col_rev = theme.c(&theme.fx_reverb);
        let col_crys = theme.c(&theme.fx_crystallizer);

        // Data-driven width: the FX chain holds ~11 modules at ≥110 px each,
        // so it overflows the minimum window width of 720 px by design.
        // Scroll horizontally rather than crush modules below their minimum.
        egui::ScrollArea::horizontal()
            .id_salt("fx_chain_h_scroll")
            .show(ui, |ui| {
        ui.horizontal(|ui| {
            // ---- Overdrive ----
            let mut on = self.fx_overdrive_on;
            let (resp, _) = fx_module(ui, "OVERDRIVE", col_od, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_overdrive_drive, 1.0_f32..=10.0, "Drive")
                    .decimals(1)
                    .show(ui, &theme)
                    .on_hover_text("Drive — how hard the signal is pushed into tanh saturation.");
                Slider::new(&mut self.fx_overdrive_tone, 0.0_f32..=1.0, "Tone")
                    .show(ui, &theme)
                    .on_hover_text("Tone — post-clipper low-pass: 0 = dark (400 Hz), 1 = bright (18 kHz).");
                Slider::new(&mut self.fx_overdrive_asym, 0.0_f32..=1.0, "Asym")
                    .show(ui, &theme)
                    .on_hover_text("Asymmetry — DC bias before clipping adds even harmonics for a warmer, tube-like character.");
                Slider::new(&mut self.fx_overdrive_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix: 0 = dry, 1 = fully overdriven.");
                self.engine.set_fx_overdrive_drive(self.fx_overdrive_drive);
                self.engine.set_fx_overdrive_tone(self.fx_overdrive_tone);
                self.engine.set_fx_overdrive_asym(self.fx_overdrive_asym);
                if self.fx_overdrive_on {
                    self.engine.set_fx_overdrive_mix(self.fx_overdrive_mix);
                }
            });
            if resp.clicked() {
                self.fx_overdrive_on = on;
                self.engine.set_fx_overdrive_mix(if on { self.fx_overdrive_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle overdrive (soft-clip / tanh saturation).");

            // ---- Distortion ----
            let mut on = self.fx_distortion_on;
            let (resp, _) = fx_module(ui, "DISTORTION", col_dist, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_distortion_drive, 1.0_f32..=20.0, "Drive")
                    .decimals(1)
                    .show(ui, &theme)
                    .on_hover_text("Drive — pre-gain before hard clipping. Higher = more of the wave is squared off.");
                Slider::new(&mut self.fx_distortion_pre, 0.0_f32..=1.0, "Pre")
                    .show(ui, &theme)
                    .on_hover_text("Pre — high-pass before clipper (0 = all bass in, 1 = 800 Hz cut). Removes mud from low-end distortion.");
                Slider::new(&mut self.fx_distortion_tone, 0.0_f32..=1.0, "Tone")
                    .show(ui, &theme)
                    .on_hover_text("Tone — post-clipper low-pass: 0 = dark (400 Hz), 1 = bright (18 kHz). Rolls off harsh high harmonics.");
                Slider::new(&mut self.fx_distortion_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix: 0 = dry, 1 = fully distorted.");
                self.engine.set_fx_distortion_drive(self.fx_distortion_drive);
                self.engine.set_fx_distortion_pre(self.fx_distortion_pre);
                self.engine.set_fx_distortion_tone(self.fx_distortion_tone);
                if self.fx_distortion_on {
                    self.engine.set_fx_distortion_mix(self.fx_distortion_mix);
                }
            });
            if resp.clicked() {
                self.fx_distortion_on = on;
                self.engine.set_fx_distortion_mix(if on { self.fx_distortion_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle distortion (hard clipping).");

            // ---- Chorus ----
            let mut on = self.fx_chorus_on;
            let (resp, _) = fx_module(ui, "CHORUS", col_cho, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_chorus_rate, 0.1_f32..=5.0, "Rate")
                    .suffix(" Hz")
                    .decimals(2)
                    .show(ui, &theme)
                    .on_hover_text("LFO rate in Hz — how fast the chorus modulates.");
                Slider::new(&mut self.fx_chorus_depth, 0.0_f32..=0.02, "Depth")
                    .formatter(|v| format!("{:.1} ms", v * 1000.0))
                    .show(ui, &theme)
                    .on_hover_text("Depth of LFO modulation in seconds (0–20 ms).");
                Slider::new(&mut self.fx_chorus_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix.");
                self.engine.set_fx_chorus_rate(self.fx_chorus_rate);
                self.engine.set_fx_chorus_depth(self.fx_chorus_depth);
                if self.fx_chorus_on {
                    self.engine.set_fx_chorus_mix(self.fx_chorus_mix);
                }
            });
            if resp.clicked() {
                self.fx_chorus_on = on;
                self.engine.set_fx_chorus_mix(if on { self.fx_chorus_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle chorus (LFO-modulated delay for width/shimmer).");

            // ---- Delay ----
            let mut on = self.fx_delay_on;
            let (resp, _) = fx_module(ui, "DELAY", col_dly, &mut on, &theme, |ui| {
                ui.add_enabled_ui(!self.global_sync, |ui| {
                    let delay_sync_on = self.delay_sync_active();
                    let sync_label = egui::RichText::new("BPM Sync")
                        .color(if delay_sync_on { col_dly } else { theme.c(&theme.text_secondary) });
                    if ui.button(sync_label).on_hover_text("Sync delay time to the Global BPM.").clicked() {
                        self.fx_delay_sync = !self.fx_delay_sync;
                    }
                });

                if self.delay_sync_active() {
                    let bpm = self.global_bpm as f32;
                    let beat_sec = 60.0 / bpm;
                    ui.horizontal_wrapped(|ui| {
                        for (i, (name, _)) in DELAY_DIVISIONS.iter().enumerate() {
                            let active = self.fx_delay_division == i;
                            let btn_label = egui::RichText::new(*name).small()
                                .color(if active { col_dly } else { theme.c(&theme.text_secondary) });
                            if ui.button(btn_label).on_hover_text(format!("Set delay to {} note ({:.0} BPM → {:.3}s)", name, bpm, beat_sec * DELAY_DIVISIONS[i].1)).clicked() {
                                self.fx_delay_division = i;
                            }
                        }
                    });
                    self.fx_delay_division = self.fx_delay_division.min(DELAY_DIVISIONS.len() - 1);
                    let synced_time = (beat_sec * DELAY_DIVISIONS[self.fx_delay_division].1).clamp(0.01, 1.0);
                    self.fx_delay_time = synced_time;
                    ui.label(egui::RichText::new(format!("{:.3} s  @{}BPM", synced_time, self.global_bpm)).small().color(theme.c(&theme.text_disabled)))
                        .on_hover_text("Current delay time computed from Global BPM and selected note division.");
                } else {
                    Slider::new(&mut self.fx_delay_time, 0.01_f32..=1.0, "Time")
                        .suffix(" s")
                        .decimals(2)
                        .show(ui, &theme)
                        .on_hover_text("Delay time in seconds (10 ms – 1 s).");
                }

                Slider::new(&mut self.fx_delay_feedback, 0.0_f32..=0.95, "Feedback")
                    .show(ui, &theme)
                    .on_hover_text("Feedback amount — how much of the delayed signal repeats.");
                Slider::new(&mut self.fx_delay_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix.");
                self.engine.set_fx_delay_time(self.fx_delay_time);
                self.engine.set_fx_delay_feedback(self.fx_delay_feedback);
                if self.fx_delay_on {
                    self.engine.set_fx_delay_mix(self.fx_delay_mix);
                }
            });
            if resp.clicked() {
                self.fx_delay_on = on;
                if on {
                    self.engine.reset_fx_tails();
                }
                self.engine.set_fx_delay_mix(if on { self.fx_delay_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle delay (echo effect with feedback).");

            // ---- Reverb ----
            let mut on = self.fx_reverb_on;
            let (resp, _) = fx_module(ui, "REVERB", col_rev, &mut on, &theme, |ui| {
                ui.horizontal(|ui| {
                    for (i, name) in ["Free", "Plate", "Hall"].iter().enumerate() {
                        let selected = self.fx_reverb_type == i as u8;
                        let label = egui::RichText::new(*name).small()
                            .color(if selected { col_rev } else { theme.c(&theme.text_secondary) });
                        if ui.selectable_label(selected, label).clicked() {
                            self.fx_reverb_type = i as u8;
                            self.engine.set_fx_reverb_type(i as u8);
                        }
                    }
                });
                Slider::new(&mut self.fx_reverb_predelay, 0.0_f32..=0.1, "Pre")
                    .formatter(|v| format!("{:.0} ms", v * 1000.0))
                    .show(ui, &theme)
                    .on_hover_text("Pre-delay: silence before the reverb tail starts. 20–80 ms separates the dry note from the wash, giving cinematic depth.");
                Slider::new(&mut self.fx_reverb_size, 0.0_f32..=1.0, "Size")
                    .show(ui, &theme)
                    .on_hover_text("Room size — controls reverb decay time.");
                Slider::new(&mut self.fx_reverb_damp, 0.0_f32..=1.0, "Damp")
                    .show(ui, &theme)
                    .on_hover_text("High-frequency damping — 0 = bright, 1 = dark/muffled.");
                Slider::new(&mut self.fx_reverb_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix.");
                self.engine.set_fx_reverb_predelay(self.fx_reverb_predelay);
                self.engine.set_fx_reverb_size(self.fx_reverb_size);
                self.engine.set_fx_reverb_damp(self.fx_reverb_damp);
                if self.fx_reverb_on {
                    self.engine.set_fx_reverb_mix(self.fx_reverb_mix);
                }
            });
            if resp.clicked() {
                self.fx_reverb_on = on;
                if on {
                    self.engine.reset_fx_tails();
                }
                self.engine.set_fx_reverb_mix(if on { self.fx_reverb_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle reverb.");

            // ---- Shimmer ----
            let col_shim = theme.c(&theme.fx_shimmer);
            let mut on = self.fx_shimmer_on;
            let (resp, _) = fx_module(ui, "SHIMMER", col_shim, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_shimmer_size, 0.0_f32..=1.0, "Size")
                    .show(ui, &theme)
                    .on_hover_text("Shimmer reverb room size.");
                Slider::new(&mut self.fx_shimmer_damp, 0.0_f32..=1.0, "Damp")
                    .show(ui, &theme)
                    .on_hover_text("Shimmer high-frequency damping.");
                Slider::new(&mut self.fx_shimmer_amt, 0.0_f32..=1.0, "Shimmer")
                    .show(ui, &theme)
                    .on_hover_text("Amount of pitch-shifted signal fed back into the reverb loop.");
                Slider::new(&mut self.fx_shimmer_width, 0.5_f32..=2.0, "Width")
                    .show(ui, &theme)
                    .on_hover_text("Stereo width of the wet reverb/shimmer field. 1.0 = neutral.");
                Slider::new(&mut self.fx_shimmer_spread, 0.0_f32..=0.3, "Spread")
                    .show(ui, &theme)
                    .on_hover_text("Left/right decorrelation depth for reverb and shimmer tails.");
                Slider::new(&mut self.fx_shimmer_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Shimmer wet level.");
                ui.horizontal(|ui| {
                    ui.label("Pitch:");
                    for (i, lbl) in ["0", "+12", "+24"].iter().enumerate() {
                        if ui.selectable_label(self.fx_shimmer_pitch == i as u8, *lbl).clicked() {
                            self.fx_shimmer_pitch = i as u8;
                            self.engine.set_shimmer_pitch(i as u8);
                        }
                    }
                });
                self.engine.set_shimmer_size(self.fx_shimmer_size);
                self.engine.set_shimmer_damp(self.fx_shimmer_damp);
                self.engine.set_shimmer_amount(if self.fx_shimmer_on { self.fx_shimmer_amt } else { 0.0 });
                self.engine.set_shimmer_width(self.fx_shimmer_width);
                self.engine.set_shimmer_spread(self.fx_shimmer_spread);
                self.engine.set_shimmer_mix(if self.fx_shimmer_on { self.fx_shimmer_mix } else { 0.0 });
            });
            if resp.clicked() {
                self.fx_shimmer_on = on;
                if on {
                    self.engine.reset_fx_tails();
                }
                self.engine.set_shimmer_mix(if on { self.fx_shimmer_mix } else { 0.0 });
            }
            resp.on_hover_text("Shimmer reverb — pitch-shifted feedback loop creates a rising harmonic halo.");

            // ---- Crystallizer ----
            let mut on = self.fx_crystal_on;
            let (resp, _) = fx_module(ui, "CRYSTAL", col_crys, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_crystal_grain_ms, 10.0_f32..=400.0, "Grain")
                    .suffix(" ms")
                    .decimals(0)
                    .show(ui, &theme)
                    .on_hover_text("Grain size in milliseconds.");
                Slider::new(&mut self.fx_crystal_scatter, 0.0_f32..=1.0, "Scatter")
                    .show(ui, &theme)
                    .on_hover_text("Random grain position offset.");
                Slider::new(&mut self.fx_crystal_delay_ms, 20.0_f32..=1200.0, "Delay")
                    .suffix(" ms")
                    .decimals(0)
                    .show(ui, &theme)
                    .on_hover_text("Base delay time.");
                Slider::new(&mut self.fx_crystal_feedback, 0.0_f32..=0.95, "Feedback")
                    .show(ui, &theme)
                    .on_hover_text("Feedback amount.");
                Slider::new(&mut self.fx_crystal_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Crystallizer wet level.");
                ui.horizontal(|ui| {
                    ui.label("Pitch:");
                    for (i, lbl) in ["0.5x", "1x", "2x", "4x"].iter().enumerate() {
                        if ui.selectable_label(self.fx_crystal_pitch == i as u8, *lbl).clicked() {
                            self.fx_crystal_pitch = i as u8;
                            self.engine.set_crystal_pitch(i as u8);
                        }
                    }
                });
                self.engine.set_crystal_grain(self.fx_crystal_grain_ms);
                self.engine.set_crystal_scatter(self.fx_crystal_scatter);
                self.engine.set_crystal_delay(self.fx_crystal_delay_ms);
                self.engine.set_crystal_feedback(self.fx_crystal_feedback);
                self.engine.set_crystal_mix(if self.fx_crystal_on { self.fx_crystal_mix } else { 0.0 });
            });
            if resp.clicked() {
                self.fx_crystal_on = on;
                self.engine.set_crystal_mix(if on { self.fx_crystal_mix } else { 0.0 });
            }
            resp.on_hover_text("Crystallizer — granular pitch-shift delay with feedback.");

            // ---- Bit Crusher ----
            let col_bc = theme.c(&theme.fx_distortion);
            let mut on = self.fx_bitcrush_on;
            let (resp, _) = fx_module(ui, "BIT CRUSH", col_bc, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_bitcrush_bits, 1.0_f32..=16.0, "Bits")
                    .decimals(1)
                    .show(ui, &theme)
                    .on_hover_text("Bit depth: 16 = CD quality, 8 = classic lo-fi, 4 = extreme crunch, 1 = 1-bit noise.");
                Slider::new(&mut self.fx_bitcrush_rate, 1.0_f32..=32.0, "S/R Div")
                    .formatter(|v| format!("÷{:.0}", v))
                    .show(ui, &theme)
                    .on_hover_text("Sample-rate divisor: 1 = no decimation, 32 = extreme aliasing crunch.");
                Slider::new(&mut self.fx_bitcrush_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix.");
                self.engine.set_fx_bitcrush_bits(self.fx_bitcrush_bits);
                self.engine.set_fx_bitcrush_rate(self.fx_bitcrush_rate);
                if self.fx_bitcrush_on {
                    self.engine.set_fx_bitcrush_mix(self.fx_bitcrush_mix);
                }
            });
            if resp.clicked() {
                self.fx_bitcrush_on = on;
                self.engine.set_fx_bitcrush_mix(if on { self.fx_bitcrush_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle bit crusher / sample-rate reducer.");

            // ---- Tape Saturation ----
            let col_tape = theme.c(&theme.fx_overdrive);
            let mut on = self.fx_tape_on;
            let (resp, _) = fx_module(ui, "TAPE SAT", col_tape, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_tape_drive, 0.0_f32..=1.0, "Drive")
                    .show(ui, &theme)
                    .on_hover_text("How hard the tape head is driven. Higher = more saturation and harmonic content.");
                Slider::new(&mut self.fx_tape_tone, 0.0_f32..=1.0, "Tone")
                    .show(ui, &theme)
                    .on_hover_text("Post-saturation bandwidth: 0 = vintage dark (2 kHz rolloff), 1 = modern full bandwidth.");
                Slider::new(&mut self.fx_tape_bias, 0.0_f32..=1.0, "Bias")
                    .show(ui, &theme)
                    .on_hover_text("Tape bias: adds even harmonics (2nd harmonic content) for a warmer, slightly asymmetric character.");
                Slider::new(&mut self.fx_tape_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix.");
                self.engine.set_fx_tape_drive(self.fx_tape_drive);
                self.engine.set_fx_tape_tone(self.fx_tape_tone);
                self.engine.set_fx_tape_bias(self.fx_tape_bias);
                if self.fx_tape_on {
                    self.engine.set_fx_tape_mix(self.fx_tape_mix);
                }
            });
            if resp.clicked() {
                self.fx_tape_on = on;
                self.engine.set_fx_tape_mix(if on { self.fx_tape_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle tape saturation (warm harmonic saturation with head-bump warmth).");

            // ---- Phaser ----
            let col_ph = theme.c(&theme.fx_chorus);
            let mut on = self.fx_phaser_on;
            let (resp, _) = fx_module(ui, "PHASER", col_ph, &mut on, &theme, |ui| {
                Slider::new(&mut self.fx_phaser_rate, 0.05_f32..=10.0, "Rate")
                    .suffix(" Hz")
                    .decimals(2)
                    .show(ui, &theme)
                    .on_hover_text("LFO rate in Hz — how fast the notch sweep moves.");
                Slider::new(&mut self.fx_phaser_depth, 0.0_f32..=1.0, "Depth")
                    .show(ui, &theme)
                    .on_hover_text("LFO modulation depth — how wide the notch sweeps.");
                Slider::new(&mut self.fx_phaser_center, 100.0_f32..=8000.0, "Center")
                    .suffix(" Hz")
                    .logarithmic(true)
                    .decimals(0)
                    .show(ui, &theme)
                    .on_hover_text("Center frequency of the all-pass notch sweep.");
                Slider::new(&mut self.fx_phaser_feedback, -0.9_f32..=0.9, "Feedback")
                    .show(ui, &theme)
                    .on_hover_text("Feedback amount. Positive = bright resonance, negative = darker hollow character.");
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Stages:").small());
                    for &(n, lbl) in PHASER_STAGE_OPTIONS {
                        if ui.selectable_label(self.fx_phaser_stages == n, lbl).clicked() {
                            self.fx_phaser_stages = n;
                            self.engine.set_fx_phaser_stages(n as u8);
                        }
                    }
                });
                Slider::new(&mut self.fx_phaser_mix, 0.0_f32..=1.0, "Mix")
                    .show(ui, &theme)
                    .on_hover_text("Wet/dry mix.");
                self.engine.set_fx_phaser_rate(self.fx_phaser_rate);
                self.engine.set_fx_phaser_depth(self.fx_phaser_depth);
                self.engine.set_fx_phaser_center(self.fx_phaser_center);
                self.engine.set_fx_phaser_feedback(self.fx_phaser_feedback);
                if self.fx_phaser_on {
                    self.engine.set_fx_phaser_mix(self.fx_phaser_mix);
                }
            });
            if resp.clicked() {
                self.fx_phaser_on = on;
                self.engine.set_fx_phaser_mix(if on { self.fx_phaser_mix } else { 0.0 });
            }
            resp.on_hover_text("Toggle phaser (all-pass filter chain with stereo-decorrelated LFO).");

            // ---- Stereo Width ----
            // Stereo has no on/off toggle; render manually with a header label.
            ui.group(|ui| {
                ui.set_min_width(120.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("STEREO").small().strong()
                        .color(theme.c(&theme.accent)));
                    Slider::new(&mut self.stereo_spread, 0.0_f32..=0.012, "Spread")
                        .formatter(|v| format!("{:.1} ms", v * 1000.0))
                        .show(ui, &theme)
                        .on_hover_text("Haas spread: delays R channel by 0–12 ms. Creates stereo width from mono unison voices. Keep under 10 ms to avoid comb filtering.");
                    Slider::new(&mut self.stereo_width, 0.0_f32..=2.0, "Width")
                        .show(ui, &theme)
                        .on_hover_text("M/S width on the final output. 0 = mono, 1 = unchanged, 2 = maximum stereo expansion.");
                    self.engine.set_stereo_spread(self.stereo_spread);
                    self.engine.set_stereo_width(self.stereo_width);
                });
            });
        });
            });
    }
}
