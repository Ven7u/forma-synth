use crate::audio::{DRUM_DEFAULT_NOISE_MIX, DRUM_STEP_COUNT};
use crate::ui::design::chip::{chip_selector, color_chip};
use crate::ui::design::drum_step::{drum_step, DrumStepState};
use crate::ui::design::slider::Slider as DesignSlider;
use crate::ui::design::toggle::{toggle_button, ToggleSize};
use crate::ui::design::Tier;
use crate::ui::frame::SynthFrame;
use crate::SynthApp;
use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json;

// ── Constants ────────────────────────────────────────────────────────────────

pub const STEP_COUNT: usize = DRUM_STEP_COUNT;
pub const CHANNEL_COUNT: usize = 8;

/// Pixel width of the channel-name column. Used to align the step-number header
/// and the voice-editor inset with the step grid.
const CHANNEL_LABEL_W: f32 = 70.0;

pub const CHANNEL_NAMES: [&str; CHANNEL_COUNT] = [
    "KICK", "SNARE", "HAT", "CLAP", "TOM1", "TOM2", "PERC", "NOISE",
];

// ── Drum machine state (UI-owned) ────────────────────────────────────────────

pub const PATTERN_COUNT: usize = 4;
pub const PATTERN_NAMES: [&str; PATTERN_COUNT] = ["A", "B", "C", "D"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumMachineState {
    pub enabled: bool,
    /// 4 independent step patterns; active one is `active_pattern`.
    pub patterns: [[[bool; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT],
    /// Per-step velocity (0–127) mirroring `patterns` layout.
    pub step_vel: [[[u8; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT],
    pub active_pattern: usize,
    #[serde(skip)]
    pub pattern_clipboard: Option<[[bool; STEP_COUNT]; CHANNEL_COUNT]>,
    pub muted: [bool; CHANNEL_COUNT],
    pub soloed: [bool; CHANNEL_COUNT],
    pub channel_volume: [f32; CHANNEL_COUNT],
    pub base_freq: [f32; CHANNEL_COUNT],   // Hz
    pub pitch_range: [f32; CHANNEL_COUNT], // Hz of sweep
    pub amp_decay: [f32; CHANNEL_COUNT],   // seconds
    pub noise_mix: [f32; CHANNEL_COUNT],   // 0.0–1.0
    // Euclidean generator per lane (applies to active pattern)
    pub euclid_on: [bool; CHANNEL_COUNT],
    pub euclid_hits: [u8; CHANNEL_COUNT],   // 1–16
    pub euclid_steps: [u8; CHANNEL_COUNT],  // 1–16
    pub euclid_offset: [u8; CHANNEL_COUNT], // 0–15
    /// Which channel's voice editor is currently expanded (None = collapsed all).
    #[serde(skip)]
    pub expanded_channel: Option<usize>,
    #[serde(skip)]
    pub current_step: usize, // playhead, driven by clock in future phases
    pub swing: f32, // 0.0–0.75
}

impl Default for DrumMachineState {
    fn default() -> Self {
        let mut patterns = [[[false; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT];
        // Seed a basic four-on-floor in slot A so the grid isn't empty on first open.
        let a = &mut patterns[0];
        a[0][0] = true;
        a[0][4] = true;
        a[0][8] = true;
        a[0][12] = true;
        a[1][4] = true;
        a[1][12] = true;
        for (i, slot) in a[2][..STEP_COUNT].iter_mut().enumerate() {
            if i % 2 == 0 {
                *slot = true;
            }
        }
        Self {
            enabled: false,
            patterns,
            step_vel: [[[100u8; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT],
            active_pattern: 0,
            pattern_clipboard: None,
            muted: [false; CHANNEL_COUNT],
            soloed: [false; CHANNEL_COUNT],
            channel_volume: [0.8; CHANNEL_COUNT],
            base_freq: [55.0, 180.0, 0.0, 0.0, 120.0, 75.0, 350.0, 0.0],
            pitch_range: [150.0, 0.0, 0.0, 0.0, 80.0, 60.0, 200.0, 0.0],
            amp_decay: [0.25, 0.12, 0.045, 0.09, 0.20, 0.26, 0.06, 0.35],
            noise_mix: DRUM_DEFAULT_NOISE_MIX,
            euclid_on: [false; CHANNEL_COUNT],
            euclid_hits: [4; CHANNEL_COUNT],
            euclid_steps: [16; CHANNEL_COUNT],
            euclid_offset: [0; CHANNEL_COUNT],
            expanded_channel: None,
            current_step: 0,
            swing: 0.0,
        }
    }
}

// ── Kit preset ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrumKit {
    pub name: String,
    pub channel_volume: [f32; CHANNEL_COUNT],
    pub base_freq: [f32; CHANNEL_COUNT],
    pub pitch_range: [f32; CHANNEL_COUNT],
    pub amp_decay: [f32; CHANNEL_COUNT],
    pub noise_mix: [f32; CHANNEL_COUNT],
    pub patterns: [[[bool; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT],
    pub step_vel: [[[u8; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT],
    pub swing: f32,
}

impl DrumKit {
    pub fn from_state(name: impl Into<String>, s: &DrumMachineState) -> Self {
        Self {
            name: name.into(),
            channel_volume: s.channel_volume,
            base_freq: s.base_freq,
            pitch_range: s.pitch_range,
            amp_decay: s.amp_decay,
            noise_mix: s.noise_mix,
            patterns: s.patterns,
            step_vel: s.step_vel,
            swing: s.swing,
        }
    }

    pub fn apply_to_state(&self, s: &mut DrumMachineState) {
        s.channel_volume = self.channel_volume;
        s.base_freq = self.base_freq;
        s.pitch_range = self.pitch_range;
        s.amp_decay = self.amp_decay;
        s.noise_mix = self.noise_mix;
        s.patterns = self.patterns;
        s.step_vel = self.step_vel;
        s.swing = self.swing;
        s.active_pattern = 0;
    }
}

/// A small set of factory kits bundled with the app.
pub fn factory_kits() -> Vec<DrumKit> {
    let empty = [[[false; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT];
    let flat_vel = [[[100u8; STEP_COUNT]; CHANNEL_COUNT]; PATTERN_COUNT];

    // ── Four-on-Floor ──────────────────────────────────────────────────────────
    let mut fof_pats = empty;
    let a = &mut fof_pats[0];
    a[0][0] = true;
    a[0][4] = true;
    a[0][8] = true;
    a[0][12] = true;
    a[1][4] = true;
    a[1][12] = true;
    for (i, slot) in a[2][..16].iter_mut().enumerate() {
        if i % 2 == 0 {
            *slot = true;
        }
    }
    let fof = DrumKit {
        name: "Four-on-Floor".into(),
        channel_volume: [0.9, 0.8, 0.6, 0.7, 0.75, 0.75, 0.7, 0.5],
        base_freq: [55.0, 180.0, 0.0, 0.0, 120.0, 75.0, 350.0, 0.0],
        pitch_range: [150.0, 0.0, 0.0, 0.0, 80.0, 60.0, 200.0, 0.0],
        amp_decay: [0.25, 0.12, 0.045, 0.09, 0.20, 0.26, 0.06, 0.35],
        noise_mix: DRUM_DEFAULT_NOISE_MIX,
        patterns: fof_pats,
        step_vel: flat_vel,
        swing: 0.0,
    };

    // ── Breakbeat ──────────────────────────────────────────────────────────────
    let mut bb_pats = empty;
    let a = &mut bb_pats[0];
    // kick
    a[0][0] = true;
    a[0][6] = true;
    a[0][9] = true;
    a[0][14] = true;
    // snare
    a[1][4] = true;
    a[1][12] = true;
    a[1][14] = true;
    // hat (all 16)
    for slot in a[2][..16].iter_mut() {
        *slot = true;
    }
    // clap on 4 & 12
    a[3][4] = true;
    a[3][12] = true;
    let bb = DrumKit {
        name: "Breakbeat".into(),
        channel_volume: [0.9, 0.8, 0.5, 0.7, 0.75, 0.75, 0.7, 0.5],
        base_freq: [55.0, 180.0, 0.0, 0.0, 120.0, 75.0, 350.0, 0.0],
        pitch_range: [150.0, 0.0, 0.0, 0.0, 80.0, 60.0, 200.0, 0.0],
        amp_decay: [0.22, 0.10, 0.035, 0.09, 0.20, 0.26, 0.06, 0.35],
        noise_mix: DRUM_DEFAULT_NOISE_MIX,
        patterns: bb_pats,
        step_vel: flat_vel,
        swing: 0.05,
    };

    // ── Minimal ────────────────────────────────────────────────────────────────
    let mut min_pats = empty;
    let a = &mut min_pats[0];
    a[0][0] = true;
    a[0][8] = true;
    a[1][4] = true;
    a[1][12] = true;
    a[2][2] = true;
    a[2][6] = true;
    a[2][10] = true;
    a[2][14] = true;
    let minimal = DrumKit {
        name: "Minimal".into(),
        channel_volume: [0.85, 0.75, 0.5, 0.6, 0.7, 0.7, 0.65, 0.4],
        base_freq: [50.0, 160.0, 0.0, 0.0, 110.0, 70.0, 320.0, 0.0],
        pitch_range: [130.0, 0.0, 0.0, 0.0, 70.0, 50.0, 180.0, 0.0],
        amp_decay: [0.30, 0.14, 0.05, 0.09, 0.22, 0.28, 0.07, 0.35],
        noise_mix: DRUM_DEFAULT_NOISE_MIX,
        patterns: min_pats,
        step_vel: flat_vel,
        swing: 0.0,
    };

    vec![fof, bb, minimal]
}

// ── Euclidean rhythm ─────────────────────────────────────────────────────────

/// Bresenham/Bjorklund: distribute `hits` evenly across `steps`, then rotate by `offset`.
fn euclidean_pattern(hits: usize, steps: usize, offset: usize) -> [bool; STEP_COUNT] {
    let mut out = [false; STEP_COUNT];
    if steps == 0 || hits == 0 {
        return out;
    }
    let hits = hits.min(steps).min(STEP_COUNT);
    let steps = steps.min(STEP_COUNT);
    for i in 0..steps {
        if (i * hits) % steps < hits {
            out[(i + offset) % steps] = true;
        }
    }
    out
}

// ── UI ───────────────────────────────────────────────────────────────────────

impl SynthApp {
    /// Renders the drum machine view inside an already-open central panel ui.
    pub fn ui_drum_machine(&mut self, ui: &mut egui::Ui) {
        // Apply euclidean generator for any lanes that have it enabled.
        let pat = self.drums.active_pattern;
        for ch in 0..CHANNEL_COUNT {
            if self.drums.euclid_on[ch] {
                let hits  = self.drums.euclid_hits[ch] as usize;
                let steps = self.drums.euclid_steps[ch] as usize;
                let off   = self.drums.euclid_offset[ch] as usize;
                self.drums.patterns[pat][ch] = euclidean_pattern(hits, steps, off);
            }
        }

        // Pre-resolve Copy-type token values so closures can capture them.
        let accent      = self.theme.c(&self.theme.accent);
        let text_sec    = self.theme.c(&self.theme.text_secondary);
        let text_dis    = self.theme.c(&self.theme.text_disabled);
        let accent_hold = self.theme.c(&self.theme.accent_hold);
        let seq_rec     = self.theme.c(&self.theme.seq_rec_cursor);
        let sp_xs       = self.theme.sp_xs;
        let sp_xxs      = self.theme.sp_xxs;

        ui.add_space(sp_xs);

        // ── Toolbar card ──────────────────────────────────────────────────
        SynthFrame::section(&self.theme).show(ui, |ui| {
            ui.horizontal(|ui| {
                // ON / OFF
                toggle_button(
                    ui, &mut self.drums.enabled, "DRUM",
                    ToggleSize::Standard, Tier::Secondary, &self.theme, None,
                );

                ui.separator();

                // Pattern selector A / B / C / D
                ui.label(egui::RichText::new("Pattern").small().color(text_sec));
                let pat_opts: &[(usize, &str)] = &[(0,"A"),(1,"B"),(2,"C"),(3,"D")];
                chip_selector(
                    ui, &mut self.drums.active_pattern, pat_opts, &self.theme, None,
                )
                .on_hover_text("Select active pattern");

                ui.separator();

                // Copy / Paste / Clear — plain buttons, theme handles color.
                if ui.button("Copy").on_hover_text("Copy active pattern").clicked() {
                    self.drums.pattern_clipboard =
                        Some(self.drums.patterns[self.drums.active_pattern]);
                }
                ui.add_enabled_ui(self.drums.pattern_clipboard.is_some(), |ui| {
                    if ui.button("Paste").on_hover_text("Paste into active pattern").clicked() {
                        if let Some(clip) = self.drums.pattern_clipboard {
                            self.drums.patterns[self.drums.active_pattern] = clip;
                        }
                    }
                });
                if ui.button("Clear").on_hover_text("Clear all steps").clicked() {
                    self.drums.patterns[self.drums.active_pattern] =
                        [[false; STEP_COUNT]; CHANNEL_COUNT];
                }

                ui.separator();

                // Static division label
                ui.label(egui::RichText::new("Div: 1/16").small().color(text_sec));

                ui.separator();

                // Swing — constrained slider so it fits inline.
                ui.scope(|ui| {
                    ui.set_max_width(160.0);
                    DesignSlider::new(&mut self.drums.swing, 0.0..=0.75, "Swing")
                        .decimals(2)
                        .show(ui, &self.theme);
                });

                ui.separator();

                // RST — disabled placeholder
                ui.add_enabled(
                    false,
                    egui::Button::new(egui::RichText::new("▶ RST").small()),
                )
                .on_hover_text("Playback and reset — coming soon");

                ui.separator();

                // KITS browser toggle
                toggle_button(
                    ui, &mut self.show_kit_browser, "KITS",
                    ToggleSize::Standard, Tier::Tertiary, &self.theme, None,
                )
                .on_hover_text("Open kit browser");
            });
        });

        ui.add_space(sp_xs);

        // ── Step-number header ────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(CHANNEL_LABEL_W);
            for step in 0..STEP_COUNT {
                let col = if step % 4 == 0 { accent } else { text_dis };
                ui.add_sized(
                    [28.0, 14.0],
                    egui::Label::new(
                        egui::RichText::new(format!("{}", step + 1))
                            .font(self.theme.font_body())
                            .color(col),
                    ),
                );
            }
        });

        ui.add_space(sp_xxs);

        // ── Channel rows ──────────────────────────────────────────────────
        let playhead = self.drums.current_step;

        for (ch, &ch_name) in CHANNEL_NAMES.iter().enumerate().take(CHANNEL_COUNT) {
            let muted      = self.drums.muted[ch];
            let soloed_any = self.drums.soloed.iter().any(|&s| s);
            let eff_muted  = muted || (soloed_any && !self.drums.soloed[ch]);
            let expanded   = self.drums.expanded_channel == Some(ch);

            ui.horizontal(|ui| {
                // ── Channel name toggle (fixed-width scope keeps step grid aligned) ──
                let mut is_expanded = expanded;
                ui.scope(|ui| {
                    ui.set_min_width(CHANNEL_LABEL_W);
                    ui.set_max_width(CHANNEL_LABEL_W);
                    let tint = if eff_muted {
                        Some(text_dis)
                    } else if self.drums.soloed[ch] {
                        Some(accent_hold)
                    } else {
                        None
                    };
                    toggle_button(
                        ui, &mut is_expanded, ch_name,
                        ToggleSize::Standard, Tier::Tertiary, &self.theme, tint,
                    )
                    .on_hover_text("Click to open voice editor");
                });
                if is_expanded != expanded {
                    self.drums.expanded_channel = if is_expanded { Some(ch) } else { None };
                }

                // ── 16 DrumStep cells ─────────────────────────────────────
                ui.spacing_mut().item_spacing.x = sp_xxs;
                for step in 0..STEP_COUNT {
                    let active   = self.drums.patterns[self.drums.active_pattern][ch][step];
                    let velocity = self.drums.step_vel[self.drums.active_pattern][ch][step] as f32
                        / 127.0;
                    let resp = drum_step(
                        ui,
                        DrumStepState {
                            active,
                            velocity,
                            is_playhead:   step == playhead && self.drums.enabled,
                            is_beat_group: step % 4 == 0,
                            is_muted:      eff_muted,
                        },
                        &self.theme,
                    );
                    let p = self.drums.active_pattern;
                    if resp.clicked() {
                        self.drums.patterns[p][ch][step] = !active;
                    }
                    if resp.dragged() && active {
                        let v = &mut self.drums.step_vel[p][ch][step];
                        *v = (*v as f32 - resp.drag_delta().y * 2.0).clamp(1.0, 127.0) as u8;
                    }
                }
                // Restore default spacing for the control buttons.
                ui.spacing_mut().item_spacing.x = sp_xs;

                // ── Per-lane controls ─────────────────────────────────────
                if color_chip(ui, "M", seq_rec, muted, &self.theme)
                    .on_hover_text("Mute this lane")
                    .clicked()
                {
                    self.drums.muted[ch] = !muted;
                }

                let soloed = self.drums.soloed[ch];
                if color_chip(ui, "S", accent_hold, soloed, &self.theme)
                    .on_hover_text("Solo — mutes all other lanes")
                    .clicked()
                {
                    self.drums.soloed[ch] = !soloed;
                }

                if ui.button("⇄").on_hover_text("Reverse this lane's pattern").clicked() {
                    let p = self.drums.active_pattern;
                    self.drums.patterns[p][ch].reverse();
                    self.drums.step_vel[p][ch].reverse();
                }

                if ui.button("?").on_hover_text("Randomize (50% density)").clicked() {
                    let p = self.drums.active_pattern;
                    // LCG seeded from time — avoids pulling the `rand` crate.
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.subsec_nanos())
                        .unwrap_or(12345)
                        .wrapping_add((ch as u32).wrapping_mul(2891336453u32));
                    let mut rng = seed;
                    for step in 0..STEP_COUNT {
                        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        self.drums.patterns[p][ch][step] = (rng >> 31) != 0;
                    }
                }

                let euclid_on = self.drums.euclid_on[ch];
                if color_chip(ui, "E", accent, euclid_on, &self.theme)
                    .on_hover_text("Toggle euclidean rhythm generator")
                    .clicked()
                {
                    self.drums.euclid_on[ch] = !euclid_on;
                }
            });

            // ── Voice editor (expands below channel row) ──────────────────
            if expanded {
                SynthFrame::inset(&self.theme)
                    .outer_margin(egui::Margin {
                        left:   CHANNEL_LABEL_W as i8,
                        right:  0,
                        top:    self.theme.sp_xxs as i8,
                        bottom: self.theme.sp_xs as i8,
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{ch_name} — Voice Editor"))
                                    .font(self.theme.font_body())
                                    .color(accent),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("✕").clicked() {
                                        self.drums.expanded_channel = None;
                                    }
                                },
                            );
                        });
                        ui.add_space(self.theme.sp_xs);

                        // Voice params — vertical Slider stack.
                        DesignSlider::new(
                            &mut self.drums.base_freq[ch], 0.0..=800.0, "Freq",
                        )
                        .suffix(" Hz").decimals(0).show(ui, &self.theme);

                        DesignSlider::new(
                            &mut self.drums.pitch_range[ch], 0.0..=500.0, "Sweep",
                        )
                        .suffix(" Hz").decimals(0).show(ui, &self.theme);

                        DesignSlider::new(
                            &mut self.drums.amp_decay[ch], 0.01..=2.0, "Decay",
                        )
                        .suffix(" s").decimals(3).show(ui, &self.theme);

                        DesignSlider::new(
                            &mut self.drums.noise_mix[ch], 0.0..=1.0, "Noise",
                        )
                        .decimals(2).show(ui, &self.theme);

                        DesignSlider::new(
                            &mut self.drums.channel_volume[ch], 0.0..=1.0, "Volume",
                        )
                        .decimals(2).show(ui, &self.theme);

                        // Euclidean section.
                        ui.add_space(self.theme.sp_xs);

                        toggle_button(
                            ui, &mut self.drums.euclid_on[ch], "Euclidean",
                            ToggleSize::Standard, Tier::Tertiary, &self.theme, None,
                        )
                        .on_hover_text("Toggle euclidean rhythm generator");

                        ui.add_enabled_ui(self.drums.euclid_on[ch], |ui| {
                            let steps_max = self.drums.euclid_steps[ch] as f32;
                            let mut hits = self.drums.euclid_hits[ch] as f32;
                            if DesignSlider::new(&mut hits, 1.0..=steps_max, "Hits")
                                .decimals(0)
                                .show(ui, &self.theme)
                                .changed()
                            {
                                self.drums.euclid_hits[ch] = hits as u8;
                            }

                            let mut steps = self.drums.euclid_steps[ch] as f32;
                            if DesignSlider::new(&mut steps, 1.0..=16.0, "Steps")
                                .decimals(0)
                                .show(ui, &self.theme)
                                .changed()
                            {
                                self.drums.euclid_steps[ch] = steps as u8;
                                self.drums.euclid_hits[ch] =
                                    self.drums.euclid_hits[ch].min(steps as u8);
                            }

                            let steps_max = self.drums.euclid_steps[ch] as f32;
                            let mut offset = self.drums.euclid_offset[ch] as f32;
                            if DesignSlider::new(
                                &mut offset,
                                0.0..=(steps_max - 1.0).max(0.0),
                                "Offset",
                            )
                            .decimals(0)
                            .show(ui, &self.theme)
                            .changed()
                            {
                                self.drums.euclid_offset[ch] = offset as u8;
                            }
                        });
                    });
            }
        }

        ui.add_space(sp_xs);
        ui.separator();
        ui.add_space(sp_xs);

        // ── Footer ────────────────────────────────────────────────────────
        ui.label(
            egui::RichText::new(format!(
                "♩ {} BPM  ·  16 steps  ·  Pattern {}",
                self.global_bpm, PATTERN_NAMES[self.drums.active_pattern]
            ))
            .font(self.theme.font_body())
            .color(text_dis),
        );

        let _ = (accent, text_sec, text_dis, accent_hold, seq_rec, sp_xs, sp_xxs);
    }

    /// Floating kit browser window — call every frame from `ui_drum_machine`.
    pub fn ui_kit_browser(&mut self, ctx: &egui::Context) {
        if !self.show_kit_browser {
            return;
        }
        let accent = self.theme.c(&self.theme.accent);
        let text_sec = self.theme.c(&self.theme.text_secondary);
        let text_dis = self.theme.c(&self.theme.text_disabled);
        let bg = self.theme.c(&self.theme.bg_surface);

        let mut open = self.show_kit_browser;
        egui::Window::new("Kit Library")
            .open(&mut open)
            .resizable(true)
            .default_size([280.0, 400.0])
            .show(ctx, |ui| {
                // ── Save current kit ──────────────────────────────────────
                ui.label(
                    egui::RichText::new("Save current kit")
                        .small()
                        .color(text_sec),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.drum_kit_name)
                            .desired_width(140.0)
                            .hint_text("Kit name"),
                    );
                    if ui.button("Save").on_hover_text("Add to library").clicked() {
                        let kit = DrumKit::from_state(&self.drum_kit_name, &self.drums);
                        self.drum_kit_library.push(kit);
                    }
                    if ui
                        .button("Export…")
                        .on_hover_text("Save kit to a JSON file")
                        .clicked()
                    {
                        let kit = DrumKit::from_state(&self.drum_kit_name, &self.drums);
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(format!("{}.drumkit.json", self.drum_kit_name))
                            .add_filter("Drum Kit", &["json"])
                            .save_file()
                        {
                            if let Ok(json) = serde_json::to_string_pretty(&kit) {
                                let _ = std::fs::write(path, json);
                            }
                        }
                    }
                });

                ui.separator();

                // ── Import from file ──────────────────────────────────────
                if ui
                    .button("Import kit from file…")
                    .on_hover_text("Load a .drumkit.json file")
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Drum Kit", &["json"])
                        .pick_file()
                    {
                        if let Ok(data) = std::fs::read_to_string(&path) {
                            if let Ok(kit) = serde_json::from_str::<DrumKit>(&data) {
                                self.drum_kit_library.push(kit);
                            }
                        }
                    }
                }

                ui.separator();
                ui.label(egui::RichText::new("Library").small().color(text_sec));
                ui.add_space(self.theme.sp_xxs);

                // ── Kit list ──────────────────────────────────────────────
                let avail = ui.available_height();
                egui::ScrollArea::vertical()
                    .max_height(avail)
                    .show(ui, |ui| {
                        let mut to_delete: Option<usize> = None;
                        for (idx, kit) in self.drum_kit_library.iter().enumerate() {
                            let name = kit.name.clone();
                            ui.horizontal(|ui| {
                                let resp = ui.add_sized(
                                    [180.0, 22.0],
                                    egui::Button::new(egui::RichText::new(&name).color(accent))
                                        .fill(bg),
                                );
                                if resp.on_hover_text("Click to load this kit").clicked() {
                                    kit.apply_to_state(&mut self.drums);
                                    self.drum_kit_name = name.clone();
                                }
                                if ui
                                    .small_button(egui::RichText::new("✕").color(text_dis))
                                    .on_hover_text("Remove from library")
                                    .clicked()
                                {
                                    to_delete = Some(idx);
                                }
                            });
                        }
                        if let Some(i) = to_delete {
                            self.drum_kit_library.remove(i);
                        }
                    });
            });
        self.show_kit_browser = open;
    }
}
