//! Fader component — Layer 3.
//!
//! Per `04-components.md` §Fader: a linear continuous-value slider.
//! Vertical for channel volumes / expression; horizontal for sends / pan /
//! depth values where position-along-a-line reads more naturally than
//! a rotary angle.
//!
//! Sizing follows the design system's tier hierarchy:
//!   Large (Tier 1)    — 120 px length, 16 px thumb
//!   Standard (Tier 2) —  80 px length, 12 px thumb
//!   Small (Tier 3)    —  48 px length,  8 px thumb
//! Track width is fixed at `fader_track_w` (8 px).
//!
//! Interaction:
//! - Click anywhere along the track to jump there.
//! - Drag the thumb to scrub; Shift+drag for fine mode (sensitivity ÷ 5).
//! - Double-click resets to the midpoint of the range.

use egui::{CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaderOrientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaderSize {
    /// Tier 1 — performance.  120 px length, 16 px thumb.
    Large,
    /// Tier 2 — sound design.  80 px length, 12 px thumb.
    Standard,
    /// Tier 3 — config.  48 px length, 8 px thumb.
    Small,
}

impl FaderSize {
    /// Track length along the slide direction.
    pub fn length(self) -> f32 {
        match self {
            FaderSize::Large => 120.0,
            FaderSize::Standard => 80.0,
            FaderSize::Small => 48.0,
        }
    }

    /// Thumb extent along the slide direction.
    pub fn thumb_size(self) -> f32 {
        match self {
            FaderSize::Large => 16.0,
            FaderSize::Standard => 12.0,
            FaderSize::Small => 8.0,
        }
    }
}

/// Track width perpendicular to the slide direction.
///
/// 04-components.md originally specified 8 px (`fader_track_w`), but in
/// practice that read as a thin pencil line in real panels — Ableton
/// uses ~22 px, Logic ~20 px. Bumped to 18 px for visual weight and
/// grab-ability; thumb width is `TRACK_WIDTH + 4` so the thumb still
/// overhangs the track cleanly.
const TRACK_WIDTH: f32 = 18.0;

/// Shift-drag fine-mode factor — matches Knob's FINE_FACTOR.
const FINE_FACTOR: f32 = 0.2;

/// Render a fader. Returns the egui Response so callers can chain
/// `.on_hover_text(...)` etc.
pub fn fader(
    ui: &mut Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    orientation: FaderOrientation,
    size: FaderSize,
    theme: &SynthTheme,
) -> Response {
    let length = size.length();
    let thumb_extent = size.thumb_size();
    // The interactive rect includes the thumb's overhang so the hit target
    // covers the full vertical extent the thumb can travel.
    let (rect_size, axis_len) = match orientation {
        FaderOrientation::Vertical => (Vec2::new(TRACK_WIDTH, length), length),
        FaderOrientation::Horizontal => {
            let avail = ui.available_width().max(length);
            (Vec2::new(avail, TRACK_WIDTH), avail)
        }
    };

    let (rect, mut response) = ui.allocate_exact_size(rect_size, Sense::click_and_drag());

    let span = *range.end() - *range.start();
    let old_value = *value;

    // Map pointer position → value when clicking.
    let pointer_to_value = |p: Pos2| -> f32 {
        let t = match orientation {
            // Vertical: bottom = min, top = max.
            FaderOrientation::Vertical => {
                ((rect.bottom() - p.y) / axis_len).clamp(0.0, 1.0)
            }
            FaderOrientation::Horizontal => {
                ((p.x - rect.left()) / axis_len).clamp(0.0, 1.0)
            }
        };
        *range.start() + t * span
    };

    if response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            *value = pointer_to_value(p).clamp(*range.start(), *range.end());
        }
    }

    if response.dragged() {
        let delta = match orientation {
            // Drag up = positive on vertical.
            FaderOrientation::Vertical => -response.drag_delta().y,
            FaderOrientation::Horizontal => response.drag_delta().x,
        };
        // Pixels of travel per full-range traversal — larger fader = finer.
        let mut sensitivity = span / axis_len;
        if ui.input(|i| i.modifiers.shift) {
            sensitivity *= FINE_FACTOR;
        }
        *value = (*value + delta * sensitivity).clamp(*range.start(), *range.end());
    }

    if response.double_clicked() {
        *value = *range.start() + 0.5 * span;
    }

    if (*value - old_value).abs() > f32::EPSILON {
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let rounding = CornerRadius::same(theme.rounding_sm as u8);

        // Track background.
        painter.rect_filled(rect, rounding, theme.c(&theme.bg_sunken));
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
            StrokeKind::Inside,
        );

        // Value position 0..1 along the axis.
        let t = if span > 0.0 {
            ((*value - *range.start()) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Filled portion below/left of the thumb.
        let fill_color = theme.c(&theme.accent_dim);
        let filled_rect = match orientation {
            FaderOrientation::Vertical => {
                let h = axis_len * t;
                Rect::from_min_size(
                    Pos2::new(rect.left(), rect.bottom() - h),
                    Vec2::new(TRACK_WIDTH, h),
                )
            }
            FaderOrientation::Horizontal => {
                let w = axis_len * t;
                Rect::from_min_size(rect.min, Vec2::new(w, TRACK_WIDTH))
            }
        };
        if filled_rect.width() > 0.0 && filled_rect.height() > 0.0 {
            painter.rect_filled(filled_rect, rounding, fill_color);
        }

        // Thumb.
        let thumb_color = if response.dragged() || response.is_pointer_button_down_on() {
            theme.c(&theme.accent)
        } else if response.hovered() {
            theme.c(&theme.text_primary)
        } else {
            theme.c(&theme.text_secondary)
        };
        let thumb_stroke = if response.dragged() {
            Stroke::new(theme.stroke_active, theme.c(&theme.accent))
        } else {
            Stroke::NONE
        };
        let thumb_rect = match orientation {
            FaderOrientation::Vertical => {
                let cy = rect.bottom() - axis_len * t;
                // Clamp so thumb stays inside the rect at extremes.
                let cy = cy.clamp(rect.top() + thumb_extent * 0.5, rect.bottom() - thumb_extent * 0.5);
                Rect::from_center_size(
                    Pos2::new(rect.center().x, cy),
                    Vec2::new(TRACK_WIDTH + 4.0, thumb_extent),
                )
            }
            FaderOrientation::Horizontal => {
                let cx = rect.left() + axis_len * t;
                let cx = cx.clamp(rect.left() + thumb_extent * 0.5, rect.right() - thumb_extent * 0.5);
                Rect::from_center_size(
                    Pos2::new(cx, rect.center().y),
                    Vec2::new(thumb_extent, TRACK_WIDTH + 4.0),
                )
            }
        };
        painter.rect_filled(thumb_rect, rounding, thumb_color);
        if thumb_stroke.width > 0.0 {
            painter.rect_stroke(thumb_rect, rounding, thumb_stroke, StrokeKind::Inside);
        }
    }

    response
}
