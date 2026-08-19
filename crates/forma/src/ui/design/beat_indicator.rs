//! Beat indicator — Phase 7 design system component.
//!
//! Draws the time-signature label + one pulse dot per beat:
//!   • Dot 0 (beat 1): pulses with `accent` color (downbeat)
//!   • Dots 1+ (beats 2…N): pulse with `accent_beat` color
//!
//! Width is dynamic — scales with the beat count.
//! All colors come from theme tokens; no hardcoded values.

use egui::{Align2, Color32, FontId, Pos2, Rect, Response, Sense, Vec2};

use crate::ui::theme::SynthTheme;

const DOT_R: f32 = 3.5;
const DOT_GAP: f32 = 4.0;
const SIG_W: f32 = 30.0;
const SIG_GAP: f32 = 6.0;

fn dots_width(beats: u8) -> f32 {
    let n = beats.max(1) as f32;
    n * DOT_R * 2.0 + (n - 1.0) * DOT_GAP
}

/// State passed to [`draw_beat_indicator`].
pub struct BeatState {
    /// Whether the metronome / sequencer is currently running.
    pub active: bool,
    /// Current beat index within the bar (0 = beat 1).
    pub beat_idx: usize,
    /// Phase within the current beat (0.0 = start, 1.0 = end).
    pub beat_frac: f32,
    /// Time-signature numerator (e.g. 4 for 4/4).
    pub beats: u8,
    /// Time-signature denominator (e.g. 4 for 4/4).
    pub denom: u8,
    /// Whether the metronome settings popover is open (tints the sig label).
    pub show_metronome: bool,
}

/// Allocates a rect sized for the current beat count and draws the indicator.
///
/// Returns the `Response` so the caller can attach `.on_hover_text()` and a
/// click handler for opening the metronome settings popover.
pub fn draw_beat_indicator(ui: &mut egui::Ui, state: &BeatState, theme: &SynthTheme) -> Response {
    let total_w = SIG_W + SIG_GAP + dots_width(state.beats);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(total_w, ui.available_height()), Sense::click());

    if ui.is_rect_visible(rect) {
        paint_beat_indicator(ui.painter(), rect, state, theme);
    }

    response
}

/// Paint-only variant for callers that manage their own allocation.
pub fn paint_beat_indicator(
    painter: &egui::Painter,
    rect: Rect,
    state: &BeatState,
    theme: &SynthTheme,
) {
    let cy = rect.center().y;

    // ── Time-signature label ("4/4") ──────────────────────────────────────
    let sig_col = if state.show_metronome {
        theme.c(&theme.accent)
    } else {
        theme.c(&theme.text_primary)
    };
    painter.text(
        Pos2::new(rect.left() + SIG_W * 0.5, cy),
        Align2::CENTER_CENTER,
        format!("{}/{}", state.beats, state.denom),
        FontId::monospace(10.0),
        sig_col,
    );

    // ── One dot per beat ──────────────────────────────────────────────────
    let accent_full = theme.c(&theme.accent);
    let beat_full = theme.c(&theme.accent_beat);

    let dots_x0 = rect.left() + SIG_W + SIG_GAP + DOT_R;

    for i in 0..state.beats as usize {
        let cx = dots_x0 + i as f32 * (DOT_R * 2.0 + DOT_GAP);
        let center = Pos2::new(cx, cy);

        let is_current = state.active && i == state.beat_idx;
        let pulse = if is_current {
            (1.0_f32 - state.beat_frac).powf(2.2)
        } else {
            0.0
        };

        let full_color = if i == 0 { accent_full } else { beat_full };
        let dim_color = Color32::from_rgb(
            (full_color.r() as f32 * 0.12) as u8,
            (full_color.g() as f32 * 0.12) as u8,
            (full_color.b() as f32 * 0.12) as u8,
        );
        let color = lerp_color(dim_color, full_color, pulse);

        // Downbeat dot is slightly larger
        let r = if i == 0 { DOT_R + 1.0 } else { DOT_R };
        painter.circle_filled(center, r, color);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}
