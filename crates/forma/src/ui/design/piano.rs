//! Piano component — Layer 3.
//!
//! A chromatic piano keyboard spanning any MIDI range. Handles two sizes
//! (Full / Preview), optional click-and-drag interaction, scale / range-bar
//! highlight, and octave labels. All colors come from `key_*` theme tokens.
//!
//! The caller supplies a `key_state` callback that returns a `KeyVisualState`
//! for each MIDI note; the component maps that to theme colors and draws the
//! keys in two passes (white first, then black on top).

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

// ─── Public types ────────────────────────────────────────────────────────────

/// White-key height variants. Black keys are always 60 % of the white height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PianoSize {
    /// 64 px white keys — full interactive piano.
    Full,
    /// 36 px white keys — compact chord-KB preview strip.
    Preview,
}

impl PianoSize {
    pub fn white_key_height(self) -> f32 {
        match self {
            Self::Full => 64.0,
            Self::Preview => 36.0,
        }
    }
}

/// Visual state for a single MIDI note, returned by the `key_state` callback.
/// Default gives a plain unlit key.
#[derive(Clone, Copy, Default)]
pub struct KeyVisualState {
    /// Note is currently depressed (sounding).
    pub pressed: bool,
    /// Note falls within the computer-keyboard mapped range.
    pub in_kb_range: bool,
    /// Note's pitch class is the scale root.
    pub is_scale_root: bool,
    /// Note's pitch class is in the highlighted scale.
    pub in_scale: bool,
}

impl KeyVisualState {
    /// Theme-driven fill color for a white key.
    pub fn white_fill(self, theme: &SynthTheme) -> Color32 {
        if self.pressed {
            theme.c(&theme.key_white_pressed)
        } else if self.is_scale_root {
            theme.c(&theme.key_scale_root)
        } else if self.in_scale {
            theme.c(&theme.key_scale_in)
        } else if self.in_kb_range {
            theme.c(&theme.key_white_range)
        } else {
            theme.c(&theme.key_white_default)
        }
    }

    /// Theme-driven fill color for a black key.
    pub fn black_fill(self, theme: &SynthTheme) -> Color32 {
        if self.pressed {
            theme.c(&theme.key_black_pressed)
        } else if self.is_scale_root {
            theme.c(&theme.key_scale_root_dark)
        } else if self.in_scale {
            theme.c(&theme.key_scale_in_dark)
        } else if self.in_kb_range {
            theme.c(&theme.key_black_range)
        } else {
            theme.c(&theme.key_black_default)
        }
    }
}

/// Configuration for a piano render.
pub struct PianoConfig {
    /// First MIDI note (inclusive).
    pub first_midi: u8,
    /// Last MIDI note (inclusive).
    pub last_midi: u8,
    /// Key size — drives the white-key height.
    pub size: PianoSize,
    /// Respond to clicks and drags (fills `PianoResult::pointer_midi`).
    pub interactive: bool,
    /// Show "C4" octave labels at the bottom of each C key.
    pub show_labels: bool,
    /// Draw a colored accent bar at the top spanning `(start, end_exclusive)`.
    pub range_bar: Option<(u8, u8)>,
}

/// Result of a `piano()` call.
pub struct PianoResult {
    pub response: Response,
    /// MIDI note under the pointer, or `None`. Populated on hover (preview) or
    /// on pointer-down (interactive). Black keys take priority over white keys.
    pub pointer_midi: Option<u8>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Returns `true` if `midi` maps to a white key.
pub fn is_white_key(midi: u8) -> bool {
    matches!(midi % 12, 0 | 2 | 4 | 5 | 7 | 9 | 11)
}

/// Count the white keys in `[first, last]` (inclusive).
pub fn count_white_keys_in(first: u8, last: u8) -> usize {
    (first..=last).filter(|&m| is_white_key(m)).count()
}

// ─── Component ───────────────────────────────────────────────────────────────

/// Render a piano keyboard.
///
/// `key_state` is called once per MIDI note in the configured range and
/// returns the visual state for that key. Use `KeyVisualState::default()` for
/// a plain unlit key.
pub fn piano(
    ui: &mut Ui,
    config: &PianoConfig,
    key_state: &dyn Fn(u8) -> KeyVisualState,
    theme: &SynthTheme,
) -> PianoResult {
    let white_h = config.size.white_key_height();
    let black_h = white_h * 0.60;
    let num_white = count_white_keys_in(config.first_midi, config.last_midi);
    let avail_w = ui.available_width();
    let white_w = (avail_w / num_white as f32).max(6.0);
    let black_w = white_w * 0.62;
    let total_w = white_w * num_white as f32;

    let sense = if config.interactive {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let (resp, painter) = ui.allocate_painter(Vec2::new(total_w, white_h + 4.0), sense);
    let origin = resp.rect.left_top();
    let pointer_pos = resp.interact_pointer_pos();
    let mut pointer_midi: Option<u8> = None;

    let rounding_white = CornerRadius::same(theme.rounding_xs as u8);
    // Black keys get half the rounding — less square, not fully round.
    let rounding_black = CornerRadius::same((theme.rounding_xs * 0.5) as u8);
    // Hairline dividers between adjacent white keys — intentionally thinner
    // than stroke_ui to avoid visual clutter at narrow key widths.
    let sep_stroke = Stroke::new(theme.stroke_ui * 0.5, theme.c(&theme.key_stroke));
    let accent = theme.c(&theme.accent);
    let label_col = theme.c(&theme.key_label);
    let label_size = if white_w > 12.0 {
        match config.size {
            PianoSize::Full => 8.0,
            PianoSize::Preview => 7.0,
        }
    } else {
        6.0
    };

    // Build the x-position table for white keys (used by black-key and range-bar passes).
    let mut white_key_x: [f32; 128] = [0.0; 128];
    let mut white_x = 0.0_f32;

    // ── Pass 1: white keys ───────────────────────────────────────────────────
    for midi in config.first_midi..=config.last_midi {
        if !is_white_key(midi) {
            continue;
        }
        white_key_x[midi as usize] = white_x;
        let rect = Rect::from_min_size(
            origin + Vec2::new(white_x + 0.5, 1.0),
            Vec2::new(white_w - 1.0, white_h - 2.0),
        );
        white_x += white_w;

        let state = key_state(midi);
        painter.rect_filled(rect, rounding_white, state.white_fill(theme));
        painter.rect_stroke(rect, rounding_white, sep_stroke, StrokeKind::Middle);

        if config.show_labels && midi % 12 == 0 {
            let octave = (midi / 12) as i32 - 1;
            painter.text(
                Pos2::new(rect.center().x, rect.bottom() - 3.0),
                egui::Align2::CENTER_BOTTOM,
                format!("C{octave}"),
                egui::FontId::proportional(label_size),
                label_col,
            );
        }

        if let Some(pos) = pointer_pos {
            if rect.contains(pos) {
                pointer_midi = Some(midi);
            }
        }
    }

    // ── Pass 2: black keys (on top of white) ────────────────────────────────
    for midi in config.first_midi..=config.last_midi {
        if is_white_key(midi) || midi == 0 || !is_white_key(midi - 1) {
            continue;
        }
        let x = white_key_x[(midi - 1) as usize] + white_w * 0.6;
        let rect = Rect::from_min_size(origin + Vec2::new(x, 1.0), Vec2::new(black_w, black_h));

        let state = key_state(midi);
        painter.rect_filled(rect, rounding_black, state.black_fill(theme));

        if let Some(pos) = pointer_pos {
            if rect.contains(pos) {
                pointer_midi = Some(midi); // black keys shadow white keys
            }
        }
    }

    // ── Pass 3: range bracket bar ────────────────────────────────────────────
    if let Some((bar_start, bar_end)) = config.range_bar {
        let mut range_left = f32::MAX;
        let mut range_right = 0.0_f32;
        for midi in bar_start..bar_end.min(config.last_midi + 1) {
            if midi < config.first_midi {
                continue;
            }
            if is_white_key(midi) {
                let x = white_key_x[midi as usize];
                range_left = range_left.min(x);
                range_right = range_right.max(x + white_w);
            } else if midi > 0 && is_white_key(midi - 1) {
                let x = white_key_x[(midi - 1) as usize] + white_w * 0.6;
                range_left = range_left.min(x);
                range_right = range_right.max(x + black_w);
            }
        }
        if range_left < range_right {
            let bar = Rect::from_min_size(
                origin + Vec2::new(range_left, 0.0),
                Vec2::new(range_right - range_left, 2.5),
            );
            painter.rect_filled(bar, rounding_black, accent);
        }
    }

    PianoResult {
        response: resp,
        pointer_midi,
    }
}
