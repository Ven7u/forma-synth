//! Design System Gallery — categorized Storybook-style viewer.
//!
//! A debug window that renders every token, component, and pattern in the
//! design system against the live theme and zoom factor. Categories live
//! in a left sidebar; the right side shows samples for the active category.
//!
//! Toggle with Ctrl/Cmd + Shift + G.

use egui::{Color32, Context, RichText, Stroke, Ui, Vec2, Window};

use super::{
    chip::{chip_selector as design_chip, color_chip as design_color_chip},
    chord_pad::{chord_pad as design_chord_pad, ChordPadState, ChordQuality},
    drum_step::{drum_step as design_drum_step, DrumStepState},
    fader::{fader as design_fader, FaderOrientation, FaderSize},
    knob::knob as design_knob,
    layout::fader_column as design_fader_column,
    level_meter::{level_meter as design_level_meter, LevelMeterOrientation, LevelMeterSize},
    mini_bar::{MiniBar, MiniBarOrientation},
    piano::{piano as design_piano, KeyVisualState, PianoConfig, PianoSize},
    section::section_header as design_section_header,
    slider::Slider as DesignSlider,
    step_pad::{step_pad as design_step_pad, StepPadSize, StepState},
    toggle::{toggle_button as design_toggle, ToggleSize},
    KnobSize, SynthUi, Tier,
};
use crate::ui::frame::SynthFrame;
use crate::ui::theme::SynthTheme;

/// Top-level navigation categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Category {
    Tokens,
    Components,
    Patterns,
    Layouts,
    Frames,
    Examples,
}

impl Category {
    const ALL: &'static [Category] = &[
        Category::Tokens,
        Category::Components,
        Category::Patterns,
        Category::Layouts,
        Category::Frames,
        Category::Examples,
    ];

    fn label(self) -> &'static str {
        match self {
            Category::Tokens => "Tokens",
            Category::Components => "Components",
            Category::Patterns => "Patterns",
            Category::Layouts => "Layouts",
            Category::Frames => "Frames",
            Category::Examples => "Examples",
        }
    }
}

/// Persistent demo state.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct GalleryState {
    category: Category,
    knob_values: [[f32; 3]; 3],
    legacy_value: f32,
    toggle_states: [bool; 3],
    chip_choice: usize,
    section_header_toggle: bool,
    /// Per-section-card toggle so the three card variants are independent.
    card_toggles: [bool; 3],
    /// Example OSC card state (waveform, detune, PW, unison on).
    example_wave: usize,
    example_detune: f32,
    example_pw: f32,
    example_uni: bool,
    /// 3 sizes of vertical fader sample, plus 1 horizontal.
    fader_values: [f32; 3],
    fader_horizontal: f32,
    /// FaderColumn pattern demo — 4 channel volumes.
    column_values: [f32; 4],
    /// 4 Slider demos: linear / suffix / logarithmic / formatter.
    slider_values: [f32; 4],
    /// MiniBar samples — velocity, probability, pitch index.
    mini_bar_velocity: f32,
    mini_bar_probability: f32,
    mini_bar_pitch: f32,
    mini_bar_pitch_accum: f32,
    /// ColorChip demo — 8 EQ-band-style toggles.
    color_chip_active: [bool; 8],
    /// DrumStep demo — 16 interactive cells.
    drum_step_active: [bool; 16],
    drum_step_vel: [f32; 16],
    /// ChordPad demo — held state for the 3 interactive pads (one per quality).
    chord_pad_held: [bool; 3],
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            category: Category::Components,
            knob_values: [[0.3; 3]; 3],
            legacy_value: 0.3,
            toggle_states: [false, true, false],
            chip_choice: 1,
            section_header_toggle: true,
            card_toggles: [true, false, false],
            example_wave: 1,
            example_detune: 12.0,
            example_pw: 0.5,
            example_uni: false,
            fader_values: [0.8, 0.5, 0.2],
            fader_horizontal: 0.6,
            column_values: [0.75, 0.55, 0.3, 0.4],
            slider_values: [0.5, 440.0, 2000.0, 0.4],
            mini_bar_velocity: 95.0,
            mini_bar_probability: 70.0,
            mini_bar_pitch: 60.0,
            mini_bar_pitch_accum: 0.0,
            color_chip_active: [true, false, true, true, false, true, false, true],
            drum_step_active: [
                true, false, false, false, true, false, false, false, true, false, false, false,
                true, false, false, false,
            ],
            drum_step_vel: [0.9; 16],
            chord_pad_held: [false; 3],
        }
    }
}

const STATE_ID: &str = "forma_design_gallery_state";

fn load_state(ctx: &Context) -> GalleryState {
    ctx.data_mut(|d| {
        d.get_temp::<GalleryState>(egui::Id::new(STATE_ID))
            .unwrap_or_default()
    })
}

fn save_state(ctx: &Context, s: GalleryState) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(STATE_ID), s));
}

pub fn show(ctx: &Context, open: &mut bool, theme: &SynthTheme) {
    if !*open {
        return;
    }
    let mut state = load_state(ctx);

    Window::new("Design System Gallery")
        .open(open)
        .default_size([900.0, 640.0])
        .resizable(true)
        .show(ctx, |ui| {
            // Claim the full available size so the inner horizontal layout
            // can grow with the window — without this the row sizes to its
            // tallest child (the sidebar) and the window can't be enlarged
            // vertically beyond the sidebar's natural height.
            let avail = ui.available_size_before_wrap();
            ui.allocate_ui_with_layout(
                avail,
                egui::Layout::left_to_right(egui::Align::TOP),
                |ui| {
                    sidebar(ui, theme, &mut state);
                    ui.separator();
                    ui.vertical(|ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| match state.category {
                                Category::Tokens => render_tokens(ui, theme),
                                Category::Components => render_components(ui, theme, &mut state),
                                Category::Patterns => render_patterns(ui, theme, &mut state),
                                Category::Layouts => render_layouts(ui, theme),
                                Category::Frames => render_frames(ui, theme),
                                Category::Examples => render_examples(ui, theme, &mut state),
                            });
                    });
                },
            );
        });

    save_state(ctx, state);
}

fn sidebar(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    ui.vertical(|ui| {
        ui.set_min_width(140.0);
        ui.set_max_width(160.0);
        ui.add_space(theme.sp_sm);
        ui.label(
            RichText::new("Categories")
                .font(theme.font_small())
                .color(theme.c(&theme.text_secondary)),
        );
        ui.add_space(theme.sp_xs);
        for cat in Category::ALL {
            let active = state.category == *cat;
            let label = RichText::new(cat.label())
                .font(theme.font_body())
                .color(if active {
                    theme.c(&theme.text_primary)
                } else {
                    theme.c(&theme.text_secondary)
                });
            if ui
                .add_sized([130.0, 26.0], egui::SelectableLabel::new(active, label))
                .clicked()
            {
                state.category = *cat;
            }
        }
    });
}

// ─── Category renderers ─────────────────────────────────────────────────────

fn render_tokens(ui: &mut Ui, theme: &SynthTheme) {
    sub_header(ui, "Typography", theme);
    font_samples(ui, theme);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Surfaces", theme);
    color_swatches(
        ui,
        theme,
        &[
            ("bg_app", theme.bg_app),
            ("bg_surface", theme.bg_surface),
            ("bg_sunken", theme.bg_sunken),
            ("bg_bar", theme.bg_bar),
        ],
    );

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Accent + tier arc colors", theme);
    color_swatches(
        ui,
        theme,
        &[
            ("accent", theme.accent),
            ("accent_dim", theme.accent_dim),
            ("knob_tier1_arc", theme.knob_tier1_arc),
            ("knob_tier2_arc", theme.knob_tier2_arc),
            ("knob_tier3_arc", theme.knob_tier3_arc),
            ("text_on_accent", theme.text_on_accent),
        ],
    );

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Domain accents", theme);
    color_swatches(
        ui,
        theme,
        &[
            ("accent_hard_sync", theme.accent_hard_sync),
            ("accent_fm", theme.accent_fm),
            ("accent_ring", theme.accent_ring),
            ("accent_hold", theme.accent_hold),
            ("accent_walker", theme.accent_walker),
            ("accent_limiter", theme.accent_limiter),
        ],
    );

    ui.add_space(theme.sp_xl);
    sub_header(ui, "FX colors", theme);
    color_swatches(
        ui,
        theme,
        &[
            ("fx_overdrive", theme.fx_overdrive),
            ("fx_distortion", theme.fx_distortion),
            ("fx_chorus", theme.fx_chorus),
            ("fx_delay", theme.fx_delay),
            ("fx_reverb", theme.fx_reverb),
            ("fx_shimmer", theme.fx_shimmer),
            ("fx_crystallizer", theme.fx_crystallizer),
        ],
    );

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Spacing scale", theme);
    spacing_bars(ui, theme);
}

fn render_components(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    sub_header(ui, "Knob — 3 sizes × 3 tiers", theme);
    knob_grid(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Knob — legacy widget for comparison", theme);
    ui.horizontal(|ui| {
        crate::ui::widgets::knob(
            ui,
            &mut state.legacy_value,
            0.0..=1.0,
            "LEGACY",
            theme,
            false,
        );
        ui.add_space(theme.sp_md);
        ui.label(
            RichText::new(
                "Old path: standard size only, full-accent arc, 9 pt label.\n\
                 New Standard + Tier Secondary above is the migration target.",
            )
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
        );
    });

    ui.add_space(theme.sp_xl);
    sub_header(ui, "ToggleButton — 3 sizes", theme);
    toggle_row(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "ChipSelector", theme);
    chip_row_sample(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "SectionHeader — title + optional right slot", theme);
    section_header_sample(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Fader — 3 sizes vertical + horizontal sample", theme);
    fader_grid(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Slider — inline parameter row", theme);
    slider_samples(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "MiniBar — sequencer-style value bars", theme);
    mini_bar_samples(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "LevelMeter — 3 levels × peak hold", theme);
    level_meter_row(ui, theme);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "ColorChip — tinted band toggle", theme);
    color_chip_sample(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "DrumStep — drum machine step cell", theme);
    drum_step_demo(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "StepPad — 2 sizes × velocity-encoded fill", theme);
    step_pad_grid(ui, theme);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "ChordPad — quality strip × 3 states", theme);
    chord_pad_grid(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "Piano — Preview (36 px) + Full (64 px)", theme);
    piano_samples(ui, theme);
}

fn render_patterns(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    sub_header(ui, "SectionCard — one per Tier", theme);
    ui.horizontal(|ui| {
        for (i, tier) in [Tier::Primary, Tier::Secondary, Tier::Tertiary]
            .iter()
            .enumerate()
        {
            ui.vertical(|ui| {
                ui.set_max_width(240.0);
                let title = match tier {
                    Tier::Primary => "Tier 1 — Primary",
                    Tier::Secondary => "Tier 2 — Secondary",
                    Tier::Tertiary => "Tier 3 — Tertiary",
                };
                ui.section_card(title, *tier, theme, |ui| {
                    ui.label(
                        RichText::new("Card content")
                            .font(theme.font_body())
                            .color(theme.c(&theme.text_secondary)),
                    );
                    ui.add_space(theme.sp_xs);
                    design_toggle(
                        ui,
                        &mut state.card_toggles[i],
                        "ENABLE",
                        ToggleSize::Small,
                        Tier::Tertiary,
                        theme,
                        None,
                    );
                });
            });
        }
    });

    ui.add_space(theme.sp_xl);
    sub_header(ui, "FaderColumn — mixer channel strip", theme);
    fader_column_row(ui, theme, state);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "ChordGrid — 3 rows × 7 cols chord keyboard", theme);
    chord_grid_pattern(ui, theme);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "KnobRow — uniform spacing", theme);
    ui.knob_row(theme, |ui| {
        for col in 0..3 {
            design_knob(
                ui,
                &mut state.knob_values[1][col],
                0.0..=1.0,
                ["CUT", "RES", "ENV"][col],
                theme,
                false,
                KnobSize::Standard,
                Tier::Secondary,
            );
        }
    });
}

fn render_frames(ui: &mut Ui, theme: &SynthTheme) {
    sub_header(ui, "SynthFrame variants", theme);
    ui.label(
        RichText::new(
            "Each frame is a `SynthFrame` factory that pulls its fill / border / \
             rounding / margin from theme tokens.",
        )
        .font(theme.font_small())
        .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_md);

    let demo = |ui: &mut Ui, name: &'static str, frame: egui::Frame| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(name)
                    .font(theme.font_small())
                    .color(theme.c(&theme.text_secondary)),
            );
            ui.add_space(theme.sp_xxs);
            frame.show(ui, |ui| {
                ui.set_min_size(Vec2::new(160.0, 60.0));
                ui.label(
                    RichText::new("sample content")
                        .font(theme.font_body())
                        .color(theme.c(&theme.text_primary)),
                );
            });
        });
    };

    egui::Grid::new("frame_grid")
        .num_columns(2)
        .spacing(Vec2::new(theme.sp_lg, theme.sp_md))
        .show(ui, |ui| {
            demo(ui, "section()", SynthFrame::section(theme));
            demo(ui, "tier1()", SynthFrame::tier1(theme));
            ui.end_row();
            demo(ui, "inset()", SynthFrame::inset(theme));
            demo(ui, "screen()", SynthFrame::screen(theme));
            ui.end_row();
            demo(ui, "bar()", SynthFrame::bar(theme));
            demo(ui, "transport()", SynthFrame::transport(theme));
            ui.end_row();
        });
}

fn render_examples(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    sub_header(ui, "Mini OSC card — components composed", theme);
    ui.label(
        RichText::new(
            "Realistic composition using the design-system vocabulary only. \
             No raw egui widgets, no token-violating literals.",
        )
        .font(theme.font_small())
        .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_md);

    let wave_options: &[(usize, &str)] = &[(0, "Sin"), (1, "Saw"), (2, "Sqr"), (3, "Tri")];

    ui.set_max_width(360.0);
    SynthFrame::tier1(theme).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        design_section_header(
            ui,
            "OSC 1",
            theme,
            Some(|ui: &mut Ui| {
                let mut enabled = state.card_toggles[0];
                design_toggle(
                    ui,
                    &mut enabled,
                    "ON",
                    ToggleSize::Small,
                    Tier::Tertiary,
                    theme,
                    None,
                );
                state.card_toggles[0] = enabled;
            }),
        );
        ui.add_space(theme.sp_xs);

        design_chip(
            ui,
            &mut state.example_wave,
            wave_options,
            theme,
            Some(ui.available_width()),
        );
        ui.add_space(theme.sp_sm);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_md;
            design_knob(
                ui,
                &mut state.example_detune,
                -100.0..=100.0,
                "DET",
                theme,
                false,
                KnobSize::Standard,
                Tier::Secondary,
            );
            design_knob(
                ui,
                &mut state.example_pw,
                0.01..=0.99,
                "PW",
                theme,
                false,
                KnobSize::Standard,
                Tier::Secondary,
            );
            design_toggle(
                ui,
                &mut state.example_uni,
                "UNI",
                ToggleSize::Standard,
                Tier::Secondary,
                theme,
                None,
            );
        });
    });
}

// ─── Building blocks ────────────────────────────────────────────────────────

fn sub_header(ui: &mut Ui, label: &str, theme: &SynthTheme) {
    ui.add_space(theme.sp_sm);
    ui.label(
        RichText::new(label)
            .font(theme.font_heading())
            .color(theme.c(&theme.text_primary)),
    );
    ui.separator();
}

fn knob_grid(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    let sizes = [
        (KnobSize::Large, "Large"),
        (KnobSize::Standard, "Standard"),
        (KnobSize::Small, "Small"),
    ];
    let tiers = [
        (Tier::Primary, "Primary"),
        (Tier::Secondary, "Secondary"),
        (Tier::Tertiary, "Tertiary"),
    ];

    egui::Grid::new("knob_grid")
        .num_columns(4)
        .spacing(Vec2::new(theme.sp_lg, theme.sp_md))
        .show(ui, |ui| {
            ui.label("");
            for (_, tier_label) in &tiers {
                ui.label(
                    RichText::new(*tier_label)
                        .font(theme.font_small())
                        .color(theme.c(&theme.text_secondary)),
                );
            }
            ui.end_row();

            for (size_i, (size, size_label)) in sizes.iter().enumerate() {
                ui.label(
                    RichText::new(*size_label)
                        .font(theme.font_small())
                        .color(theme.c(&theme.text_secondary)),
                );
                for (tier_i, (tier, _)) in tiers.iter().enumerate() {
                    let knob_label = match tier {
                        Tier::Primary => "T1",
                        Tier::Secondary => "T2",
                        Tier::Tertiary => "T3",
                    };
                    design_knob(
                        ui,
                        &mut state.knob_values[size_i][tier_i],
                        0.0..=1.0,
                        knob_label,
                        theme,
                        false,
                        *size,
                        *tier,
                    );
                }
                ui.end_row();
            }
        });
}

fn toggle_row(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    let sizes = [
        (ToggleSize::Large, "Large", "PLAY"),
        (ToggleSize::Standard, "Std", "SYNC"),
        (ToggleSize::Small, "Small", "M"),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.sp_md;
        for (i, (size, _, label)) in sizes.iter().enumerate() {
            ui.vertical(|ui| {
                design_toggle(
                    ui,
                    &mut state.toggle_states[i],
                    label,
                    *size,
                    Tier::Secondary,
                    theme,
                    None,
                );
                let cap = sizes[i].1;
                ui.label(
                    RichText::new(cap)
                        .font(theme.font_micro())
                        .color(theme.c(&theme.text_secondary)),
                );
            });
        }
    });
}

fn chip_row_sample(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    let options: &[(usize, &str)] = &[(0, "SIN"), (1, "SAW"), (2, "SQR"), (3, "TRI")];
    design_chip(ui, &mut state.chip_choice, options, theme, None);
}

fn section_header_sample(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    let toggle_ref = &mut state.section_header_toggle;
    design_section_header(
        ui,
        "FILTER",
        theme,
        Some(|ui: &mut Ui| {
            design_toggle(
                ui,
                toggle_ref,
                "ON",
                ToggleSize::Small,
                Tier::Tertiary,
                theme,
                None,
            );
        }),
    );
}

fn fader_grid(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    let sizes = [
        (FaderSize::Large, "Large (Tier 1)"),
        (FaderSize::Standard, "Standard (Tier 2)"),
        (FaderSize::Small, "Small (Tier 3)"),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.sp_lg;
        for (i, (size, label)) in sizes.iter().enumerate() {
            ui.vertical(|ui| {
                design_fader(
                    ui,
                    &mut state.fader_values[i],
                    0.0..=1.0,
                    FaderOrientation::Vertical,
                    *size,
                    theme,
                );
                ui.label(
                    RichText::new(*label)
                        .font(theme.font_micro())
                        .color(theme.c(&theme.text_secondary)),
                );
            });
        }
    });

    ui.add_space(theme.sp_md);
    ui.label(
        RichText::new("Horizontal (Standard)")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    ui.scope(|ui| {
        ui.set_max_width(280.0);
        design_fader(
            ui,
            &mut state.fader_horizontal,
            0.0..=1.0,
            FaderOrientation::Horizontal,
            FaderSize::Standard,
            theme,
        );
    });
}

fn fader_column_row(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    // First three columns demo the with-meter variant (LIVE-style strip).
    // Fourth column demos the no-meter variant (Studio mixer-style).
    let with_meter = [
        ("O1", state.column_values[0], 0.45, 0.78),
        ("O2", state.column_values[1], 0.62, 0.90),
        ("O3", state.column_values[2], 0.20, 0.55),
    ];
    let without_meter_label = "N (no meter)";

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.sp_md;
        for (i, (label, _, level, peak)) in with_meter.iter().enumerate() {
            design_fader_column(
                ui,
                label,
                &mut state.column_values[i],
                0.0..=1.0,
                Some((*level, *peak)),
                true,
                FaderSize::Standard,
                theme,
            );
        }
        design_fader_column(
            ui,
            without_meter_label,
            &mut state.column_values[3],
            0.0..=1.0,
            None,
            true,
            FaderSize::Standard,
            theme,
        );
    });
}

fn slider_samples(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    ui.scope(|ui| {
        ui.set_max_width(360.0);
        DesignSlider::new(&mut state.slider_values[0], 0.0..=1.0, "Mix").show(ui, theme);
        DesignSlider::new(&mut state.slider_values[1], 80.0..=18_000.0, "Freq")
            .suffix(" Hz")
            .logarithmic(true)
            .decimals(0)
            .show(ui, theme);
        DesignSlider::new(&mut state.slider_values[2], 10.0..=2_000.0, "Time")
            .formatter(|v| {
                if v >= 1000.0 {
                    format!("{:.2} s", v / 1000.0)
                } else {
                    format!("{:.0} ms", v)
                }
            })
            .show(ui, theme);
        DesignSlider::new(&mut state.slider_values[3], 0.0..=1.0, "Drive")
            .decimals(2)
            .show(ui, theme);
    });
}

fn mini_bar_samples(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    ui.label(
        RichText::new("Velocity — solid fill, centered value text, absolute drag")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    let vel_label = format!("{}", state.mini_bar_velocity as u8);
    MiniBar::new(
        &mut state.mini_bar_velocity,
        0.0..=127.0,
        MiniBarOrientation::Horizontal,
        Vec2::new(180.0, 14.0),
    )
    .fill(theme.c(&theme.seq_velocity_bar))
    .label(vel_label, theme.font_micro(), theme.c(&theme.text_primary))
    .show(ui, theme);

    ui.add_space(theme.sp_sm);
    ui.label(
        RichText::new("Probability — 3-zone color (low / mid / high), 50% and 100% thresholds")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    MiniBar::new(
        &mut state.mini_bar_probability,
        0.0..=100.0,
        MiniBarOrientation::Horizontal,
        Vec2::new(180.0, 10.0),
    )
    .zoned(
        50.0,
        100.0,
        theme.c(&theme.seq_prob_low),
        theme.c(&theme.seq_prob_mid),
        theme.c(&theme.seq_prob_high),
    )
    .show(ui, theme);

    ui.add_space(theme.sp_sm);
    ui.label(
        RichText::new("Pitch — vertical, delta drag with caller accumulator, note-name label")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    let pitch_label = crate::ui::midi_note_name(state.mini_bar_pitch as u8);
    MiniBar::new(
        &mut state.mini_bar_pitch,
        48.0..=84.0,
        MiniBarOrientation::Vertical,
        Vec2::new(48.0, 64.0),
    )
    .fill(theme.c(&theme.seq_note_bar_on))
    .label(
        pitch_label,
        theme.font_value(),
        theme.c(&theme.text_primary),
    )
    .drag_delta(&mut state.mini_bar_pitch_accum, 0.3)
    .show(ui, theme);
}

fn level_meter_row(ui: &mut Ui, theme: &SynthTheme) {
    // Static samples that demonstrate the three color zones plus peak-hold
    // line behavior.
    let samples = [
        ("0.3 (green)", 0.3, 0.55),
        ("0.8 (warn)", 0.8, 0.92),
        ("1.0 (clip)", 1.0, 1.0),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.sp_xl;
        for (label, level, peak) in samples {
            ui.vertical(|ui| {
                design_level_meter(
                    ui,
                    level,
                    peak,
                    LevelMeterOrientation::Vertical,
                    LevelMeterSize::Standard,
                    theme,
                );
                ui.label(
                    RichText::new(label)
                        .font(theme.font_micro())
                        .color(theme.c(&theme.text_secondary)),
                );
            });
        }
    });

    ui.add_space(theme.sp_md);
    ui.label(
        RichText::new("Horizontal (Small)")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    design_level_meter(
        ui,
        0.65,
        0.85,
        LevelMeterOrientation::Horizontal,
        LevelMeterSize::Small,
        theme,
    );
}

fn step_pad_grid(ui: &mut Ui, theme: &SynthTheme) {
    let sizes = [
        (StepPadSize::Drum, "Drum (26×24)"),
        (StepPadSize::Note, "Note (20×20)"),
    ];
    let active_samples = [
        (
            "Active 100%",
            StepState::Active {
                velocity: Some(1.0),
            },
        ),
        (
            "Active 50%",
            StepState::Active {
                velocity: Some(0.5),
            },
        ),
        (
            "Active 20%",
            StepState::Active {
                velocity: Some(0.2),
            },
        ),
        ("Active none", StepState::Active { velocity: None }),
        (
            "Current 80%",
            StepState::Current {
                velocity: Some(0.8),
            },
        ),
        ("Inactive", StepState::Inactive),
    ];

    for (size, size_label) in sizes {
        ui.add_space(theme.sp_sm);
        ui.label(
            RichText::new(size_label)
                .font(theme.font_small())
                .color(theme.c(&theme.text_secondary)),
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xxs;
            for (_, state) in &active_samples {
                design_step_pad(ui, *state, size, theme);
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xxs;
            for (label, _) in &active_samples {
                ui.add_sized(
                    size.rect(),
                    egui::Label::new(
                        RichText::new(*label)
                            .font(theme.font_micro())
                            .color(theme.c(&theme.text_secondary)),
                    )
                    .wrap(),
                );
            }
        });
    }
}

fn drum_step_demo(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    ui.label(
        RichText::new(
            "Click to toggle · drag-y to adjust velocity · playhead pinned at step 5 \
             · beat-group ticks at 1,5,9,13",
        )
        .font(theme.font_small())
        .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_xs);

    // Interactive row — active state is editable.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.sp_xxs;
        for step in 0..16usize {
            let s = DrumStepState {
                active: state.drum_step_active[step],
                velocity: state.drum_step_vel[step],
                is_playhead: step == 4,
                is_beat_group: step % 4 == 0,
                is_muted: false,
            };
            let resp = design_drum_step(ui, s, theme);
            if resp.clicked() {
                state.drum_step_active[step] = !state.drum_step_active[step];
            }
            if resp.dragged() && state.drum_step_active[step] {
                state.drum_step_vel[step] =
                    (state.drum_step_vel[step] - resp.drag_delta().y * 0.01).clamp(0.0, 1.0);
            }
        }
    });

    ui.add_space(theme.sp_sm);
    ui.label(
        RichText::new("Same pattern — lane muted (desaturated fill, no accent)")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_xs);

    // Muted display row.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme.sp_xxs;
        for step in 0..16usize {
            let s = DrumStepState {
                active: state.drum_step_active[step],
                velocity: state.drum_step_vel[step],
                is_playhead: false,
                is_beat_group: step % 4 == 0,
                is_muted: true,
            };
            design_drum_step(ui, s, theme);
        }
    });
}

fn color_chip_sample(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    ui.label(
        RichText::new("8 EQ-style band chips — active gets a tinted fill, inactive goes neutral")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_xs);
    let band_colors = [
        theme.c(&theme.accent_fm),
        theme.c(&theme.fx_chorus),
        theme.c(&theme.accent_hold),
        theme.c(&theme.fx_overdrive),
        theme.c(&theme.fx_distortion),
        theme.c(&theme.fx_reverb),
        theme.c(&theme.fx_crystallizer),
        theme.c(&theme.accent),
    ];
    let labels = ["LS", "P1", "P2", "P3", "P4", "P5", "P6", "HS"];
    ui.horizontal(|ui| {
        for i in 0..8usize {
            if design_color_chip(
                ui,
                labels[i],
                band_colors[i],
                state.color_chip_active[i],
                theme,
            )
            .clicked()
            {
                state.color_chip_active[i] = !state.color_chip_active[i];
            }
        }
    });
}

fn piano_samples(ui: &mut Ui, theme: &SynthTheme) {
    // C major scale pitch classes.
    const C_MAJOR: [bool; 12] = [
        true, false, true, false, true, true, false, true, false, true, false, true,
    ];
    // Demonstrate with C3–B5 (MIDI 48–83): C major chord pressed, C major scale highlighted.
    let pressed: &[u8] = &[60, 64, 67]; // C4, E4, G4
    let scale_root: u8 = 0; // C

    ui.label(
        RichText::new(
            "Preview (36 px) — C4 major chord pressed, C major scale highlighted, read-only.",
        )
        .font(theme.font_small())
        .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_xxs);
    design_piano(
        ui,
        &PianoConfig {
            first_midi: 48, // C3
            last_midi: 83,  // B5
            size: PianoSize::Preview,
            interactive: false,
            show_labels: true,
            range_bar: None,
        },
        &|midi| KeyVisualState {
            pressed: pressed.contains(&midi),
            in_kb_range: false,
            is_scale_root: midi % 12 == scale_root,
            in_scale: C_MAJOR[(midi % 12) as usize],
        },
        theme,
    );

    ui.add_space(theme.sp_md);
    ui.label(
        RichText::new("Full (64 px) — same notes, plus KB range bar (C4–E5), full 88-key span.")
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_xxs);
    design_piano(
        ui,
        &PianoConfig {
            first_midi: 21, // A0
            last_midi: 108, // C8
            size: PianoSize::Full,
            interactive: false,
            show_labels: true,
            range_bar: Some((60, 77)), // C4–E5
        },
        &|midi| KeyVisualState {
            pressed: pressed.contains(&midi),
            in_kb_range: (60..77).contains(&midi),
            is_scale_root: midi % 12 == scale_root,
            in_scale: C_MAJOR[(midi % 12) as usize],
        },
        theme,
    );
}

fn chord_pad_grid(ui: &mut Ui, theme: &SynthTheme, state: &mut GalleryState) {
    ui.label(
        RichText::new(
            "3 qualities × 3 states. Click an interactive pad (Normal column) to toggle held.",
        )
        .font(theme.font_small())
        .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_xs);

    let qualities = [
        (ChordQuality::Major, "Major", "Cmaj", "I"),
        (ChordQuality::Minor, "Minor", "Dm", "ii"),
        (ChordQuality::Diminished, "Diminished", "B°", "vii°"),
    ];
    let pad_size = Vec2::new(88.0, 52.0);

    // Column header
    ui.horizontal(|ui| {
        ui.add_space(80.0); // quality label indent
        for col_label in ["Normal (click)", "Held", "Editing"] {
            ui.add_sized(
                Vec2::new(pad_size.x + theme.sp_sm, 16.0),
                egui::Label::new(
                    RichText::new(col_label)
                        .font(theme.font_micro())
                        .color(theme.c(&theme.text_secondary)),
                ),
            );
        }
    });
    ui.add_space(theme.sp_xxs);

    for (i, (quality, quality_label, cname, degree)) in qualities.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_sm;
            // Row label
            ui.add_sized(
                Vec2::new(80.0, pad_size.y),
                egui::Label::new(
                    RichText::new(*quality_label)
                        .font(theme.font_small())
                        .color(theme.c(&theme.text_secondary)),
                ),
            );

            // Column: Normal (interactive)
            let resp = design_chord_pad(
                ui,
                ChordPadState {
                    quality: *quality,
                    chord_name: cname,
                    degree,
                    key_hint: ["Q", "A", "Z"][i],
                    held: state.chord_pad_held[i],
                    editing: false,
                },
                pad_size,
                theme,
            );
            if resp.clicked() {
                state.chord_pad_held[i] = !state.chord_pad_held[i];
            }

            // Column: Held (static)
            design_chord_pad(
                ui,
                ChordPadState {
                    quality: *quality,
                    chord_name: cname,
                    degree,
                    key_hint: "",
                    held: true,
                    editing: false,
                },
                pad_size,
                theme,
            );

            // Column: Editing (static)
            design_chord_pad(
                ui,
                ChordPadState {
                    quality: *quality,
                    chord_name: cname,
                    degree,
                    key_hint: "",
                    held: false,
                    editing: true,
                },
                pad_size,
                theme,
            );
        });
        ui.add_space(theme.sp_xxs);
    }
}

fn chord_grid_pattern(ui: &mut Ui, theme: &SynthTheme) {
    ui.label(
        RichText::new(
            "Static 3 × 7 grid showing all three qualities in a realistic chord-keyboard layout.",
        )
        .font(theme.font_small())
        .color(theme.c(&theme.text_secondary)),
    );
    ui.add_space(theme.sp_xs);

    let row_labels = ["7ths", "Triads", "Sus"];
    let degrees = ["I", "ii", "iii", "IV", "V", "vi", "vii°"];
    let qualities = [
        ChordQuality::Major,
        ChordQuality::Minor,
        ChordQuality::Minor,
        ChordQuality::Major,
        ChordQuality::Major,
        ChordQuality::Minor,
        ChordQuality::Diminished,
    ];
    let chord_names = ["C", "Dm", "Em", "F", "G", "Am", "B°"];
    let key_rows = [
        ["Q", "W", "E", "R", "T", "Y", "U"],
        ["A", "S", "D", "F", "G", "H", "J"],
        ["Z", "X", "C", "V", "B", "N", "M"],
    ];
    let label_w = 48.0_f32;
    let sp = theme.sp_xxs;
    let btn_w = 62.0_f32;
    let btn_h = 52.0_f32;

    egui::Grid::new("gallery_chord_grid")
        .num_columns(8)
        .spacing([sp, sp])
        .show(ui, |ui| {
            for (row, row_label) in row_labels.iter().enumerate() {
                ui.allocate_ui_with_layout(
                    Vec2::new(label_w, btn_h),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.label(
                            RichText::new(*row_label)
                                .weak()
                                .small()
                                .color(theme.c(&theme.text_disabled)),
                        );
                    },
                );
                for col in 0..7usize {
                    design_chord_pad(
                        ui,
                        ChordPadState {
                            quality: qualities[col],
                            chord_name: chord_names[col],
                            degree: degrees[col],
                            key_hint: key_rows[row][col],
                            held: col == 0 && row == 1, // highlight I/Triad
                            editing: col == 4 && row == 0, // highlight V/7th
                        },
                        Vec2::new(btn_w, btn_h),
                        theme,
                    );
                }
                ui.end_row();
            }
        });
}

fn font_samples(ui: &mut Ui, theme: &SynthTheme) {
    let samples = [
        ("font_heading (14 pt)", theme.font_heading()),
        ("font_body (12 pt)", theme.font_body()),
        ("font_value (11 pt mono) — 440.0 Hz", theme.font_value()),
        ("font_small (10 pt)", theme.font_small()),
        ("font_micro (9 pt) — C4 D4 E4", theme.font_micro()),
    ];
    for (label, font) in samples {
        ui.label(
            RichText::new(label)
                .font(font)
                .color(theme.c(&theme.text_primary)),
        );
    }
}

fn color_swatches(ui: &mut Ui, theme: &SynthTheme, swatches: &[(&str, [u8; 3])]) {
    ui.horizontal_wrapped(|ui| {
        for (label, rgb) in swatches {
            ui.vertical(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(80.0, 40.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    theme.rounding_sm,
                    Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                );
                ui.painter().rect_stroke(
                    rect,
                    theme.rounding_sm,
                    Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
                    egui::StrokeKind::Inside,
                );
                ui.label(
                    RichText::new(*label)
                        .font(theme.font_micro())
                        .color(theme.c(&theme.text_secondary)),
                );
            });
            ui.add_space(theme.sp_sm);
        }
    });
}

fn spacing_bars(ui: &mut Ui, theme: &SynthTheme) {
    let bars = [
        ("sp_xxs", theme.sp_xxs),
        ("sp_xs", theme.sp_xs),
        ("sp_sm", theme.sp_sm),
        ("sp_md", theme.sp_md),
        ("sp_lg", theme.sp_lg),
        ("sp_xl", theme.sp_xl),
        ("sp_xxl", theme.sp_xxl),
    ];
    for (label, value) in bars {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{label} ({value:.0} px)"))
                    .font(theme.font_small())
                    .color(theme.c(&theme.text_secondary)),
            );
            let (rect, _) = ui.allocate_exact_size(Vec2::new(value, 12.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 0.0, theme.c(&theme.accent_dim));
        });
    }
}

// ── Layouts category ─────────────────────────────────────────────────────────

/// A cell description for `paint_layout_demo`: fractional rect (0..1) within
/// the demo area, a slot label, and a color palette index.
struct LayoutCell {
    /// Fractional x origin within the demo rect (0..1).
    fx: f32,
    /// Fractional y origin within the demo rect (0..1).
    fy: f32,
    /// Fractional width (0..1).
    fw: f32,
    /// Fractional height (0..1).
    fh: f32,
    label: &'static str,
    /// 0–4: cycles through slot colors derived from theme tokens.
    color_idx: usize,
}

impl LayoutCell {
    const fn new(
        fx: f32,
        fy: f32,
        fw: f32,
        fh: f32,
        label: &'static str,
        color_idx: usize,
    ) -> Self {
        Self {
            fx,
            fy,
            fw,
            fh,
            label,
            color_idx,
        }
    }
}

/// Paint one layout diagram using colored placeholder rectangles.
/// `height` is the pixel height of the demo area; width fills available.
fn paint_layout_demo(
    ui: &mut Ui,
    theme: &SynthTheme,
    name: &str,
    use_case: &str,
    height: f32,
    cells: &[LayoutCell],
) {
    // Slot palette — 5 colors cycling through theme tokens.
    let palette = [
        Color32::from_rgba_unmultiplied(
            theme.knob_tier1_arc[0],
            theme.knob_tier1_arc[1],
            theme.knob_tier1_arc[2],
            180,
        ),
        Color32::from_rgba_unmultiplied(
            theme.knob_tier2_arc[0],
            theme.knob_tier2_arc[1],
            theme.knob_tier2_arc[2],
            180,
        ),
        Color32::from_rgba_unmultiplied(
            theme.knob_tier3_arc[0],
            theme.knob_tier3_arc[1],
            theme.knob_tier3_arc[2],
            180,
        ),
        Color32::from_rgba_unmultiplied(
            theme.accent_fm[0],
            theme.accent_fm[1],
            theme.accent_fm[2],
            160,
        ),
        Color32::from_rgba_unmultiplied(
            theme.accent_hold[0],
            theme.accent_hold[1],
            theme.accent_hold[2],
            160,
        ),
    ];

    let gap = 2.0_f32;
    let rounding = egui::CornerRadius::same(theme.rounding_xs as u8);
    let border_color = theme.c(&theme.border);
    let text_color = theme.c(&theme.text_primary);

    // Name label.
    ui.label(
        RichText::new(name)
            .font(theme.font_body())
            .color(theme.c(&theme.text_primary)),
    );

    // Demo area.
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);

    // Background.
    painter.rect_filled(rect, rounding, theme.c(&theme.bg_sunken));

    let inner = rect.shrink(gap);

    for cell in cells {
        let cell_rect = egui::Rect::from_min_size(
            egui::Pos2::new(
                inner.left()
                    + cell.fx * inner.width()
                    + if cell.fx > 0.0 { gap * 0.5 } else { 0.0 },
                inner.top()
                    + cell.fy * inner.height()
                    + if cell.fy > 0.0 { gap * 0.5 } else { 0.0 },
            ),
            Vec2::new(
                cell.fw * inner.width()
                    - if cell.fx > 0.0 && cell.fx + cell.fw < 1.0 {
                        gap
                    } else if cell.fx > 0.0 || cell.fx + cell.fw < 1.0 {
                        gap * 0.5
                    } else {
                        0.0
                    },
                cell.fh * inner.height()
                    - if cell.fy > 0.0 && cell.fy + cell.fh < 1.0 {
                        gap
                    } else if cell.fy > 0.0 || cell.fy + cell.fh < 1.0 {
                        gap * 0.5
                    } else {
                        0.0
                    },
            ),
        );

        let fill = palette[cell.color_idx % palette.len()];
        painter.rect_filled(cell_rect, rounding, fill);
        painter.rect_stroke(
            cell_rect,
            rounding,
            Stroke::new(1.0, border_color),
            egui::StrokeKind::Inside,
        );

        // Slot label centered in the cell.
        painter.text(
            cell_rect.center(),
            egui::Align2::CENTER_CENTER,
            cell.label,
            theme.font_micro(),
            text_color,
        );
    }

    // Use-case note.
    ui.add_space(2.0);
    ui.label(
        RichText::new(use_case)
            .font(theme.font_small())
            .color(theme.c(&theme.text_secondary)),
    );
}

fn render_layouts(ui: &mut Ui, theme: &SynthTheme) {
    let h_sm = 64.0_f32; // single-row layouts
    let h_md = 100.0_f32; // two-row layouts
    let h_lg = 130.0_f32; // magazine / complex

    // ── Symmetric primitives ─────────────────────────────────────────────────
    sub_header(ui, "Symmetric — equal columns", theme);

    paint_layout_demo(
        ui,
        theme,
        "col2",
        "LFO pairs, Filter/Amp envelopes",
        h_sm,
        &[
            LayoutCell::new(0.0, 0.0, 0.5, 1.0, "slot 0", 0),
            LayoutCell::new(0.5, 0.0, 0.5, 1.0, "slot 1", 1),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "col3",
        "OSC 1 / 2 / 3, config triples",
        h_sm,
        &[
            LayoutCell::new(0.0, 0.0, 1.0 / 3.0, 1.0, "slot 0", 0),
            LayoutCell::new(1.0 / 3.0, 0.0, 1.0 / 3.0, 1.0, "slot 1", 1),
            LayoutCell::new(2.0 / 3.0, 0.0, 1.0 / 3.0, 1.0, "slot 2", 2),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "col4",
        "ADSR, 4-knob parameter rows",
        h_sm,
        &[
            LayoutCell::new(0.0, 0.0, 0.25, 1.0, "A", 0),
            LayoutCell::new(0.25, 0.0, 0.25, 1.0, "D", 1),
            LayoutCell::new(0.5, 0.0, 0.25, 1.0, "S", 2),
            LayoutCell::new(0.75, 0.0, 0.25, 1.0, "R", 3),
        ],
    );

    ui.add_space(theme.sp_xl);

    // ── L-shaped: full top + split bottom ───────────────────────────────────
    sub_header(ui, "L-shaped — full-width header, then columns", theme);

    paint_layout_demo(
        ui,
        theme,
        "top_then_col2",
        "LFO card (header toggle + knobs/chips), scope + readouts",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.35, "header", 4),
            LayoutCell::new(0.0, 0.35, 0.5, 0.65, "body 0", 0),
            LayoutCell::new(0.5, 0.35, 0.5, 0.65, "body 1", 1),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "top_then_col3",
        "OSC card (header + 3 control groups)",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.35, "header", 4),
            LayoutCell::new(0.0, 0.35, 1.0 / 3.0, 0.65, "body 0", 0),
            LayoutCell::new(1.0 / 3.0, 0.35, 1.0 / 3.0, 0.65, "body 1", 1),
            LayoutCell::new(2.0 / 3.0, 0.35, 1.0 / 3.0, 0.65, "body 2", 2),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "top_then_col4",
        "ADSR (header row + 4 fader columns)",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.3, "header", 4),
            LayoutCell::new(0.0, 0.3, 0.25, 0.7, "A", 0),
            LayoutCell::new(0.25, 0.3, 0.25, 0.7, "D", 1),
            LayoutCell::new(0.5, 0.3, 0.25, 0.7, "S", 2),
            LayoutCell::new(0.75, 0.3, 0.25, 0.7, "R", 3),
        ],
    );

    ui.add_space(theme.sp_xl);

    // ── L-shaped: split top + full bottom ───────────────────────────────────
    sub_header(ui, "L-shaped — columns, then full-width footer", theme);

    paint_layout_demo(
        ui,
        theme,
        "col2_then_bottom",
        "Mixer (channel strips → master row), OSC (knob columns → waveform preview)",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 0.5, 0.65, "col 0", 0),
            LayoutCell::new(0.5, 0.0, 0.5, 0.65, "col 1", 1),
            LayoutCell::new(0.0, 0.65, 1.0, 0.35, "footer", 4),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "col3_then_bottom",
        "PULSE (controls → step grid), drum lane (pad row → step row)",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0 / 3.0, 0.55, "col 0", 0),
            LayoutCell::new(1.0 / 3.0, 0.0, 1.0 / 3.0, 0.55, "col 1", 1),
            LayoutCell::new(2.0 / 3.0, 0.0, 1.0 / 3.0, 0.55, "col 2", 2),
            LayoutCell::new(0.0, 0.55, 1.0, 0.45, "footer", 4),
        ],
    );

    ui.add_space(theme.sp_xl);

    // ── Asymmetric splits ────────────────────────────────────────────────────
    sub_header(ui, "Asymmetric — fixed-ratio splits", theme);

    paint_layout_demo(
        ui,
        theme,
        "sidebar_right  (30 / 70)",
        "Knob left + display/chips right: LFO, mod wheel, aftertouch",
        h_sm,
        &[
            LayoutCell::new(0.0, 0.0, 0.30, 1.0, "30%", 0),
            LayoutCell::new(0.30, 0.0, 0.70, 1.0, "70%", 1),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "sidebar_left   (30 / 70)",
        "Label/indicator left + main content right",
        h_sm,
        &[
            LayoutCell::new(0.0, 0.0, 0.30, 1.0, "30%", 2),
            LayoutCell::new(0.30, 0.0, 0.70, 1.0, "70%", 1),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "split_40_60",
        "Balanced asymmetric: two unequal panels",
        h_sm,
        &[
            LayoutCell::new(0.0, 0.0, 0.40, 1.0, "40%", 0),
            LayoutCell::new(0.40, 0.0, 0.60, 1.0, "60%", 3),
        ],
    );

    ui.add_space(theme.sp_xl);

    // ── Asymmetric L-shapes ──────────────────────────────────────────────────
    sub_header(ui, "Asymmetric L-shaped — header + sidebar body", theme);

    paint_layout_demo(
        ui,
        theme,
        "header_sidebar_right",
        "LFO (title + pulse dot | SYNC) → knobs left + chips right",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.3, "header (full width)", 4),
            LayoutCell::new(0.0, 0.3, 0.35, 0.7, "main (35%)", 0),
            LayoutCell::new(0.35, 0.3, 0.65, 0.7, "sidebar right (65%)", 1),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "header_sidebar_left",
        "Filter (title | toggle) → curve display right + controls left",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.3, "header (full width)", 4),
            LayoutCell::new(0.0, 0.3, 0.35, 0.7, "sidebar left (35%)", 2),
            LayoutCell::new(0.35, 0.3, 0.65, 0.7, "main (65%)", 1),
        ],
    );

    ui.add_space(theme.sp_xl);

    // ── Magazine layouts ─────────────────────────────────────────────────────
    sub_header(ui, "Magazine — one tall column + stacked rows", theme);

    paint_layout_demo(
        ui,
        theme,
        "left_tall_right_stacked",
        "OSC mod back (sync/FM/ring controls | shared indicator), EQ (band list + curve)",
        h_lg,
        &[
            LayoutCell::new(0.0, 0.0, 0.35, 1.0, "left (tall)", 0),
            LayoutCell::new(0.35, 0.0, 0.65, 0.33, "right row 0", 1),
            LayoutCell::new(0.35, 0.33, 0.65, 0.34, "right row 1", 2),
            LayoutCell::new(0.35, 0.67, 0.65, 0.33, "right row 2", 3),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "right_tall_left_stacked",
        "Live view (meters tall right | stacked controls left)",
        h_lg,
        &[
            LayoutCell::new(0.0, 0.0, 0.65, 0.33, "left row 0", 1),
            LayoutCell::new(0.0, 0.33, 0.65, 0.34, "left row 1", 2),
            LayoutCell::new(0.0, 0.67, 0.65, 0.33, "left row 2", 3),
            LayoutCell::new(0.65, 0.0, 0.35, 1.0, "right (tall)", 0),
        ],
    );

    ui.add_space(theme.sp_xl);

    // ── True grids ───────────────────────────────────────────────────────────
    sub_header(ui, "Grids — rows × columns", theme);

    paint_layout_demo(
        ui,
        theme,
        "grid 2 × 2",
        "4-param control groups, paired displays",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 0.5, 0.5, "[0,0]", 0),
            LayoutCell::new(0.5, 0.0, 0.5, 0.5, "[0,1]", 1),
            LayoutCell::new(0.0, 0.5, 0.5, 0.5, "[1,0]", 2),
            LayoutCell::new(0.5, 0.5, 0.5, 0.5, "[1,1]", 3),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "grid 2 × 3",
        "Mod matrix rows, sequencer chord grid, effect parameter tables",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0 / 3.0, 0.5, "[0,0]", 0),
            LayoutCell::new(1.0 / 3.0, 0.0, 1.0 / 3.0, 0.5, "[0,1]", 1),
            LayoutCell::new(2.0 / 3.0, 0.0, 1.0 / 3.0, 0.5, "[0,2]", 2),
            LayoutCell::new(0.0, 0.5, 1.0 / 3.0, 0.5, "[1,0]", 3),
            LayoutCell::new(1.0 / 3.0, 0.5, 1.0 / 3.0, 0.5, "[1,1]", 4),
            LayoutCell::new(2.0 / 3.0, 0.5, 1.0 / 3.0, 0.5, "[1,2]", 0),
        ],
    );
    ui.add_space(theme.sp_sm);

    paint_layout_demo(
        ui,
        theme,
        "grid 3 × 4  (step grid)",
        "Drum machine step rows, sequencer pattern grids",
        h_lg,
        &[
            LayoutCell::new(0.0, 0.0, 0.25, 1.0 / 3.0, "r0c0", 0),
            LayoutCell::new(0.25, 0.0, 0.25, 1.0 / 3.0, "r0c1", 1),
            LayoutCell::new(0.5, 0.0, 0.25, 1.0 / 3.0, "r0c2", 2),
            LayoutCell::new(0.75, 0.0, 0.25, 1.0 / 3.0, "r0c3", 3),
            LayoutCell::new(0.0, 1.0 / 3.0, 0.25, 1.0 / 3.0, "r1c0", 1),
            LayoutCell::new(0.25, 1.0 / 3.0, 0.25, 1.0 / 3.0, "r1c1", 2),
            LayoutCell::new(0.5, 1.0 / 3.0, 0.25, 1.0 / 3.0, "r1c2", 3),
            LayoutCell::new(0.75, 1.0 / 3.0, 0.25, 1.0 / 3.0, "r1c3", 0),
            LayoutCell::new(0.0, 2.0 / 3.0, 0.25, 1.0 / 3.0, "r2c0", 2),
            LayoutCell::new(0.25, 2.0 / 3.0, 0.25, 1.0 / 3.0, "r2c1", 3),
            LayoutCell::new(0.5, 2.0 / 3.0, 0.25, 1.0 / 3.0, "r2c2", 0),
            LayoutCell::new(0.75, 2.0 / 3.0, 0.25, 1.0 / 3.0, "r2c3", 1),
        ],
    );

    ui.add_space(theme.sp_xl);

    // ── Nested / composite ───────────────────────────────────────────────────
    sub_header(ui, "Nested — primitives composed inside slots", theme);

    // ADSR card: header + (col4 inside left sidebar | large display right).
    // Left ~30% split into 4 equal fader columns; right ~70% = single display.
    {
        let fw = 0.075_f32; // each fader = 30% / 4
        paint_layout_demo(ui, theme,
            "header  +  (col4 | display)  — ADSR card",
            "Title full-width · left 30% subdivided into 4 fader columns · right 70% = curve display",
            h_lg, &[
            LayoutCell::new(0.0,       0.0,  1.0,  0.22, "title", 4),
            LayoutCell::new(0.0*fw,    0.22, fw,   0.78, "A", 0),
            LayoutCell::new(1.0*fw,    0.22, fw,   0.78, "D", 1),
            LayoutCell::new(2.0*fw,    0.22, fw,   0.78, "S", 2),
            LayoutCell::new(3.0*fw,    0.22, fw,   0.78, "R", 3),
            LayoutCell::new(0.30,      0.22, 0.70, 0.78, "display", 1),
        ]);
    }
    ui.add_space(theme.sp_sm);

    // LFO card: header + (col2 knobs left | 2 stacked chip rows right).
    // Left 35% = 2 knobs side by side; right 65% = SHAPE row + DEST row stacked.
    paint_layout_demo(
        ui,
        theme,
        "header  +  (col2 | stacked rows)  — LFO card",
        "Title + SYNC toggle · left 35% = 2 equal knob slots · right 65% = 2 chip-selector rows",
        h_lg,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.22, "title  +  SYNC →", 4),
            LayoutCell::new(0.0, 0.22, 0.175, 0.78, "RATE", 0),
            LayoutCell::new(0.175, 0.22, 0.175, 0.78, "DEPTH", 1),
            LayoutCell::new(0.35, 0.22, 0.65, 0.39, "SHAPE chips", 2),
            LayoutCell::new(0.35, 0.61, 0.65, 0.39, "→ DEST chips", 3),
        ],
    );
    ui.add_space(theme.sp_sm);

    // Mixer channel strip: col4_then_bottom, each col = label + fader stacked.
    // 4 equal columns (O1/O2/O3/N), each internally: label top + fader body.
    // Footer = full-width master row.
    paint_layout_demo(
        ui,
        theme,
        "col4_then_bottom  with  stacked label+fader inside each col  — Mixer channels",
        "4 equal channel strips each with label + fader · full-width master footer",
        h_lg,
        &[
            // col 0
            LayoutCell::new(0.0, 0.0, 0.25, 0.15, "O1", 4),
            LayoutCell::new(0.0, 0.15, 0.25, 0.60, "fader", 0),
            LayoutCell::new(0.0, 0.75, 0.25, 0.10, "0.80", 0),
            // col 1
            LayoutCell::new(0.25, 0.0, 0.25, 0.15, "O2", 4),
            LayoutCell::new(0.25, 0.15, 0.25, 0.60, "fader", 1),
            LayoutCell::new(0.25, 0.75, 0.25, 0.10, "0.60", 1),
            // col 2
            LayoutCell::new(0.5, 0.0, 0.25, 0.15, "O3", 4),
            LayoutCell::new(0.5, 0.15, 0.25, 0.60, "fader", 2),
            LayoutCell::new(0.5, 0.75, 0.25, 0.10, "0.40", 2),
            // col 3
            LayoutCell::new(0.75, 0.0, 0.25, 0.15, "N", 4),
            LayoutCell::new(0.75, 0.15, 0.25, 0.60, "fader", 3),
            LayoutCell::new(0.75, 0.75, 0.25, 0.10, "0.20", 3),
            // footer
            LayoutCell::new(0.0, 0.85, 1.0, 0.15, "master footer", 4),
        ],
    );
    ui.add_space(theme.sp_sm);

    // OSC card: header + body where body = col3 (knobs) + full-width preview strip.
    // top_then_col3 for the knobs, then waveform preview spans the full width below.
    paint_layout_demo(
        ui,
        theme,
        "header  +  col3  +  full-width strip  — OSC card",
        "Title · chip selector · 3-column knob row (OCT/DET/PW) · full-width waveform preview",
        h_lg,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.18, "title  +  waveform chips", 4),
            LayoutCell::new(0.0, 0.18, 1.0 / 3.0, 0.57, "OCT", 0),
            LayoutCell::new(1.0 / 3.0, 0.18, 1.0 / 3.0, 0.57, "DET", 1),
            LayoutCell::new(2.0 / 3.0, 0.18, 1.0 / 3.0, 0.57, "PW", 2),
            LayoutCell::new(0.0, 0.75, 1.0, 0.25, "waveform preview", 3),
        ],
    );
    ui.add_space(theme.sp_sm);

    // MOD WHEEL / AFTERTOUCH: title + (knob left | chip row right), no header row.
    // Pure sidebar_right: narrow left knob, wide right destination chips.
    paint_layout_demo(
        ui,
        theme,
        "title  +  sidebar_right  (knob | chips)  — Mod Wheel / Aftertouch",
        "Title label · left 28% = single knob · right 72% = destination chip-selector row",
        h_md,
        &[
            LayoutCell::new(0.0, 0.0, 1.0, 0.28, "MOD WHEEL", 4),
            LayoutCell::new(0.0, 0.28, 0.28, 0.72, "DEPTH", 0),
            LayoutCell::new(
                0.28,
                0.28,
                0.72,
                0.72,
                "→  [Off]  [Filter]  [LFO D]  [Amp]",
                1,
            ),
        ],
    );
}
