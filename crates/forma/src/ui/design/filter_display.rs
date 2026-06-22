//! Filter response curve display — Layer 3 design system component.
//!
//! Renders a read-only LP4 Bode magnitude plot inside a given `Rect`.
//! Colors are driven entirely by `filter_curve_*` theme tokens so every
//! theme can choose its own CRT palette (amber for Classic, green for
//! Phosphor, etc.).

use egui::{Color32, CornerRadius, Pos2, Stroke, StrokeKind};

use crate::ui::theme::SynthTheme;

/// Draw a lowpass-4 frequency response curve into `rect`.
///
/// `cutoff`   — frequency in Hz (80..18 000)
/// `q_engine` — resonance in engine range (0.0..0.95)
/// `active`   — true when the parent widget is hovered or dragged (draws accent border)
pub fn draw_lp_response_curve(
    painter: &egui::Painter,
    rect: egui::Rect,
    cutoff: f32,
    q_engine: f32,
    active: bool,
    theme: &SynthTheme,
) {
    const F_MIN: f32 = 80.0;
    const F_MAX: f32 = 18_000.0;
    const DB_MIN: f32 = -60.0;
    const DB_MAX: f32 = 36.0;

    let q_display = 0.5 + (q_engine / 0.95) * 9.5;

    let border_col = if active {
        theme.c(&theme.accent)
    } else {
        theme.c(&theme.border)
    };

    let log_range = (F_MAX / F_MIN).ln();
    let freq_to_t = |f: f32| ((f / F_MIN).ln() / log_range).clamp(0.0, 1.0);
    let sx = |t: f32| rect.left() + t * rect.width();
    let sy = |db: f32| {
        let t = ((db - DB_MIN) / (DB_MAX - DB_MIN)).clamp(0.0, 1.0);
        rect.bottom() - t * rect.height()
    };

    // Background
    painter.rect_filled(rect, CornerRadius::same(theme.rounding_sm as u8), theme.c(&theme.scope_bg));

    // Grid — uses filter_curve_grid token
    let grid_col = theme.ca(&theme.filter_curve_grid);
    let label_col = theme.ca(&theme.filter_curve_label);
    let small = theme.font_small();
    for (f, label) in [
        (100.0_f32, "100"),
        (200.0, "200"),
        (500.0, "500"),
        (1_000.0, "1k"),
        (2_000.0, "2k"),
        (5_000.0, "5k"),
        (10_000.0, "10k"),
    ] {
        let x = sx(freq_to_t(f));
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            Stroke::new(theme.stroke_ui, grid_col),
        );
        painter.text(
            egui::pos2(x, rect.bottom() - 2.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            small.clone(),
            label_col,
        );
    }
    for db in [-48.0_f32, -24.0, -12.0, 0.0, 18.0] {
        let y = sy(db);
        let w = if db == 0.0 { theme.stroke_ui } else { theme.stroke_ui * 0.5 };
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            Stroke::new(w, grid_col),
        );
    }

    // Response curve — LP4 = two cascaded LP2 sections
    const N: usize = 200;
    let db_of = |f: f32| -> f32 {
        let w = f / cutoff;
        let denom = (1.0 - w * w).powi(2) + (w / q_display).powi(2);
        (20.0 * (1.0 / denom).log10()).clamp(DB_MIN, DB_MAX)
    };
    let mut pts: Vec<Pos2> = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let f = F_MIN * (F_MAX / F_MIN).powf(t);
        pts.push(egui::pos2(sx(t), sy(db_of(f))));
    }

    // Filled area — token-derived fill
    let line = theme.c(&theme.filter_curve_line);
    let fill_col = Color32::from_rgba_premultiplied(line.r() / 3, line.g() / 3, line.b() / 3, 110);
    let baseline = rect.bottom();
    for w in pts.windows(2) {
        let quad = vec![
            w[0],
            w[1],
            egui::pos2(w[1].x, baseline),
            egui::pos2(w[0].x, baseline),
        ];
        painter.add(egui::Shape::convex_polygon(quad, fill_col, Stroke::NONE));
    }

    // Curve line
    let line_col = Color32::from_rgba_premultiplied(line.r(), line.g(), line.b(), 210);
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], Stroke::new(theme.stroke_focus, line_col));
    }

    // Control node (cutoff × Q crosshair + dot)
    let node_x = sx(freq_to_t(cutoff));
    let node_y = rect.bottom() - (q_engine / 0.95) * rect.height();
    let cross = Color32::from_rgba_premultiplied(line.r(), line.g(), line.b(), 45);
    painter.line_segment(
        [egui::pos2(node_x, rect.top()), egui::pos2(node_x, rect.bottom())],
        Stroke::new(theme.stroke_ui, cross),
    );
    painter.line_segment(
        [egui::pos2(rect.left(), node_y), egui::pos2(rect.right(), node_y)],
        Stroke::new(theme.stroke_ui, cross),
    );
    painter.circle_filled(egui::pos2(node_x, node_y), 5.0, theme.c(&theme.filter_curve_line));
    painter.circle_stroke(
        egui::pos2(node_x, node_y),
        5.0,
        Stroke::new(theme.stroke_focus, theme.c(&theme.text_primary)),
    );

    // Border (on top)
    painter.rect_stroke(
        rect,
        CornerRadius::same(theme.rounding_sm as u8),
        Stroke::new(theme.stroke_ui, border_col),
        StrokeKind::Middle,
    );
}
