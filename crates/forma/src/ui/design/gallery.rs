//! Design System Gallery — categorized Storybook-style viewer.
//!
//! A debug window that renders every token, component, and pattern in the
//! design system against the live theme and zoom factor. Categories live
//! in a left sidebar; the right side shows samples for the active category.
//!
//! Toggle with Ctrl/Cmd + Shift + G.

use egui::{Color32, Context, RichText, Stroke, Ui, Vec2, Window};

use super::{
    chip::chip_selector as design_chip,
    fader::{fader as design_fader, FaderOrientation, FaderSize},
    knob::knob as design_knob,
    layout::fader_column as design_fader_column,
    level_meter::{level_meter as design_level_meter, LevelMeterOrientation, LevelMeterSize},
    section::section_header as design_section_header,
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
    Frames,
    Examples,
}

impl Category {
    const ALL: &'static [Category] = &[
        Category::Tokens,
        Category::Components,
        Category::Patterns,
        Category::Frames,
        Category::Examples,
    ];

    fn label(self) -> &'static str {
        match self {
            Category::Tokens => "Tokens",
            Category::Components => "Components",
            Category::Patterns => "Patterns",
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
                                Category::Components => {
                                    render_components(ui, theme, &mut state)
                                }
                                Category::Patterns => {
                                    render_patterns(ui, theme, &mut state)
                                }
                                Category::Frames => render_frames(ui, theme),
                                Category::Examples => {
                                    render_examples(ui, theme, &mut state)
                                }
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
                .font(if active {
                    theme.font_body()
                } else {
                    theme.font_body()
                })
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
        crate::ui::widgets::knob(ui, &mut state.legacy_value, 0.0..=1.0, "LEGACY", theme, false);
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
    sub_header(ui, "LevelMeter — 3 levels × peak hold", theme);
    level_meter_row(ui, theme);

    ui.add_space(theme.sp_xl);
    sub_header(ui, "StepPad — 2 sizes × velocity-encoded fill", theme);
    step_pad_grid(ui, theme);
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

    let wave_options: &[(usize, &str)] =
        &[(0, "Sin"), (1, "Saw"), (2, "Sqr"), (3, "Tri")];

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
        ("Active 100%", StepState::Active { velocity: Some(1.0) }),
        ("Active 50%", StepState::Active { velocity: Some(0.5) }),
        ("Active 20%", StepState::Active { velocity: Some(0.2) }),
        ("Active none", StepState::Active { velocity: None }),
        ("Current 80%", StepState::Current { velocity: Some(0.8) }),
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
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(80.0, 40.0), egui::Sense::hover());
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
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(value, 12.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(rect, 0.0, theme.c(&theme.accent_dim));
        });
    }
}
