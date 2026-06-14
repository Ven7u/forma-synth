//! LevelMeter component — Layer 3.
//!
//! Per `04-components.md` §LevelMeter: a vertical or horizontal bar
//! representing audio level. Three color zones (green / warn / clip)
//! mapped to fixed dB-ish breakpoints, with an optional peak-hold line
//! that decays over time. Paired with `Fader` in the `FaderColumn`
//! pattern; standalone for a master output meter.

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelMeterOrientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelMeterSize {
    /// Large — paired with a Large Fader (`fader_h_lg`, 120 px). Used in
    /// master sections and Tier 1 channel strips.
    Large,
    /// Standard — paired with a Standard Fader. Matches `fader_h_md` (80 px).
    Standard,
    /// Compact — meter-bridge style, ~48 px.
    Small,
}

impl LevelMeterSize {
    pub fn length(self) -> f32 {
        match self {
            LevelMeterSize::Large => 120.0,
            LevelMeterSize::Standard => 80.0,
            LevelMeterSize::Small => 48.0,
        }
    }
}

/// Track width perpendicular to the bar direction.
const TRACK_WIDTH: f32 = 6.0;

/// Color-zone breakpoints. 0.7 = "warn" onset; 1.0 = clip.
const WARN_THRESHOLD: f32 = 0.7;
const CLIP_THRESHOLD: f32 = 1.0;

/// Render a level meter. `level` and `peak_hold` are 0..=1 normalized.
/// Pass `peak_hold = 0.0` (or any value < `level`) to skip the peak line.
pub fn level_meter(
    ui: &mut Ui,
    level: f32,
    peak_hold: f32,
    orientation: LevelMeterOrientation,
    size: LevelMeterSize,
    theme: &SynthTheme,
) -> Response {
    let length = size.length();
    let rect_size = match orientation {
        LevelMeterOrientation::Vertical => Vec2::new(TRACK_WIDTH, length),
        LevelMeterOrientation::Horizontal => Vec2::new(length, TRACK_WIDTH),
    };
    let (rect, response) = ui.allocate_exact_size(rect_size, Sense::hover());

    if !ui.is_rect_visible(rect) {
        return response;
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(theme.rounding_xs as u8);

    // Track background.
    painter.rect_filled(rect, rounding, theme.c(&theme.meter_bg));
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
        StrokeKind::Inside,
    );

    let level = level.clamp(0.0, 1.0);
    if level > 0.0 {
        let fill_color = level_color(level, theme);
        let bar_rect = match orientation {
            LevelMeterOrientation::Vertical => {
                let h = length * level;
                Rect::from_min_size(
                    Pos2::new(rect.left(), rect.bottom() - h),
                    Vec2::new(TRACK_WIDTH, h),
                )
            }
            LevelMeterOrientation::Horizontal => {
                let w = length * level;
                Rect::from_min_size(rect.min, Vec2::new(w, TRACK_WIDTH))
            }
        };
        painter.rect_filled(bar_rect, rounding, fill_color);
    }

    // Peak-hold line — only if meaningfully above zero and not below level.
    let peak = peak_hold.clamp(0.0, 1.0);
    if peak > 0.01 {
        let hold_color = if peak >= CLIP_THRESHOLD {
            theme.c(&theme.meter_clip)
        } else {
            theme.c(&theme.text_primary)
        };
        let stroke = Stroke::new(theme.stroke_focus, hold_color);
        match orientation {
            LevelMeterOrientation::Vertical => {
                let y = rect.bottom() - length * peak;
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    stroke,
                );
            }
            LevelMeterOrientation::Horizontal => {
                let x = rect.left() + length * peak;
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    stroke,
                );
            }
        }
    }

    response
}

/// Map a 0..=1 level to a color: green below WARN, lerp green→clip
/// between WARN and CLIP, solid clip at or above CLIP.
fn level_color(level: f32, theme: &SynthTheme) -> Color32 {
    if level < WARN_THRESHOLD {
        theme.c(&theme.meter_green)
    } else if level < CLIP_THRESHOLD {
        let t = (level - WARN_THRESHOLD) / (CLIP_THRESHOLD - WARN_THRESHOLD);
        // Token-derived: interpolation between theme.meter_green and theme.meter_clip.
        let g = theme.meter_green;
        let c = theme.meter_clip;
        Color32::from_rgb(
            (g[0] as f32 + (c[0] as f32 - g[0] as f32) * t) as u8,
            (g[1] as f32 + (c[1] as f32 - g[1] as f32) * t) as u8,
            (g[2] as f32 + (c[2] as f32 - g[2] as f32) * t) as u8,
        )
    } else {
        theme.c(&theme.meter_clip)
    }
}
