//! Beat indicator — Phase 7 design system component.
//!
//! Draws the time-signature label + two pulse dots into an allocated rect:
//!   • Dot 1 (beat 1): pulses with `accent` color
//!   • Dot 2 (beats 2+): pulses with `accent_beat` color
//!
//! All colors come from theme tokens; no hardcoded values.

use egui::{Align2, Color32, FontId, Pos2, Rect, Response, Sense, Vec2};

use crate::ui::theme::SynthTheme;

const DOT_R: f32 = 3.5;

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

/// Allocates a fixed-width rect and draws the beat indicator into it.
///
/// Returns the `Response` so the caller can attach `.on_hover_text()` and a
/// click handler for opening the metronome settings popover.
pub fn draw_beat_indicator(ui: &mut egui::Ui, state: &BeatState, theme: &SynthTheme) -> Response {
    // Fixed layout: 30 px sig text + 5 px gap + dot + 4 px gap + dot
    let total_w = 30.0 + 5.0 + DOT_R * 2.0 + 4.0 + DOT_R * 2.0;
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
        Pos2::new(rect.left() + 15.0, cy),
        Align2::CENTER_CENTER,
        format!("{}/{}", state.beats, state.denom),
        FontId::monospace(10.0),
        sig_col,
    );

    // ── Dot envelope: power-2 decay from beat start ───────────────────────
    let accent_t = if state.active && state.beat_idx == 0 {
        (1.0_f32 - state.beat_frac).powf(2.2)
    } else {
        0.0
    };
    let beat_t = if state.active && state.beat_idx > 0 {
        (1.0_f32 - state.beat_frac).powf(2.2)
    } else {
        0.0
    };

    // ── Dot 1: accent color (beat 1) ──────────────────────────────────────
    let accent_full = theme.c(&theme.accent);
    let accent_dim = dim_color(accent_full, 0.12);
    let dot1_x = rect.left() + 30.0 + 5.0 + DOT_R;
    painter.circle_filled(
        Pos2::new(dot1_x, cy),
        DOT_R,
        lerp_color(accent_dim, accent_full, accent_t),
    );

    // ── Dot 2: accent_beat color (beats 2+) ──────────────────────────────
    let beat_full = theme.c(&theme.accent_beat);
    let beat_dim = dim_color(beat_full, 0.12);
    let dot2_x = dot1_x + DOT_R * 2.0 + 4.0;
    painter.circle_filled(
        Pos2::new(dot2_x, cy),
        DOT_R,
        lerp_color(beat_dim, beat_full, beat_t),
    );
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

fn dim_color(c: Color32, factor: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * factor) as u8,
        (c.g() as f32 * factor) as u8,
        (c.b() as f32 * factor) as u8,
    )
}
