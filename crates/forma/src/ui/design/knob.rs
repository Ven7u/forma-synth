//! Knob component — Layer 3.
//!
//! Tokenized rewrite of `ui/widgets/knob.rs`. All sizes come from `KnobSize`
//! constants; arc color is selected per `Tier`; text fonts come from
//! `SynthTheme::font_*`. No magic numbers.
//!
//! The legacy `widgets::knob` keeps its 18 callers until Phases 5–6 migrate
//! them; this new path is what `SynthUi::synth_knob` calls into.

use egui::{Color32, FontId, Pos2, Response, Sense, Stroke, Ui};
use std::f32::consts::PI;

use super::{KnobSize, Tier};
use crate::ui::theme::SynthTheme;

/// Per-size font selection. Smaller knobs use smaller tokens so text
/// fits inside the allocated rect without clipping or overlap.
fn label_font(size: KnobSize, theme: &SynthTheme) -> FontId {
    match size {
        KnobSize::Large => theme.font_body(),     // 12 pt
        KnobSize::Standard => theme.font_small(), // 10 pt
        KnobSize::Small => theme.font_micro(),    // 9 pt
    }
}

fn value_font(size: KnobSize, theme: &SynthTheme) -> FontId {
    match size {
        KnobSize::Large => theme.font_value(),        // 11 pt mono
        KnobSize::Standard => FontId::monospace(9.0), // 9 pt mono
        KnobSize::Small => FontId::monospace(8.0),    // 8 pt mono — below font_micro by design
    }
}

/// Vertical gap between the knob bottom edge and the value-text top.
fn value_gap(size: KnobSize) -> f32 {
    match size {
        KnobSize::Large => 5.0,
        KnobSize::Standard => 3.0,
        KnobSize::Small => 2.0,
    }
}

/// Radius of the indicator dot at the current-value position on the arc.
/// Slightly fatter than the arc stroke so it reads as a distinct marker.
fn dot_radius(size: KnobSize) -> f32 {
    size.arc_stroke() + 1.0
}

/// Vertical offset from `rect.top()` to the knob center. Must leave room
/// for the indicator dot at its topmost position (angle 270°, value 0.5),
/// otherwise the dot clips against the clipped painter.
fn knob_center_y_offset(size: KnobSize) -> f32 {
    size.radius() + dot_radius(size) + 1.0
}

/// Pixels of drag per full-range traversal — larger knob = finer control.
/// Matches `04-components.md` ("sensitivity: (max - min) / 300 px for
/// Standard; / 500 for Large").
fn normal_speed(size: KnobSize) -> f32 {
    match size {
        KnobSize::Large => 0.003,    // ~1/333
        KnobSize::Standard => 0.005, // ~1/200 — preserves old feel
        KnobSize::Small => 0.007,    // coarser; tier 3 controls
    }
}

/// Shift-drag fine-mode factor (sensitivity ÷ 10).
const FINE_FACTOR: f32 = 0.2;

/// Render an arc knob.
pub fn knob(
    ui: &mut Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    label: &str,
    theme: &SynthTheme,
    logarithmic: bool,
    size: KnobSize,
    tier: Tier,
) -> Response {
    let desired_size = size.rect();
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    let knob_radius = size.radius();
    let center = Pos2::new(rect.center().x, rect.top() + knob_center_y_offset(size));

    let old_value = *value;

    let log_t = |v: f32| -> f32 {
        if logarithmic && *range.start() > 0.0 {
            (v / *range.start()).ln() / (*range.end() / *range.start()).ln()
        } else {
            (v - *range.start()) / (*range.end() - *range.start())
        }
    };
    let log_v = |t: f32| -> f32 {
        if logarithmic && *range.start() > 0.0 {
            *range.start() * (*range.end() / *range.start()).powf(t)
        } else {
            *range.start() + t * (*range.end() - *range.start())
        }
    };

    if response.dragged() {
        let delta = -response.drag_delta().y;
        let mut speed = normal_speed(size);
        if ui.input(|i| i.modifiers.shift) {
            speed *= FINE_FACTOR;
        }
        let t = (log_t(*value) + delta * speed).clamp(0.0, 1.0);
        *value = log_v(t).clamp(*range.start(), *range.end());
    }

    if response.double_clicked() {
        *value = log_v(0.5);
    }

    if (*value - old_value).abs() > f32::EPSILON {
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);

        let range_span = *range.end() - *range.start();
        let t = if range_span > 0.0 { log_t(*value) } else { 0.0 };

        let start_angle = PI * 0.75;
        let sweep = PI * 1.5;

        let arc_color = match tier {
            Tier::Primary => theme.c(&theme.knob_tier1_arc),
            Tier::Secondary => theme.c(&theme.knob_tier2_arc),
            Tier::Tertiary => theme.c(&theme.knob_tier3_arc),
        };
        let arc_stroke = size.arc_stroke();

        // Dim track behind the colored fill.
        let track_color = Color32::from_rgba_premultiplied(
            arc_color.r() / 4,
            arc_color.g() / 4,
            arc_color.b() / 4,
            80,
        );
        draw_arc(
            &painter,
            center,
            knob_radius,
            start_angle,
            sweep,
            arc_stroke,
            track_color,
        );

        if t > 0.01 {
            draw_arc(
                &painter,
                center,
                knob_radius,
                start_angle,
                sweep * t,
                arc_stroke + 0.5,
                arc_color,
            );
        }

        // Indicator dot at current value.
        let indicator_angle = start_angle + sweep * t;
        let dot_pos = Pos2::new(
            center.x + indicator_angle.cos() * knob_radius,
            center.y + indicator_angle.sin() * knob_radius,
        );
        painter.circle_filled(dot_pos, dot_radius(size), arc_color);

        // Center dot — brightens on interaction.
        let center_color = if response.hovered() || response.dragged() {
            theme.c(&theme.text_primary)
        } else {
            theme.c(&theme.text_secondary)
        };
        painter.circle_filled(center, 4.0, center_color);

        let value_text = if range_span >= 100.0 {
            format!("{:.0}", *value)
        } else if range_span >= 1.0 {
            format!("{:.1}", *value)
        } else {
            format!("{:.2}", *value)
        };
        painter.text(
            Pos2::new(center.x, center.y + knob_radius + value_gap(size)),
            egui::Align2::CENTER_TOP,
            &value_text,
            value_font(size, theme),
            theme.c(&theme.text_secondary),
        );

        // Label color follows tier — Tier 1 reads as primary text.
        let label_color = match tier {
            Tier::Primary => theme.c(&theme.text_primary),
            _ => theme.c(&theme.text_secondary),
        };
        painter.text(
            Pos2::new(center.x, rect.bottom() - 1.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            label_font(size, theme),
            label_color,
        );
    }

    response
}

/// Returns `(value_y_top, value_y_bottom, label_y_top, label_y_bottom)` for
/// a knob of `size` placed at the given `rect_top`. Used by tests to assert
/// the value text and label text never overlap each other or escape the
/// allocated rect.
#[cfg(test)]
fn vertical_text_extents(size: KnobSize, rect_top: f32) -> (f32, f32, f32, f32) {
    // Mirrors the layout math in `knob()`.
    let radius = size.radius();
    let center_y = rect_top + knob_center_y_offset(size);
    let value_top = center_y + radius + value_gap(size);
    // Font heights are approximate (≈ pt × 1.1). Conservative.
    let value_font_pt = match size {
        KnobSize::Large => 11.0,
        KnobSize::Standard => 9.0,
        KnobSize::Small => 8.0,
    };
    let label_font_pt = match size {
        KnobSize::Large => 12.0,
        KnobSize::Standard => 10.0,
        KnobSize::Small => 9.0,
    };
    let value_bottom = value_top + value_font_pt * 1.1;
    let label_bottom = rect_top + size.rect().y - 1.0;
    let label_top = label_bottom - label_font_pt * 1.1;
    (value_top, value_bottom, label_top, label_bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At every value 0..=1 the indicator dot must stay inside the
    /// allocated rect. The clipped painter would otherwise crop it.
    #[test]
    fn indicator_dot_stays_inside_rect() {
        let start_angle = PI * 0.75;
        let sweep = PI * 1.5;
        for size in [KnobSize::Large, KnobSize::Standard, KnobSize::Small] {
            let rect_top = 0.0;
            let rect = size.rect();
            let center_x = rect.x / 2.0;
            let center_y = rect_top + knob_center_y_offset(size);
            let radius = size.radius();
            let dot_r = dot_radius(size);
            // Sample the sweep finely — covers all extremes (top, sides, bottom).
            for i in 0..=100 {
                let t = i as f32 / 100.0;
                let angle = start_angle + sweep * t;
                let dot_x = center_x + angle.cos() * radius;
                let dot_y = center_y + angle.sin() * radius;
                assert!(
                    dot_y - dot_r >= rect_top,
                    "{size:?} t={t}: dot top {} above rect top {rect_top}",
                    dot_y - dot_r
                );
                assert!(
                    dot_y + dot_r <= rect_top + rect.y,
                    "{size:?} t={t}: dot bottom {} below rect bottom {}",
                    dot_y + dot_r,
                    rect_top + rect.y
                );
                assert!(
                    dot_x - dot_r >= 0.0,
                    "{size:?} t={t}: dot left {} outside rect",
                    dot_x - dot_r
                );
                assert!(
                    dot_x + dot_r <= rect.x,
                    "{size:?} t={t}: dot right {} outside rect width {}",
                    dot_x + dot_r,
                    rect.x
                );
            }
        }
    }

    /// For every knob size, the value text and the label text must fit
    /// inside the allocated rect and must not overlap each other.
    #[test]
    fn text_fits_within_rect_and_does_not_overlap() {
        for size in [KnobSize::Large, KnobSize::Standard, KnobSize::Small] {
            let rect_top = 0.0;
            let rect_height = size.rect().y;
            let (v_top, v_bot, l_top, l_bot) = vertical_text_extents(size, rect_top);

            assert!(
                v_top >= rect_top,
                "{size:?}: value text top {v_top} above rect top {rect_top}"
            );
            assert!(
                l_bot <= rect_top + rect_height,
                "{size:?}: label text bottom {l_bot} below rect bottom {}",
                rect_top + rect_height
            );
            assert!(
                v_bot <= l_top,
                "{size:?}: value bottom {v_bot} overlaps label top {l_top}"
            );
        }
    }
}

fn draw_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_angle: f32,
    sweep: f32,
    width: f32,
    color: Color32,
) {
    let segments = (sweep.abs() * 20.0).ceil() as usize;
    if segments < 2 {
        return;
    }
    let step = sweep / segments as f32;
    for i in 0..segments {
        let a0 = start_angle + step * i as f32;
        let a1 = start_angle + step * (i + 1) as f32;
        let p0 = Pos2::new(center.x + a0.cos() * radius, center.y + a0.sin() * radius);
        let p1 = Pos2::new(center.x + a1.cos() * radius, center.y + a1.sin() * radius);
        painter.line_segment([p0, p1], Stroke::new(width, color));
    }
}
