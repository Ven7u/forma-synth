//! ToggleButton component — Layer 3.
//!
//! Per `04-components.md` §ToggleButton: a binary on/off button. Short
//! label (≤ 5 chars typical). Sizes follow `btn_size_*` tokens by Tier.
//! Active state uses an accent fill; inactive uses surface fill.

use egui::{CornerRadius, FontId, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use super::Tier;
use crate::ui::theme::SynthTheme;

/// Rendered size class. `btn_size_*` from §5 of the tokens doc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToggleSize {
    /// 56 × 36 px min — Tier 1 transport-style toggles.
    Large,
    /// 40 × 24 px min — Tier 2 standard.
    Standard,
    /// 28 × 18 px min — Tier 3 compact.
    Small,
}

impl ToggleSize {
    pub fn min_rect(self) -> Vec2 {
        match self {
            ToggleSize::Large => Vec2::new(56.0, 36.0),
            ToggleSize::Standard => Vec2::new(40.0, 24.0),
            ToggleSize::Small => Vec2::new(28.0, 18.0),
        }
    }

    pub fn font(self, theme: &SynthTheme) -> FontId {
        match self {
            ToggleSize::Large => theme.font_body(),
            ToggleSize::Standard => theme.font_body(),
            ToggleSize::Small => theme.font_small(),
        }
    }
}

/// Render a toggle button. The value is flipped on click. Returns the
/// underlying `Response` so callers can chain `.on_hover_text(...)` etc.
///
/// `accent_color` lets the caller pick a domain-specific accent (e.g.
/// `accent_fm` for the FM toggle) while keeping the rest of the visual
/// language consistent. Pass `None` to use the theme's primary accent.
pub fn toggle_button(
    ui: &mut Ui,
    value: &mut bool,
    label: &str,
    size: ToggleSize,
    tier: Tier,
    theme: &SynthTheme,
    accent_color: Option<egui::Color32>,
) -> Response {
    let _ = tier; // tier currently affects size selection at the call site,
                  // not the toggle's own visuals — reserved for future use.

    let min = size.min_rect();
    // Content-driven width: widen if the label needs it, but never shrink.
    let font = size.font(theme);
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), font.clone(), theme.c(&theme.text_primary));
    let desired_w = (galley.size().x + theme.sp_md * 2.0).max(min.x);
    let desired = Vec2::new(desired_w, min.y);

    let (rect, response) = ui.allocate_exact_size(desired, Sense::click());
    if response.clicked() {
        *value = !*value;
    }

    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(theme.rounding_sm as u8);
    let accent = accent_color.unwrap_or_else(|| theme.c(&theme.accent));

    let (fill, text_color, stroke) = match (*value, response.hovered()) {
        (true, _) => {
            // Active: accent fill, high-contrast text-on-accent, no border.
            let text = theme.c(&theme.text_on_accent);
            (accent, text, Stroke::NONE)
        }
        (false, true) => {
            let bg = theme.c(&theme.bg_surface);
            (
                bg,
                theme.c(&theme.text_primary),
                Stroke::new(theme.stroke_focus, theme.c(&theme.border_focus)),
            )
        }
        (false, false) => {
            let bg = theme.c(&theme.bg_sunken);
            (
                bg,
                theme.c(&theme.text_secondary),
                Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
            )
        }
    };

    painter.rect_filled(rect, rounding, fill);
    if stroke.width > 0.0 {
        painter.rect_stroke(rect, rounding, stroke, StrokeKind::Inside);
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        text_color,
    );

    response
}
