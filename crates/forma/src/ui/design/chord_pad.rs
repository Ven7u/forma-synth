//! ChordPad component — Layer 3.
//!
//! Quality-colored full border + `bg_surface` background. Held state fills
//! with the quality color at ~25 % alpha; editing state fills with
//! `accent_hold` at ~15 % alpha. The quality color drives the border in the
//! normal state too so the chord type is immediately legible at a glance.
//!
//! The caller is responsible for interaction logic. This function allocates
//! space, paints, and returns the egui `Response` so the caller can inspect
//! hover / click / drag state.

use egui::{Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

/// Chord quality — drives the left-edge strip color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
}

/// Visual state borrowed for a single render call.
pub struct ChordPadState<'a> {
    pub quality:    ChordQuality,
    pub chord_name: &'a str,
    pub degree:     &'a str,
    pub key_hint:   &'a str,
    pub held:       bool,
    pub editing:    bool,
}

/// Map the suffix returned by `chord_quality()` to a `ChordQuality`.
/// `""` → Major · `"m"` → Minor · `"°"` → Diminished.
pub fn parse_quality(s: &str) -> ChordQuality {
    match s {
        "m" => ChordQuality::Minor,
        "°" => ChordQuality::Diminished,
        _   => ChordQuality::Major,
    }
}

/// Return the theme color that represents a chord quality.
pub fn quality_color(quality: ChordQuality, theme: &SynthTheme) -> Color32 {
    match quality {
        ChordQuality::Major      => theme.c(&theme.accent),
        ChordQuality::Minor      => theme.c(&theme.accent_fm),
        ChordQuality::Diminished => theme.c(&theme.seq_rec_cursor),
    }
}

/// Render one chord pad and return the egui Response.
pub fn chord_pad(
    ui: &mut Ui,
    state: ChordPadState<'_>,
    size: Vec2,
    theme: &SynthTheme,
) -> Response {
    let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
    let r = resp.rect;
    let t = theme;
    let qcol = quality_color(state.quality, t);
    let rounding = CornerRadius::same(t.rounding_md as u8);

    // ── Background ──────────────────────────────────────────────────────────
    let bg = if state.held {
        // Quality color at ~25 % — derived from token via quality_color().
        Color32::from_rgba_premultiplied(
            (qcol.r() as f32 * 0.25) as u8,
            (qcol.g() as f32 * 0.25) as u8,
            (qcol.b() as f32 * 0.25) as u8,
            255,
        )
    } else if state.editing {
        let ah = t.c(&t.accent_hold);
        // accent_hold at ~15 % — derived from token.
        Color32::from_rgba_premultiplied(
            (ah.r() as f32 * 0.15) as u8,
            (ah.g() as f32 * 0.15) as u8,
            (ah.b() as f32 * 0.15) as u8,
            255,
        )
    } else {
        t.c(&t.bg_surface)
    };
    painter.rect_filled(r, rounding, bg);

    // ── Border — quality color in all states ────────────────────────────────
    let (stroke_w, stroke_col) = if state.held {
        (t.stroke_active, qcol)
    } else if state.editing {
        (t.stroke_active, t.c(&t.accent_hold))
    } else if resp.hovered() {
        (t.stroke_focus, t.c(&t.border_focus))
    } else {
        (t.stroke_ui, qcol)
    };
    painter.rect_stroke(r, rounding, Stroke::new(stroke_w, stroke_col), StrokeKind::Middle);

    // ── Text ────────────────────────────────────────────────────────────────
    painter.text(
        egui::pos2(r.center().x, r.top() + 14.0),
        egui::Align2::CENTER_CENTER,
        state.chord_name,
        t.font_heading(),
        t.c(&t.text_primary),
    );
    painter.text(
        egui::pos2(r.center().x, r.top() + 28.0),
        egui::Align2::CENTER_CENTER,
        state.degree,
        t.font_value(),
        t.c(&t.text_secondary),
    );
    // Key hint: bottom-right corner.
    painter.text(
        egui::pos2(r.right() - 5.0, r.bottom() - 4.0),
        egui::Align2::RIGHT_BOTTOM,
        state.key_hint,
        t.font_micro(),
        t.c(&t.text_disabled),
    );

    resp
}
