//! Horizontal Slider component — Layer 3.
//!
//! Distinct from `Fader`: this is the inline-parameter-row control used in
//! the FX Chain and similar panels — label on the left, filled track in
//! the middle, formatted value on the right, all in a single row.
//!
//! Fader is for channel-strip volumes (big track, separate value display);
//! Slider is for parameter rows (compact, inline value, formatter-friendly).
//!
//! Visual language matches Fader: `bg_sunken` track with `border` stroke,
//! `accent_dim` filled portion, thumb that brightens on hover and uses
//! `accent` when dragged.

use egui::{CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::ui::theme::SynthTheme;

/// Builder for a horizontal parameter slider. Configure with the
/// `.suffix(...)` / `.logarithmic(...)` / `.formatter(...)` chain, then
/// call `.show(ui, theme)`.
pub struct Slider<'a> {
    value: &'a mut f32,
    range: std::ops::RangeInclusive<f32>,
    label: &'a str,
    suffix: Option<&'a str>,
    logarithmic: bool,
    formatter: Option<Box<dyn Fn(f32) -> String + 'a>>,
    decimals: usize,
}

impl<'a> Slider<'a> {
    /// Construct a new slider for `value` clamped to `range`, with the
    /// given inline label.
    pub fn new(value: &'a mut f32, range: std::ops::RangeInclusive<f32>, label: &'a str) -> Self {
        Self {
            value,
            range,
            label,
            suffix: None,
            logarithmic: false,
            formatter: None,
            decimals: 2,
        }
    }

    /// Append a static suffix to the numeric readout (e.g. `" Hz"`).
    pub fn suffix(mut self, s: &'a str) -> Self {
        self.suffix = Some(s);
        self
    }

    /// Map drag position logarithmically. Requires `range.start() > 0.0`.
    pub fn logarithmic(mut self, b: bool) -> Self {
        self.logarithmic = b;
        self
    }

    /// Provide a custom value-to-string formatter; overrides the default
    /// `{:.<decimals>}` rendering and any suffix.
    pub fn formatter(mut self, f: impl Fn(f32) -> String + 'a) -> Self {
        self.formatter = Some(Box::new(f));
        self
    }

    /// Number of decimals in the default numeric readout. Ignored when a
    /// custom formatter is set.
    pub fn decimals(mut self, n: usize) -> Self {
        self.decimals = n;
        self
    }

    /// Render the slider. Returns the track's `Response`; chain
    /// `.on_hover_text(...)` on it for the tooltip.
    pub fn show(self, ui: &mut Ui, theme: &SynthTheme) -> Response {
        let track_h = 6.0;
        let thumb_w = 6.0;
        let thumb_h = 14.0;
        // Row height — driven by the thumb so click targets stay generous.
        let row_h = thumb_h + 2.0;

        let span = *self.range.end() - *self.range.start();

        // Value text first so we can reserve width for it.
        let value_text = if let Some(f) = &self.formatter {
            f(*self.value)
        } else if let Some(sfx) = self.suffix {
            format!("{:.*}{}", self.decimals, *self.value, sfx)
        } else {
            format!("{:.*}", self.decimals, *self.value)
        };

        // Measure label and value text widths so we can give the track
        // the remaining horizontal space.
        let label_font = theme.font_body();
        let value_font = theme.font_value();
        let label_galley = ui.painter().layout_no_wrap(
            self.label.to_string(),
            label_font.clone(),
            theme.c(&theme.text_secondary),
        );
        let value_galley = ui.painter().layout_no_wrap(
            value_text.clone(),
            value_font.clone(),
            theme.c(&theme.text_primary),
        );
        let label_w = label_galley.size().x + theme.sp_xs;
        let value_w = value_galley.size().x + theme.sp_xs;

        let available_w = ui.available_width();
        let min_track_w = 60.0;
        let track_w = (available_w - label_w - value_w).max(min_track_w);
        let row_w = label_w + track_w + value_w;

        let (row_rect, _) =
            ui.allocate_exact_size(Vec2::new(row_w, row_h), Sense::hover());

        // Label — vertically centered, left-aligned.
        ui.painter_at(row_rect).text(
            Pos2::new(row_rect.left(), row_rect.center().y),
            egui::Align2::LEFT_CENTER,
            self.label,
            label_font,
            theme.c(&theme.text_secondary),
        );

        // Track rect — vertically centered, with the rest of the row.
        let track_left = row_rect.left() + label_w;
        let track_top = row_rect.center().y - track_h * 0.5;
        let track_rect =
            Rect::from_min_size(Pos2::new(track_left, track_top), Vec2::new(track_w, track_h));

        // Hit-test rect — larger than the visible track so the thumb is
        // easy to grab vertically.
        let hit_rect = Rect::from_min_size(
            Pos2::new(track_left, row_rect.top()),
            Vec2::new(track_w, row_h),
        );
        let response = ui.interact(
            hit_rect,
            ui.id().with(("synth_slider", self.label)),
            Sense::click_and_drag(),
        );

        let log_t = |v: f32| -> f32 {
            if self.logarithmic && *self.range.start() > 0.0 {
                (v / *self.range.start()).ln() / (*self.range.end() / *self.range.start()).ln()
            } else {
                (v - *self.range.start()) / span
            }
        };
        let log_v = |t: f32| -> f32 {
            if self.logarithmic && *self.range.start() > 0.0 {
                *self.range.start() * (*self.range.end() / *self.range.start()).powf(t)
            } else {
                *self.range.start() + t * span
            }
        };

        // Click anywhere in the track to jump there.
        if response.clicked() || response.dragged() {
            if let Some(p) = response.interact_pointer_pos() {
                let t = ((p.x - track_rect.left()) / track_rect.width()).clamp(0.0, 1.0);
                *self.value = log_v(t).clamp(*self.range.start(), *self.range.end());
            }
        }
        if response.double_clicked() {
            *self.value = log_v(0.5);
        }

        // ── Draw ──────────────────────────────────────────────────────
        if ui.is_rect_visible(track_rect) {
            let painter = ui.painter_at(row_rect);
            let rounding = CornerRadius::same(theme.rounding_sm as u8);

            // Track background.
            painter.rect_filled(track_rect, rounding, theme.c(&theme.bg_sunken));
            painter.rect_stroke(
                track_rect,
                rounding,
                Stroke::new(theme.stroke_ui, theme.c(&theme.border)),
                StrokeKind::Inside,
            );

            // Filled portion.
            let t = log_t(*self.value).clamp(0.0, 1.0);
            let fill_w = track_w * t;
            if fill_w > 0.0 {
                let fill_rect =
                    Rect::from_min_size(track_rect.min, Vec2::new(fill_w, track_h));
                painter.rect_filled(fill_rect, rounding, theme.c(&theme.accent_dim));
            }

            // Thumb.
            let thumb_color = if response.dragged() {
                theme.c(&theme.accent)
            } else if response.hovered() {
                theme.c(&theme.text_primary)
            } else {
                theme.c(&theme.text_secondary)
            };
            let thumb_x = track_rect.left() + fill_w;
            let thumb_cx = thumb_x.clamp(
                track_rect.left() + thumb_w * 0.5,
                track_rect.right() - thumb_w * 0.5,
            );
            let thumb_rect = Rect::from_center_size(
                Pos2::new(thumb_cx, row_rect.center().y),
                Vec2::new(thumb_w, thumb_h),
            );
            painter.rect_filled(thumb_rect, rounding, thumb_color);

            // Value — right-aligned at the row's right edge.
            painter.text(
                Pos2::new(row_rect.right(), row_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                &value_text,
                value_font,
                theme.c(&theme.text_primary),
            );
        }

        response
    }
}
