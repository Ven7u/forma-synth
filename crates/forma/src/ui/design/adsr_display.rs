//! ADSR envelope visualizer — Layer 3 design system component.
//!
//! Renders a read-only ADSR shape into the remaining available width at
//! the specified height. Colors are driven by `adsr_*` and `bg_adsr`
//! theme tokens so every theme has full control over the palette.

use egui::{Color32, CornerRadius, Pos2, Stroke};

use crate::ui::theme::SynthTheme;

/// Draw an ADSR envelope shape into the next available horizontal space.
///
/// `adsr`    — [attack, decay, sustain, release] in engine units
/// `cursors` — active voice playhead positions (phase + fractional progress)
/// `height`  — pixel height to allocate
pub fn draw_adsr_visualizer(
    ui: &mut egui::Ui,
    adsr: &[f32; 4],
    cursors: &[f32],
    theme: &SynthTheme,
    height: f32,
) {
    let (resp, painter) = ui.allocate_painter(
        egui::Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let rect = resp.rect;

    painter.rect_filled(rect, CornerRadius::same(theme.rounding_sm as u8), theme.c(&theme.bg_adsr));

    let a = adsr[0];
    let d = adsr[1];
    let s = adsr[2];
    let r = adsr[3];

    let total = a + d + r;
    let s_vis = total * 0.35;
    let span = a + d + s_vis + r;

    let w = rect.width();
    let h = rect.height();
    let pad_y = 4.0;
    let usable_h = h - pad_y * 2.0;

    let tx = |t: f32| rect.left() + (t / span) * w;
    let ly = |level: f32| rect.bottom() - pad_y - level * usable_h;

    let p0 = Pos2::new(rect.left(), ly(0.0));
    let p1 = Pos2::new(tx(a), ly(1.0));
    let p2 = Pos2::new(tx(a + d), ly(s));
    let p3 = Pos2::new(tx(a + d + s_vis), ly(s));
    let p4 = Pos2::new(rect.right(), ly(0.0));

    // Filled envelope shape
    let fill_pts = vec![
        p0,
        p1,
        p2,
        p3,
        p4,
        Pos2::new(rect.right(), rect.bottom() - pad_y),
        Pos2::new(rect.left(), rect.bottom() - pad_y),
    ];
    painter.add(egui::Shape::convex_polygon(
        fill_pts,
        theme.ca(&theme.adsr_fill),
        Stroke::NONE,
    ));

    // Outline
    let stroke = Stroke::new(theme.stroke_focus, theme.c(&theme.adsr_outline));
    for w in [p0, p1, p2, p3, p4].windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }

    // Stage labels (A / D / S / R) inside the plot
    let label_color = theme.ca(&theme.adsr_label);
    let small = theme.font_body();
    for (label, x) in [
        ("A", tx(a * 0.5)),
        ("D", tx(a + d * 0.5)),
        ("S", tx(a + d + s_vis * 0.5)),
        ("R", tx(a + d + s_vis + r * 0.5)),
    ] {
        painter.text(
            Pos2::new(x, rect.bottom() - pad_y - 2.0),
            egui::Align2::CENTER_BOTTOM,
            label,
            small.clone(),
            label_color,
        );
    }

    // Active voice playheads
    for &cursor in cursors {
        if cursor < 0.5 {
            continue;
        }
        let phase = cursor as u8;
        let progress = cursor.fract();
        let pos = match phase {
            1 => Pos2::new(tx(a * progress), ly(progress)),
            2 => Pos2::new(tx(a + d * progress), ly(1.0 - (1.0 - s) * progress)),
            3 => Pos2::new(tx(a + d + s_vis * 0.5), ly(s)),
            4 => Pos2::new(tx(a + d + s_vis + r * progress), ly(s * (1.0 - progress))),
            _ => continue,
        };
        let cc = theme.c(&theme.adsr_cursor);
        painter.circle_filled(
            pos,
            5.0,
            Color32::from_rgba_premultiplied(cc.r(), cc.g(), cc.b(), 40),
        );
        painter.circle_filled(pos, 2.5, cc);
    }
}
