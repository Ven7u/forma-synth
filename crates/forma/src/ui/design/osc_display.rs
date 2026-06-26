//! OSC mini-waveform preview — Layer 3 design system component.
//!
//! Renders a waveform shape (sine/saw/square/tri) into a given Rect.
//! Color is driven by the `osc_preview_line` theme token so every theme
//! can choose its own palette independently of the scope/filter displays.
//!
//! Future: when the engine exposes per-OSC audio buffers, replace the
//! parametric wave shapes with `draw_osc_audio_buffer(samples: &[f32], ...)`.

use egui::{Color32, CornerRadius, Pos2, Rect, Stroke};

use crate::ui::theme::SynthTheme;

/// Render a parametric waveform preview.
///
/// `wave`        — 0=Sin 1=Saw 2=Sqr 3=Tri
/// `pulse_width` — duty cycle for square wave (0.01..0.99)
/// `active`      — dims the line when the oscillator is off / no note held
pub fn draw_wave_preview(
    painter: &egui::Painter,
    rect: Rect,
    wave: usize,
    pulse_width: f32,
    active: bool,
    theme: &SynthTheme,
) {
    // Background — same dark CRT surface as scope / filter / ADSR
    painter.rect_filled(
        rect,
        CornerRadius::same(theme.rounding_sm as u8),
        theme.c(&theme.scope_bg),
    );

    let line_base = theme.c(&theme.osc_preview_line);
    let line_color = if active {
        line_base
    } else {
        // Dim to ~28% for inactive oscillators
        Color32::from_rgba_premultiplied(
            (line_base.r() as f32 * 0.28) as u8,
            (line_base.g() as f32 * 0.28) as u8,
            (line_base.b() as f32 * 0.28) as u8,
            line_base.a(),
        )
    };

    let w = rect.width();
    let h = rect.height();
    let amp = h * 0.38;
    let cycles = 2.0_f32;
    let steps = 80usize;

    let points: Vec<Pos2> = (0..=steps)
        .map(|s| {
            let t = s as f32 / steps as f32;
            let norm_phase = (t * cycles).fract();
            let phase_rad = t * cycles * std::f32::consts::TAU;

            let y = match wave {
                0 => phase_rad.sin(),
                1 => 1.0 - 2.0 * norm_phase,
                2 => {
                    if norm_phase < pulse_width {
                        1.0
                    } else {
                        -1.0
                    }
                }
                3 => {
                    if norm_phase < 0.5 {
                        4.0 * norm_phase - 1.0
                    } else {
                        3.0 - 4.0 * norm_phase
                    }
                }
                _ => 0.0,
            };

            Pos2::new(rect.left() + t * w, rect.center().y - y * amp)
        })
        .collect();

    let clip = painter.clip_rect();
    let painter = painter.with_clip_rect(clip.intersect(rect));
    for pair in points.windows(2) {
        painter.line_segment(
            [pair[0], pair[1]],
            Stroke::new(theme.stroke_focus, line_color),
        );
    }
}
