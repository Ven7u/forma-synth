//! 8-band parametric EQ panel — draggable dots on a frequency/gain canvas.

use crate::eq::{response_curve_db, BandType, EqParams, BAND_COUNT};
use crate::ui::design::chip::color_chip;
use crate::ui::frame::SynthFrame;
use crate::ui::theme::SynthTheme;
use crate::SynthApp;
use eframe::egui;
use egui::{Color32, Pos2, Rect, RichText, Stroke, Vec2};

// Frequency axis: 20 Hz – 20 kHz (log scale)
const FREQ_MIN: f32 = 20.0;
const FREQ_MAX: f32 = 20_000.0;
// Gain axis: ±18 dB
const GAIN_MIN: f32 = -18.0;
const GAIN_MAX: f32 = 18.0;

const SAMPLE_RATE: f32 = 44100.0;

// ── Theme helpers ─────────────────────────────────────────────────────────────

/// Per-band dot color, drawn from the theme's FX palette so it adapts to any theme.
fn band_color(i: usize, t: &SynthTheme, enabled: bool) -> Color32 {
    let rgb = match i {
        0 => &t.accent_fm,       // Low Shelf  — blue
        1 => &t.fx_chorus,       // Peak 200Hz — green
        2 => &t.accent_hold,     // Peak 500Hz — yellow
        3 => &t.fx_overdrive,    // Peak 1kHz  — orange
        4 => &t.fx_distortion,   // Peak 2.5kHz— red
        5 => &t.fx_reverb,       // Peak 5kHz  — violet
        6 => &t.fx_crystallizer, // Peak 10kHz — amber
        _ => &t.accent,          // High Shelf — main teal
    };
    let c = t.c(rgb);
    if enabled {
        c
    } else {
        c.gamma_multiply(0.25)
    }
}

// ── Coordinate helpers ────────────────────────────────────────────────────────

fn freq_to_x(rect: &Rect, freq: f32) -> f32 {
    let t = (freq.ln() - FREQ_MIN.ln()) / (FREQ_MAX.ln() - FREQ_MIN.ln());
    rect.left() + t * rect.width()
}

fn gain_to_y(rect: &Rect, gain: f32) -> f32 {
    let t = 1.0 - (gain - GAIN_MIN) / (GAIN_MAX - GAIN_MIN);
    rect.top() + t * rect.height()
}

fn dot_pos(rect: &Rect, freq: f32, gain: f32) -> Pos2 {
    Pos2::new(freq_to_x(rect, freq), gain_to_y(rect, gain))
}

// ── EQ Panel ──────────────────────────────────────────────────────────────────

impl SynthApp {
    pub fn ui_eq_panel(&mut self, ui: &mut egui::Ui) {
        let frame = SynthFrame::section(&self.theme);
        frame.show(ui, |ui| {
            let mut params = match self.eq.lock() {
                Ok(p) => p.clone(),
                Err(_) => return,
            };
            let mut changed = false;

            // ── Header row ───────────────────────────────────────────────────
            ui.horizontal(|ui| {
                let label = RichText::new("EQ")
                    .small()
                    .strong()
                    .color(if params.enabled {
                        self.theme.c(&self.theme.accent)
                    } else {
                        self.theme.c(&self.theme.text_disabled)
                    });
                if ui
                    .button(label)
                    .on_hover_text("Toggle mix-bus EQ on/off")
                    .clicked()
                {
                    params.enabled = !params.enabled;
                    changed = true;
                }

                ui.separator();

                for i in 0..BAND_COUNT {
                    let b = &mut params.bands[i];
                    let short = match b.band_type {
                        BandType::LowShelf => "LS",
                        BandType::HighShelf => "HS",
                        BandType::Peak => match i {
                            1 => "P1",
                            2 => "P2",
                            3 => "P3",
                            4 => "P4",
                            5 => "P5",
                            _ => "P6",
                        },
                    };
                    let col = band_color(i, &self.theme, true);
                    if color_chip(ui, short, col, b.enabled, &self.theme)
                        .on_hover_text(band_hover(i, b))
                        .clicked()
                    {
                        b.enabled = !b.enabled;
                        changed = true;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("Reset")
                        .on_hover_text("Reset all bands to 0 dB")
                        .clicked()
                    {
                        for b in params.bands.iter_mut() {
                            b.gain_db = 0.0;
                        }
                        changed = true;
                    }
                });
            });

            ui.add_space(self.theme.sp_xs);

            // ── Canvas ───────────────────────────────────────────────────────
            let canvas_h = ui.available_height().max(80.0);
            let canvas_size = Vec2::new(ui.available_width(), canvas_h);
            let (rect, response) =
                ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());

            if !ui.is_rect_visible(rect) {
                if changed {
                    if let Ok(mut p) = self.eq.lock() {
                        *p = params;
                    }
                }
                return;
            }

            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, self.theme.rounding_sm, self.theme.c(&self.theme.scope_bg));
            draw_grid(&painter, rect, &self.theme);
            if params.enabled {
                draw_response_curve(&painter, rect, &params, &self.theme);
            }

            // ── Band dots ────────────────────────────────────────────────────
            let dot_r = 7.0_f32;
            let mut hovered_band: Option<usize> = None;

            for i in 0..BAND_COUNT {
                let (freq, gain_db, q, enabled, band_type) = {
                    let b = &params.bands[i];
                    (b.freq, b.gain_db, b.q, b.enabled, b.band_type)
                };
                let pos = dot_pos(&rect, freq, gain_db);
                let col = band_color(i, &self.theme, enabled);

                let dot_rect = Rect::from_center_size(pos, Vec2::splat(dot_r * 2.5));
                if let Some(ptr) = response.hover_pos() {
                    if dot_rect.contains(ptr) {
                        hovered_band = Some(i);
                    }
                }

                let band_id = response.id.with(i);
                let drag_resp = ui.interact(dot_rect, band_id, egui::Sense::click_and_drag());

                if drag_resp.dragged() {
                    let delta = drag_resp.drag_delta();
                    let log_min = FREQ_MIN.ln();
                    let log_max = FREQ_MAX.ln();
                    let new_log = (freq.ln() + delta.x / rect.width() * (log_max - log_min))
                        .clamp(log_min, log_max);
                    params.bands[i].freq = new_log.exp();
                    params.bands[i].gain_db = (gain_db
                        - delta.y / rect.height() * (GAIN_MAX - GAIN_MIN))
                        .clamp(GAIN_MIN, GAIN_MAX);
                    changed = true;
                }

                if drag_resp.hovered() {
                    let scroll = ui.input(|inp| inp.smooth_scroll_delta.y);
                    if scroll.abs() > 0.1 {
                        params.bands[i].q = (q * (1.0 + scroll * 0.02)).clamp(0.1, 10.0);
                        changed = true;
                    }
                }

                if drag_resp.double_clicked() {
                    params.bands[i].gain_db = 0.0;
                    changed = true;
                }

                let is_active = drag_resp.dragged() || drag_resp.hovered();
                let r = if is_active { dot_r + 2.0 } else { dot_r };
                painter.circle_filled(pos, r, col);
                let ring_col = if is_active {
                    self.theme.c(&self.theme.text_primary)
                } else {
                    self.theme.c(&self.theme.text_primary).gamma_multiply(0.4)
                };
                painter.circle_stroke(pos, r, Stroke::new(self.theme.stroke_focus, ring_col));

                let short = band_short_label(i, &band_type);
                painter.text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    short,
                    self.theme.font_small(),
                    self.theme.c(&self.theme.bg_sunken),
                );
            }

            // Tooltip
            if let Some(i) = hovered_band {
                let (freq, gain_db, q) = {
                    let b = &params.bands[i];
                    (b.freq, b.gain_db, b.q)
                };
                let freq_str = if freq >= 1000.0 {
                    format!("{:.1}k", freq / 1000.0)
                } else {
                    format!("{:.0}", freq)
                };
                response.clone().on_hover_text(format!(
                    "Band {} | {} Hz | {:.1} dB | Q {:.2}\nDrag: freq/gain  •  Scroll: Q  •  Double-click: reset",
                    i + 1, freq_str, gain_db, q
                ));
            }

            draw_axis_labels(&painter, rect, &self.theme);

            if changed {
                if let Ok(mut p) = self.eq.lock() {
                    *p = params;
                }
            }
        });
    }
}

// ── Drawing helpers ───────────────────────────────────────────────────────────

fn draw_grid(painter: &egui::Painter, rect: Rect, t: &SynthTheme) {
    // On the dark CRT bg, derive grid lines from accent so every theme gets a
    // matching-hue grid (amber in Classic, teal in Phosphor, etc.).
    let a = t.c(&t.accent);
    let grid_col = Color32::from_rgba_premultiplied(a.r() / 4, a.g() / 4, a.b() / 4, 110);
    let zero_col = Color32::from_rgba_premultiplied(a.r() / 2, a.g() / 2, a.b() / 2, 150);

    for &freq in &[
        50.0_f32, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0,
    ] {
        let x = freq_to_x(&rect, freq);
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(t.stroke_ui, grid_col),
        );
    }

    for &gain in &[-12.0_f32, -6.0, 0.0, 6.0, 12.0] {
        let y = gain_to_y(&rect, gain);
        let (col, w) = if gain == 0.0 {
            (zero_col, t.stroke_focus)
        } else {
            (grid_col, t.stroke_ui)
        };
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(w, col),
        );
    }
}

fn draw_response_curve(painter: &egui::Painter, rect: Rect, params: &EqParams, t: &SynthTheme) {
    let n = rect.width() as usize;
    if n < 2 {
        return;
    }
    let db_vals = response_curve_db(params, SAMPLE_RATE, n);

    let points: Vec<Pos2> = db_vals
        .iter()
        .enumerate()
        .map(|(i, &db)| {
            let x = rect.left() + i as f32 / (n - 1) as f32 * rect.width();
            let y = gain_to_y(&rect, db.clamp(GAIN_MIN, GAIN_MAX));
            Pos2::new(x, y)
        })
        .collect();

    // Trapezoid-strip fill between curve and 0 dB — always convex per strip.
    let zero_y = gain_to_y(&rect, 0.0);
    let accent = t.c(&t.accent);
    // Derived from accent token with reduced brightness/alpha — no token applies.
    let fill_col =
        Color32::from_rgba_premultiplied(accent.r() / 5, accent.g() / 5, accent.b() / 5, 90);
    for w in points.windows(2) {
        let quad = vec![
            w[0],
            w[1],
            Pos2::new(w[1].x, zero_y),
            Pos2::new(w[0].x, zero_y),
        ];
        painter.add(egui::Shape::convex_polygon(quad, fill_col, Stroke::NONE));
    }

    // Curve line — derived from accent with alpha; stroke_active gives the right weight.
    let line_col = Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 200);
    painter.add(egui::Shape::line(points, Stroke::new(t.stroke_active, line_col)));
}

fn draw_axis_labels(painter: &egui::Painter, rect: Rect, t: &SynthTheme) {
    let a = t.c(&t.accent);
    let col = Color32::from_rgba_premultiplied(a.r(), a.g(), a.b(), 145);
    let font = t.font_body();

    for &(freq, label) in &[
        (50.0_f32, "50"),
        (100.0, "100"),
        (200.0, "200"),
        (500.0, "500"),
        (1000.0, "1k"),
        (2000.0, "2k"),
        (5000.0, "5k"),
        (10000.0, "10k"),
    ] {
        let x = freq_to_x(&rect, freq);
        painter.text(
            Pos2::new(x, rect.bottom() - 12.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            font.clone(),
            col,
        );
    }

    for &(gain, label) in &[(-12.0_f32, "-12"), (-6.0, "-6"), (6.0, "+6"), (12.0, "+12")] {
        let y = gain_to_y(&rect, gain);
        painter.text(
            Pos2::new(rect.left() + 3.0, y),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            col,
        );
    }
}

fn band_short_label(i: usize, bt: &BandType) -> &'static str {
    match *bt {
        BandType::LowShelf => "L",
        BandType::HighShelf => "H",
        BandType::Peak => match i {
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            _ => "6",
        },
    }
}

fn band_hover(i: usize, b: &crate::eq::BandParams) -> String {
    let type_str = match b.band_type {
        BandType::LowShelf => "Low Shelf",
        BandType::HighShelf => "High Shelf",
        BandType::Peak => "Peak",
    };
    let freq_str = if b.freq >= 1000.0 {
        format!("{:.1}k Hz", b.freq / 1000.0)
    } else {
        format!("{:.0} Hz", b.freq)
    };
    format!(
        "Band {} — {} @ {}, {:.1} dB, Q={:.2}\nClick to toggle on/off",
        i + 1,
        type_str,
        freq_str,
        b.gain_db,
        b.q
    )
}
