//! StepPad component — Layer 3.
//!
//! A sequencer / drum grid step button. Per `04-components.md` §StepPad:
//! three logical states (Inactive / Active / Current playhead), two sizes
//! (drum 26×24, note seq 20×20), and an optional velocity value that
//! encodes via fill height inside the pad.
//!
//! All dimensions and colors come from tokens. No hardcoded numbers in
//! the public API.

use egui::{CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

/// Logical state of a single step. `velocity` (0..=1) is honored on Active
/// / Current and ignored on Inactive. When `velocity == None` the pad
/// fills to full height (binary on/off mode).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepState {
    /// Programmed off — neutral background.
    Inactive,
    /// Programmed on — fill color = `seq_step_on`.
    Active { velocity: Option<f32> },
    /// The step currently playing — fill + glow + active stroke.
    Current { velocity: Option<f32> },
}

/// Pad size. Drum machine uses Drum; note sequencer uses Note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepPadSize {
    /// 26 × 24 px (drum machine).
    Drum,
    /// 20 × 20 px (note sequencer).
    Note,
}

impl StepPadSize {
    pub fn rect(self) -> Vec2 {
        match self {
            StepPadSize::Drum => Vec2::new(26.0, 24.0),
            StepPadSize::Note => Vec2::new(20.0, 20.0),
        }
    }
}

/// Render a step pad. Returns the egui Response so the caller can detect
/// click / drag / hover.
pub fn step_pad(
    ui: &mut Ui,
    state: StepState,
    size: StepPadSize,
    theme: &SynthTheme,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size.rect(), Sense::click());

    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(theme.rounding_xs as u8);

    let (base_fill, is_current) = match state {
        StepState::Inactive => (theme.c(&theme.seq_step_off), false),
        StepState::Active { .. } => (theme.c(&theme.seq_step_on), false),
        StepState::Current { .. } => (theme.c(&theme.seq_current), true),
    };

    // Background — always full rect, even when velocity < 1 so the pad
    // outline reads as the same shape across all states.
    let bg_fill = theme.c(&theme.seq_step_off);
    painter.rect_filled(rect, rounding, bg_fill);

    // Velocity-encoded inner fill. `None` velocity = full pad.
    let velocity = match state {
        StepState::Inactive => 0.0,
        StepState::Active { velocity } | StepState::Current { velocity } => {
            velocity.unwrap_or(1.0).clamp(0.0, 1.0)
        }
    };

    if velocity > 0.0 {
        let inner_h = (rect.height() * velocity).max(2.0);
        let inner_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x, rect.max.y - inner_h),
            Vec2::new(rect.width(), inner_h),
        );
        painter.rect_filled(inner_rect, rounding, base_fill);
    }

    // State-specific border / stroke.
    let stroke = if is_current {
        Stroke::new(theme.stroke_active, theme.c(&theme.accent))
    } else if response.is_pointer_button_down_on() {
        Stroke::new(theme.stroke_active, theme.c(&theme.border_focus))
    } else if response.hovered() {
        Stroke::new(theme.stroke_focus, theme.c(&theme.border_focus))
    } else {
        Stroke::new(theme.stroke_ui, theme.c(&theme.border))
    };
    painter.rect_stroke(rect, rounding, stroke, StrokeKind::Inside);

    response
}
