//! DrumStep component — Layer 3.
//!
//! A single step cell for the drum machine grid. Visually distinct from
//! `StepPad` (note sequencer): drum steps encode beat-group accents via a
//! top-edge tick, velocity via a bottom-up fill, playhead via a bright
//! top bar, and muted-lane state via a desaturated fill color.
//!
//! The component only draws — it returns a `Response` and the caller
//! handles all state mutations:
//! - `resp.clicked()` → toggle active
//! - `resp.dragged()` + `resp.drag_delta().y` → adjust velocity

use egui::{CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

/// Logical state passed to [`drum_step`] each frame.
#[derive(Clone, Copy, Debug)]
pub struct DrumStepState {
    /// Step is programmed on.
    pub active: bool,
    /// Normalized velocity (0.0–1.0). Controls fill height; only used when `active`.
    pub velocity: f32,
    /// This step is the current playhead position.
    pub is_playhead: bool,
    /// Step falls on a beat boundary (index % 4 == 0).
    pub is_beat_group: bool,
    /// The lane owning this step is muted (or effectively muted via solo).
    pub is_muted: bool,
}

/// Render a drum machine step cell (26 × 24 px).
///
/// Visual layers (back → front):
/// 1. `seq_step_off` background — the "empty" pad.
/// 2. Velocity fill from bottom — `accent` (beat), `accent_dim` (off-beat),
///    or `border` (muted). Height = velocity × pad height.
/// 3. Beat-group tick — 2 px top-edge line in `border_focus` marking
///    bar divisions. Hidden when the playhead occupies this step.
/// 4. Playhead bar — 2 px top-edge line in `accent`, overrides the beat tick.
/// 5. Hover / press stroke — `border_focus` ring that brightens on interaction.
pub fn drum_step(ui: &mut Ui, state: DrumStepState, theme: &SynthTheme) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(26.0, 24.0), Sense::click_and_drag());

    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(theme.rounding_xs as u8);

    // ── 1. Background ─────────────────────────────────────────────────────────
    painter.rect_filled(rect, rounding, theme.c(&theme.seq_step_off));

    // ── 2. Velocity fill ──────────────────────────────────────────────────────
    if state.active {
        let fill_color = if state.is_muted {
            theme.c(&theme.border)
        } else if state.is_beat_group {
            theme.c(&theme.accent)
        } else {
            theme.c(&theme.accent_dim)
        };
        let fill_h = (rect.height() * state.velocity.clamp(0.0, 1.0)).max(2.0);
        let fill_rect = Rect::from_min_size(
            egui::pos2(rect.left(), rect.bottom() - fill_h),
            Vec2::new(rect.width(), fill_h),
        );
        painter.rect_filled(fill_rect, rounding, fill_color);
    }

    // ── 3. Beat-group tick ────────────────────────────────────────────────────
    if state.is_beat_group && !state.is_playhead {
        let tick = Rect::from_min_size(
            rect.min,
            Vec2::new(rect.width(), theme.stroke_active),
        );
        painter.rect_filled(tick, CornerRadius::ZERO, theme.c(&theme.border_focus));
    }

    // ── 4. Playhead bar ───────────────────────────────────────────────────────
    if state.is_playhead {
        let bar = Rect::from_min_size(
            rect.min,
            Vec2::new(rect.width(), theme.stroke_active),
        );
        painter.rect_filled(bar, CornerRadius::ZERO, theme.c(&theme.accent));
    }

    // ── 5. Hover / press border ───────────────────────────────────────────────
    let stroke = if response.is_pointer_button_down_on() {
        Stroke::new(theme.stroke_active, theme.c(&theme.border_focus))
    } else if response.hovered() {
        Stroke::new(theme.stroke_focus, theme.c(&theme.border_focus))
    } else {
        Stroke::NONE
    };
    if stroke != Stroke::NONE {
        painter.rect_stroke(rect, rounding, stroke, StrokeKind::Inside);
    }

    response
}
