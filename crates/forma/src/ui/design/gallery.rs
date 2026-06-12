//! Design System Gallery — Storybook-style viewer for design system
//! components. A debug window that renders every component in every
//! variant against the live theme and zoom factor.
//!
//! Toggle with Ctrl/Cmd + Shift + G or via the menu.

use egui::{Color32, Context, RichText, Stroke, Ui, Vec2, Window};

use super::{
    chip::chip_selector as design_chip,
    knob::knob as design_knob,
    section::section_header as design_section_header,
    step_pad::{step_pad as design_step_pad, StepPadSize, StepState},
    toggle::{toggle_button as design_toggle, ToggleSize},
    KnobSize, Tier,
};
use crate::ui::theme::SynthTheme;

/// Per-session demo state — knob values the user can drag around.
#[derive(Clone)]
struct GalleryState {
    /// 3 sizes × 3 tiers of knob values, plus 1 for the legacy comparison.
    knob_values: [[f32; 3]; 3],
    legacy_value: f32,
    /// 3 toggle samples — one per size.
    toggle_states: [bool; 3],
    /// Chip selector demo.
    chip_choice: usize,
    /// Independent toggle for the section-header right-slot sample so it
    /// doesn't shadow one of the `toggle_states` entries.
    section_header_toggle: bool,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            knob_values: [[0.3; 3]; 3],
            legacy_value: 0.3,
            toggle_states: [false, true, false],
            chip_choice: 1,
            section_header_toggle: true,
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
        .default_size([720.0, 640.0])
        .resizable(true)
        .scroll([false, true])
        .show(ctx, |ui| {
            section_header(ui, "Knobs — 3 sizes × 3 tiers", theme);
            knob_grid(ui, theme, &mut state);

            ui.add_space(theme.sp_xl);
            section_header(ui, "Legacy knob (widgets::knob) — visual baseline", theme);
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
                        "Old path — Standard size only, accent-colored arc, 9 pt label.\n\
                         New Standard + Tier Secondary above should match in dimensions.",
                    )
                    .font(theme.font_small())
                    .color(theme.c(&theme.text_secondary)),
                );
            });

            ui.add_space(theme.sp_xl);
            section_header(ui, "Font tokens", theme);
            font_samples(ui, theme);

            ui.add_space(theme.sp_xl);
            section_header(ui, "Toggle buttons — 3 sizes, on/off", theme);
            toggle_row(ui, theme, &mut state);

            ui.add_space(theme.sp_xl);
            section_header(ui, "Chip selector — content-driven width", theme);
            chip_row_sample(ui, theme, &mut state);

            ui.add_space(theme.sp_xl);
            section_header(ui, "Section header — title + optional right slot", theme);
            section_header_sample(ui, theme, &mut state);

            ui.add_space(theme.sp_xl);
            section_header(ui, "Step pads — 2 sizes × 3 states", theme);
            step_pad_grid(ui, theme);

            ui.add_space(theme.sp_xl);
            section_header(ui, "Tier arc colors", theme);
            color_swatches(
                ui,
                theme,
                &[
                    ("accent", theme.accent),
                    ("knob_tier1_arc", theme.knob_tier1_arc),
                    ("knob_tier2_arc", theme.knob_tier2_arc),
                    ("knob_tier3_arc", theme.knob_tier3_arc),
                ],
            );

            ui.add_space(theme.sp_xl);
            section_header(ui, "Surfaces", theme);
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
            section_header(ui, "Spacing scale", theme);
            spacing_bars(ui, theme);
        });

    save_state(ctx, state);
}

fn section_header(ui: &mut Ui, label: &str, theme: &SynthTheme) {
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
            // Header row: tier names.
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
                    // Use short labels — real knob labels are 3–4 chars
                    // ("CUT", "RES"); the row header already names the size.
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
                    egui::RichText::new(cap)
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

fn step_pad_grid(ui: &mut Ui, theme: &SynthTheme) {
    let sizes = [
        (StepPadSize::Drum, "Drum (26×24)"),
        (StepPadSize::Note, "Note (20×20)"),
    ];
    // Sample velocities so the "velocity = inner fill height" property
    // is visible at a glance. None = binary on/off.
    let active_samples = [
        ("Active 100%", StepState::Active { velocity: Some(1.0) }),
        ("Active 50%",  StepState::Active { velocity: Some(0.5) }),
        ("Active 20%",  StepState::Active { velocity: Some(0.2) }),
        ("Active none", StepState::Active { velocity: None }),
        ("Current 80%", StepState::Current { velocity: Some(0.8) }),
        ("Inactive",    StepState::Inactive),
    ];

    for (size, size_label) in sizes {
        ui.add_space(theme.sp_sm);
        ui.label(
            egui::RichText::new(size_label)
                .font(theme.font_small())
                .color(theme.c(&theme.text_secondary)),
        );
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xxs;
            for (_, state) in &active_samples {
                design_step_pad(ui, *state, size, theme);
            }
        });
        // Labels under the row.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xxs;
            for (label, _) in &active_samples {
                ui.add_sized(
                    size.rect(),
                    egui::Label::new(
                        egui::RichText::new(*label)
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
    ui.horizontal(|ui| {
        for (label, rgb) in swatches {
            ui.vertical(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(64.0, 40.0), egui::Sense::hover());
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
