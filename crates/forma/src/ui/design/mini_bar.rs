//! MiniBar component — Layer 3.
//!
//! A compact value bar (horizontal or vertical) for inline parameter
//! display. Used in the sequencer for velocity, probability, and pitch
//! per step. Lighter and more flexible than Fader / LevelMeter — exactly
//! one rectangle, no thumb, no labels around it, optional value-text
//! overlay.
//!
//! Interaction modes:
//! - `Absolute` — click/drag position directly sets the value
//!   (used for velocity and probability bars).
//! - `Delta { scale }` — drag delta accumulates; `scale` is units of
//!   value per pixel of drag (used for the pitch bar's per-step drag).

use egui::{
    Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};

use crate::ui::theme::SynthTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiniBarOrientation {
    /// Bar grows left→right.
    Horizontal,
    /// Bar grows bottom→top.
    Vertical,
}

#[derive(Clone, Copy, Debug)]
pub enum MiniBarDrag {
    /// Click/drag pointer position directly sets the value.
    Absolute,
    /// Accumulate drag delta into a caller-managed float. Returned via
    /// the per-call drag-accumulator pattern; pass an `&mut f32` that
    /// persists between frames (e.g. into your sequencer step's
    /// `drag_accum` field).
    Delta {
        /// Units of value per pixel of drag (typically ~0.3 for pitch).
        scale: f32,
    },
}

/// Color picker for the bar fill.
#[derive(Clone, Copy)]
pub enum MiniBarFill {
    /// Single constant color.
    Solid(Color32),
    /// Three-zone color. value < `low_threshold` → `low`,
    /// value < `high_threshold` → `mid`, otherwise `high`.
    Zoned {
        low_threshold: f32,
        high_threshold: f32,
        low: Color32,
        mid: Color32,
        high: Color32,
    },
}

/// Builder for a MiniBar. Configure with `.fill(...)`, `.bg(...)`,
/// `.label(...)`, `.drag(...)`, then call `.show(ui, theme)`.
pub struct MiniBar<'a> {
    value: &'a mut f32,
    range: std::ops::RangeInclusive<f32>,
    orientation: MiniBarOrientation,
    size: Vec2,
    fill: MiniBarFill,
    bg: Option<Color32>,
    label: Option<(String, FontId, Color32)>,
    drag: MiniBarDrag,
    drag_accum: Option<&'a mut f32>,
    border: bool,
}

impl<'a> MiniBar<'a> {
    /// Construct a new bar bound to `value` in `range`, with the given
    /// orientation and pixel size.
    pub fn new(
        value: &'a mut f32,
        range: std::ops::RangeInclusive<f32>,
        orientation: MiniBarOrientation,
        size: Vec2,
    ) -> Self {
        Self {
            value,
            range,
            orientation,
            size,
            fill: MiniBarFill::Solid(Color32::GRAY),
            bg: None,
            label: None,
            drag: MiniBarDrag::Absolute,
            drag_accum: None,
            border: false,
        }
    }

    /// Solid fill color.
    pub fn fill(mut self, color: Color32) -> Self {
        self.fill = MiniBarFill::Solid(color);
        self
    }

    /// 3-zone fill — low / mid / high colors with the given thresholds.
    pub fn zoned(
        mut self,
        low_threshold: f32,
        high_threshold: f32,
        low: Color32,
        mid: Color32,
        high: Color32,
    ) -> Self {
        self.fill = MiniBarFill::Zoned {
            low_threshold,
            high_threshold,
            low,
            mid,
            high,
        };
        self
    }

    /// Override the background color. Defaults to `theme.bg_sunken`.
    pub fn bg(mut self, color: Color32) -> Self {
        self.bg = Some(color);
        self
    }

    /// Centered text overlay (e.g. velocity number, note name).
    pub fn label(mut self, text: impl Into<String>, font: FontId, color: Color32) -> Self {
        self.label = Some((text.into(), font, color));
        self
    }

    /// Use absolute-position dragging (the default).
    pub fn drag_absolute(mut self) -> Self {
        self.drag = MiniBarDrag::Absolute;
        self
    }

    /// Use delta-drag mode with caller-owned accumulator. `scale` is
    /// units of value per pixel of drag.
    pub fn drag_delta(mut self, accum: &'a mut f32, scale: f32) -> Self {
        self.drag = MiniBarDrag::Delta { scale };
        self.drag_accum = Some(accum);
        self
    }

    /// Draw a `stroke_ui` border around the bar (off by default).
    pub fn border(mut self, on: bool) -> Self {
        self.border = on;
        self
    }

    /// Render the bar and return its interaction Response.
    pub fn show(self, ui: &mut Ui, theme: &SynthTheme) -> Response {
        // Use an explicit auto-ID (egui's built-in widget idiom) so multiple
        // MiniBars in the same parent layout — e.g. velocity + probability
        // across all sequencer step columns — don't collide and flash the
        // debug ID-collision overlay on click.
        let id = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        let rect = ui.allocate_space(self.size).1;
        let response = ui.interact(rect, id, Sense::click_and_drag());
        let span = *self.range.end() - *self.range.start();

        match self.drag {
            MiniBarDrag::Absolute => {
                if response.dragged() || response.clicked() {
                    if let Some(p) = response.interact_pointer_pos() {
                        let t = match self.orientation {
                            MiniBarOrientation::Horizontal => {
                                ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0)
                            }
                            MiniBarOrientation::Vertical => {
                                ((rect.bottom() - p.y) / rect.height()).clamp(0.0, 1.0)
                            }
                        };
                        *self.value =
                            (*self.range.start() + t * span).clamp(*self.range.start(), *self.range.end());
                    }
                }
            }
            MiniBarDrag::Delta { scale } => {
                if let Some(accum) = self.drag_accum {
                    if response.dragged() {
                        let delta = match self.orientation {
                            MiniBarOrientation::Horizontal => response.drag_delta().x,
                            MiniBarOrientation::Vertical => -response.drag_delta().y,
                        };
                        *accum += delta * scale;
                        let steps = *accum as i32;
                        if steps != 0 {
                            *accum -= steps as f32;
                            *self.value = (*self.value + steps as f32)
                                .clamp(*self.range.start(), *self.range.end());
                        }
                    }
                    if response.drag_stopped() {
                        *accum = 0.0;
                    }
                }
            }
        }

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let painter = ui.painter_at(rect);
        let rounding = CornerRadius::same(theme.rounding_xs as u8);
        let bg = self.bg.unwrap_or_else(|| theme.c(&theme.bg_sunken));
        painter.rect_filled(rect, rounding, bg);

        let t = if span > 0.0 {
            ((*self.value - *self.range.start()) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if t > 0.0 {
            let fill_color = match self.fill {
                MiniBarFill::Solid(c) => c,
                MiniBarFill::Zoned {
                    low_threshold,
                    high_threshold,
                    low,
                    mid,
                    high,
                } => {
                    if *self.value < low_threshold {
                        low
                    } else if *self.value < high_threshold {
                        mid
                    } else {
                        high
                    }
                }
            };
            let fill_rect = match self.orientation {
                MiniBarOrientation::Horizontal => {
                    let w = rect.width() * t;
                    Rect::from_min_size(rect.min, Vec2::new(w, rect.height()))
                }
                MiniBarOrientation::Vertical => {
                    let h = rect.height() * t;
                    Rect::from_min_size(
                        Pos2::new(rect.left(), rect.bottom() - h),
                        Vec2::new(rect.width(), h),
                    )
                }
            };
            painter.rect_filled(fill_rect, rounding, fill_color);
        }

        if self.border {
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
                StrokeKind::Inside,
            );
        }

        if let Some((text, font, color)) = self.label {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &text,
                font,
                color,
            );
        }

        response
    }
}
