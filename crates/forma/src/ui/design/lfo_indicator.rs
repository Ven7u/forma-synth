//! LFO rate pulse indicator — Phase 7 animation.
//!
//! A small glowing dot that beats at the LFO rate, driven entirely from the
//! UI thread using `ctx.elapsed_seconds()` × `rate_hz`. No engine changes
//! needed — the visual phase drifts at most a few ms from the real LFO phase,
//! which is imperceptible at all LFO rates below audio range.

use egui::{Color32, Sense, Vec2};

use crate::ui::theme::SynthTheme;

/// Allocates a small square and draws a circle that pulses at `rate_hz`.
///
/// Call inside a `ui.horizontal()` to place the dot beside a label or toggle.
/// When `active` is false the dot is rendered dim and static.
pub fn lfo_pulse_dot(ui: &mut egui::Ui, rate_hz: f32, active: bool, theme: &SynthTheme) {
    let size = 8.0_f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    let base = theme.c(&theme.knob_tier2_arc);
    let center = rect.center();
    let radius = size * 0.42;

    let color = if active {
        // Phase driven by wall-clock time — no engine read needed.
        let phase = (ui.ctx().time() as f32 * rate_hz).fract();
        // Sine envelope: 0 = dark, 1 = full brightness.
        let t = 0.5 + 0.5 * (phase * std::f32::consts::TAU).sin();
        let alpha = (80.0 + t * 175.0) as u8;
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha)
    } else {
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), 35)
    };

    ui.painter().circle_filled(center, radius, color);

    // Outer glow ring at peak (t > 0.7).
    if active {
        let phase = (ui.ctx().time() as f32 * rate_hz).fract();
        let t = 0.5 + 0.5 * (phase * std::f32::consts::TAU).sin();
        if t > 0.7 {
            let glow_alpha = ((t - 0.7) / 0.3 * 60.0) as u8;
            let glow = Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), glow_alpha);
            ui.painter()
                .circle_stroke(center, radius + 1.5, egui::Stroke::new(1.0, glow));
        }
        // Keep repainting so the animation ticks every frame.
        ui.ctx().request_repaint();
    }

    // Tiny static center dot so the indicator is always visible even when dim.
    ui.painter().circle_filled(
        center,
        radius * 0.35,
        Color32::from_rgba_unmultiplied(
            base.r(),
            base.g(),
            base.b(),
            if active { 200 } else { 50 },
        ),
    );
}

/// Draw a vertical bar (like a VU notch) that flashes on each LFO beat.
/// Alternative style — not used by default, kept for easy A/B.
#[allow(dead_code)]
pub fn lfo_beat_flash(ui: &mut egui::Ui, rate_hz: f32, active: bool, theme: &SynthTheme) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(4.0, 10.0), Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let base = theme.c(&theme.knob_tier2_arc);
    let phase = if active {
        (ui.ctx().time() as f32 * rate_hz).fract()
    } else {
        0.0
    };
    // Flash on the first 15% of each cycle.
    let bright = active && phase < 0.15;
    let alpha = if bright { 220u8 } else { 40u8 };
    ui.painter().rect_filled(
        rect,
        egui::CornerRadius::same(1),
        Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha),
    );
    if active {
        ui.ctx().request_repaint();
    }
}
