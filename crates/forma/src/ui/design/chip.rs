//! ChipSelector component — Layer 3.
//!
//! Per `04-components.md` §ChipSelector: a row of mutually-exclusive
//! options rendered as connected chips. Selected chip gets the accent
//! fill; others share `state_idle`. Chips share their inner borders
//! (no gap between them) so the row reads as a unified control.

use egui::{Color32, CornerRadius, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

/// Render a chip selector. `options` is `(value, label)` pairs. Sets
/// `*selected` when an option is clicked; returns the union response
/// so callers can hover-text the whole row.
///
/// `width` controls horizontal sizing strategy: `None` lets each chip
/// size to its label (content-driven); `Some(w)` forces a fixed total
/// width split evenly.
pub fn chip_selector<T: Copy + PartialEq>(
    ui: &mut Ui,
    selected: &mut T,
    options: &[(T, &str)],
    theme: &SynthTheme,
    width: Option<f32>,
) -> Response {
    let chip_h = 22.0_f32;
    let n = options.len();

    // Pre-measure label widths for content sizing.
    let font = theme.font_body();
    let mut chip_widths: Vec<f32> = options
        .iter()
        .map(|(_, lbl)| {
            let g =
                ui.painter().layout_no_wrap(
                    lbl.to_string(),
                    font.clone(),
                    theme.c(&theme.text_primary),
                );
            (g.size().x + theme.sp_md * 2.0).max(28.0)
        })
        .collect();

    if let Some(total_w) = width {
        let per = total_w / n as f32;
        chip_widths = vec![per; n];
    }
    let total_w: f32 = chip_widths.iter().sum();

    let inner = ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0; // chips share borders
        ui.horizontal(|ui| {
            for (i, (value, label)) in options.iter().enumerate() {
                let w = chip_widths[i];
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(w, chip_h), Sense::click());
                let active = *selected == *value;
                if resp.clicked() {
                    *selected = *value;
                }
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let painter = ui.painter_at(rect);
                // Rounded only on the outer edges of the chip row.
                let r = match (i == 0, i == n - 1) {
                    (true, true) => CornerRadius::same(theme.rounding_sm as u8),
                    (true, false) => CornerRadius {
                        nw: theme.rounding_sm as u8,
                        sw: theme.rounding_sm as u8,
                        ne: 0,
                        se: 0,
                    },
                    (false, true) => CornerRadius {
                        nw: 0,
                        sw: 0,
                        ne: theme.rounding_sm as u8,
                        se: theme.rounding_sm as u8,
                    },
                    (false, false) => CornerRadius::ZERO,
                };
                let fill = if active {
                    theme.c(&theme.accent)
                } else if resp.hovered() {
                    theme.c(&theme.bg_surface)
                } else {
                    theme.c(&theme.bg_sunken)
                };
                painter.rect_filled(rect, r, fill);
                painter.rect_stroke(
                    rect,
                    r,
                    egui::Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
                    StrokeKind::Inside,
                );
                let text_color = if active {
                    theme.c(&theme.text_on_accent)
                } else {
                    theme.c(&theme.text_secondary)
                };
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    *label,
                    font.clone(),
                    text_color,
                );
            }
        });
    });

    let _ = total_w;
    // The scope's response covers the whole chip row — return it so callers
    // can chain `.on_hover_text(...)` on the entire selector.
    inner.response
}

/// A tinted toggle chip for color-coded band controls (EQ bands, per-channel
/// toggles, etc.). Active: dim fill + colored text/border. Inactive: neutral.
///
/// The caller manages state and handles clicks:
/// ```ignore
/// if color_chip(ui, "LS", band_color, band.enabled, theme).clicked() {
///     band.enabled = !band.enabled;
/// }
/// ```
pub fn color_chip(
    ui: &mut Ui,
    label: &str,
    color: Color32,
    active: bool,
    theme: &crate::ui::theme::SynthTheme,
) -> Response {
    let (bg, text_col, stroke_col) = if active {
        (color.gamma_multiply(0.25), color, color)
    } else {
        (
            theme.c(&theme.bg_sunken),
            theme.c(&theme.text_disabled),
            theme.c(&theme.border),
        )
    };
    let btn = egui::Button::new(RichText::new(label).small().color(text_col))
        .fill(bg)
        .stroke(Stroke::new(theme.stroke_ui, stroke_col));
    ui.add(btn)
}
