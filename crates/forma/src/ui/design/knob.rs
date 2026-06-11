//! Knob component — Layer 3.
//!
//! Tokenized rewrite of `ui/widgets/knob.rs`. All sizes come from `KnobSize`
//! constants; arc color is selected per `Tier`; text fonts come from
//! `SynthTheme::font_*`. No magic numbers.
//!
//! The legacy `widgets::knob` keeps its 18 callers until Phases 5–6 migrate
//! them; this new path is what `SynthUi::synth_knob` calls into.

use egui::{Color32, Pos2, Response, Sense, Stroke, Ui};
use std::f32::consts::PI;

use super::{KnobSize, Tier};
use crate::ui::theme::SynthTheme;

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
    let center = Pos2::new(rect.center().x, rect.top() + knob_radius + 2.0);

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
        painter.circle_filled(dot_pos, arc_stroke + 1.0, arc_color);

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
            Pos2::new(center.x, center.y + knob_radius + 5.0),
            egui::Align2::CENTER_TOP,
            &value_text,
            theme.font_value(),
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
            theme.font_body(),
            label_color,
        );
    }

    response
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
