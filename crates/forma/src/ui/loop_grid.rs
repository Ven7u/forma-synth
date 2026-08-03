//! Live loop grid — 4×3 matrix of independent audio loops.
//!
//! This is a thin view over [`crate::audio_loops::LoopGridAtomics`]: the actual
//! recording, playback, and bar-quantized timing all run sample-accurately on
//! the audio thread. The UI only reads back per-pad state / progress / waveform
//! and posts tap / length / clear requests.
//!
//! Pads are fully independent — any pad can hold any sound. To give pads
//! different timbres, change the live synth's patch between recordings; each pad
//! freezes whatever was live when it was recorded.

use crate::audio_loops::{PadState, LOOP_PADS, LOOP_WAVE_BINS};
use crate::SynthApp;
use eframe::egui;
use egui::{Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Vec2};

pub const GRID_COLS: usize = 4;
pub const GRID_ROWS: usize = 3;

/// Preset loop lengths in bars.
const LEN_PRESETS: [u32; 3] = [1, 2, 4];

impl SynthApp {
    /// Draw the full 4×3 loop grid inside the Live central panel.
    pub fn ui_loop_grid(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();

        // Manage every gap manually — kill egui's automatic item spacing so the
        // grid fits the panel exactly instead of overflowing.
        ui.spacing_mut().item_spacing = Vec2::ZERO;

        let available = ui.available_size();

        let margin = 10.0_f32;
        let header_h = 26.0_f32;
        let row_label_w = 28.0_f32;
        let gap = 8.0_f32;

        let cell_w = ((available.x - margin * 2.0 - row_label_w - gap * (GRID_COLS as f32 - 1.0))
            / GRID_COLS as f32)
            .max(0.0);
        let cell_h =
            ((available.y - margin * 2.0 - header_h - gap - gap * (GRID_ROWS as f32 - 1.0))
                / GRID_ROWS as f32)
                .max(0.0);

        let track_names: Vec<String> = self.track_names.to_vec();
        let focused_track = self.focused_track;

        let accent = theme.c(&theme.accent);
        let text_sec = theme.c(&theme.text_secondary);

        // Collected interactions (applied after the render pass).
        let mut tap: Option<usize> = None;
        let mut clear: Option<usize> = None;
        let mut set_len: Option<(usize, u32)> = None;

        ui.add_space(margin);

        // ── Column headers ─────────────────────────────────────────────────────
        // The focused (live) track's column is highlighted; the others are
        // secondary. Track identity is position + label, not hue.
        ui.horizontal(|ui| {
            ui.add_space(margin + row_label_w);
            #[allow(clippy::needless_range_loop)]
            for col in 0..GRID_COLS {
                let name = track_names.get(col).map(|s| s.as_str()).unwrap_or("");
                let header_label = format!("T{}  {}", col + 1, name);
                let header_col = if col == focused_track {
                    accent
                } else {
                    text_sec
                };
                let (header_rect, _) =
                    ui.allocate_exact_size(Vec2::new(cell_w, header_h), Sense::hover());
                ui.painter_at(header_rect).text(
                    header_rect.center_bottom() - Vec2::new(0.0, 4.0),
                    egui::Align2::CENTER_BOTTOM,
                    &header_label,
                    theme.font_small(),
                    header_col,
                );
                if col < GRID_COLS - 1 {
                    ui.add_space(gap);
                }
            }
        });

        ui.add_space(gap);

        // ── Grid rows ───────────────────────────────────────────────────────────
        for row in 0..GRID_ROWS {
            ui.horizontal(|ui| {
                // Row label (scene number).
                ui.add_space(margin);
                let (label_rect, _) =
                    ui.allocate_exact_size(Vec2::new(row_label_w, cell_h), Sense::hover());
                ui.painter_at(label_rect).text(
                    label_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", row + 1),
                    theme.font_body(),
                    theme.c(&theme.text_disabled),
                );

                for col in 0..GRID_COLS {
                    let idx = row * GRID_COLS + col;
                    let action =
                        draw_pad(ui, idx, &self.loop_audio, Vec2::new(cell_w, cell_h), &theme);
                    match action {
                        PadAction::None => {}
                        PadAction::Tap => tap = Some(idx),
                        PadAction::Clear => clear = Some(idx),
                        PadAction::SetLen(bars) => set_len = Some((idx, bars)),
                    }
                    if col < GRID_COLS - 1 {
                        ui.add_space(gap);
                    }
                }
            });

            if row < GRID_ROWS - 1 {
                ui.add_space(gap);
            }
        }

        // Apply interactions.
        if let Some(idx) = tap {
            self.loop_audio.tap(idx);
        }
        if let Some(idx) = clear {
            self.loop_audio.clear(idx);
        }
        if let Some((idx, bars)) = set_len {
            self.loop_audio.set_len_bars(idx, bars);
        }

        // Keep the UI animating while anything is armed / running.
        ui.ctx().request_repaint();
    }
}

enum PadAction {
    None,
    Tap,
    Clear,
    SetLen(u32),
}

/// Draw one pad from the shared atomics. Returns the interaction (if any).
fn draw_pad(
    ui: &mut egui::Ui,
    idx: usize,
    loops: &crate::audio_loops::LoopGridAtomics,
    size: Vec2,
    theme: &crate::ui::theme::SynthTheme,
) -> PadAction {
    debug_assert!(idx < LOOP_PADS);

    let state = loops.pad_state(idx);
    let has_content = loops.pad_has_content(idx);
    let progress = loops.pad_progress(idx);
    let cur_len = loops.pad_len_bars(idx);

    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let mut action = if resp.clicked() {
        PadAction::Tap
    } else if resp.secondary_clicked() {
        PadAction::Clear
    } else {
        PadAction::None
    };

    if !ui.is_rect_visible(rect) {
        return action;
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(theme.rounding_md as u8);

    // ── Semantic state colors (design-system tokens only) ──────────────────
    let accent = theme.c(&theme.accent);
    let rec = theme.c(&theme.transport_rec);
    let hold = theme.c(&theme.accent_hold);
    let border = theme.c(&theme.border);

    // Background / border by state.
    let (bg, border_col, border_w) = match state {
        PadState::Empty => (theme.c(&theme.bg_sunken), border, theme.stroke_ui),
        PadState::ArmedRec => (rec.gamma_multiply(0.14), rec, theme.stroke_focus),
        PadState::Recording => (rec.gamma_multiply(0.30), rec, theme.stroke_focus),
        PadState::ArmedPlay => (accent.gamma_multiply(0.14), accent, theme.stroke_focus),
        PadState::Playing => (accent.gamma_multiply(0.28), accent, theme.stroke_focus),
        PadState::ArmedStop => (hold.gamma_multiply(0.14), hold, theme.stroke_focus),
        PadState::Stopped => (theme.c(&theme.bg_surface), border, theme.stroke_ui),
    };

    painter.rect_filled(rect, rounding, bg);
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(border_w, border_col),
        StrokeKind::Inside,
    );

    // ── Mini waveform (content present) ────────────────────────────────────
    if has_content {
        let wave_col = if state == PadState::Playing {
            accent
        } else {
            theme.c(&theme.accent_dim)
        };
        draw_mini_wave(&painter, rect, loops, idx, wave_col);
    }

    // ── Playhead progress bar (Recording / Playing) ───────────────────────
    if matches!(state, PadState::Recording | PadState::Playing) {
        let bar_rect = Rect::from_min_size(
            rect.min + Vec2::new(0.0, rect.height() - 4.0),
            Vec2::new(rect.width() * progress, 4.0),
        );
        let progress_col = if state == PadState::Recording {
            rec
        } else {
            accent
        };
        painter.rect_filled(
            bar_rect,
            CornerRadius {
                sw: theme.rounding_md as u8,
                se: theme.rounding_md as u8,
                nw: 0,
                ne: 0,
            },
            progress_col,
        );
    }

    // ── State badge (top-left) ────────────────────────────────────────────
    let (badge, badge_col) = match state {
        PadState::Empty => ("+ REC", theme.c(&theme.text_disabled)),
        PadState::ArmedRec => ("● REC…", rec),
        PadState::Recording => ("⬤ REC", rec),
        PadState::ArmedPlay => ("▶ PLAY…", accent),
        PadState::Playing => ("▶ PLAY", accent),
        PadState::ArmedStop => ("◼ STOP…", hold),
        PadState::Stopped => ("◼ STOP", theme.c(&theme.text_secondary)),
    };
    painter.text(
        rect.min + Vec2::new(8.0, 8.0),
        egui::Align2::LEFT_TOP,
        badge,
        theme.font_small(),
        badge_col,
    );

    // Pulse the border while armed (waiting for the next bar).
    if matches!(
        state,
        PadState::ArmedRec | PadState::ArmedPlay | PadState::ArmedStop
    ) {
        let t = ui.input(|i| i.time);
        let pulse = 0.5 + 0.5 * (t * 6.0).sin() as f32;
        painter.rect_stroke(
            rect,
            rounding,
            Stroke::new(theme.stroke_focus, badge_col.gamma_multiply(pulse)),
            StrokeKind::Inside,
        );
    }

    // ── Length chips (bottom center) — only editable when empty/stopped ────
    let editable = matches!(state, PadState::Empty | PadState::Stopped);
    let chip_y = rect.max.y - 24.0;
    let chip_h = 18.0;
    let chip_w = 26.0;
    let chip_gap = 2.0;
    let chips_total = 3.0 * chip_w + 2.0 * chip_gap;
    let chip_x_start = rect.center().x - chips_total / 2.0;

    for (i, bars) in LEN_PRESETS.iter().enumerate() {
        let chip_rect = Rect::from_min_size(
            egui::pos2(chip_x_start + i as f32 * (chip_w + chip_gap), chip_y),
            Vec2::new(chip_w, chip_h),
        );
        let chip_resp = ui.interact(
            chip_rect,
            egui::Id::new(("looplen", idx, i)),
            Sense::click(),
        );
        if chip_resp.clicked() && editable {
            // A chip click sets the length instead of triggering the pad.
            action = PadAction::SetLen(*bars);
        }
        let active = cur_len == *bars;
        let chip_fill = if active {
            accent.gamma_multiply(0.25)
        } else {
            Color32::TRANSPARENT
        };
        let chip_stroke_col = if active { accent } else { border };
        let chip_text_col = if active {
            accent
        } else if editable {
            theme.c(&theme.text_disabled)
        } else {
            theme.c(&theme.bg_surface)
        };
        painter.rect_filled(chip_rect, CornerRadius::same(3), chip_fill);
        painter.rect_stroke(
            chip_rect,
            CornerRadius::same(3),
            Stroke::new(1.0, chip_stroke_col),
            StrokeKind::Inside,
        );
        painter.text(
            chip_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", bars),
            theme.font_micro(),
            chip_text_col,
        );
    }

    action
}

/// Draw the pad's peak-envelope waveform, centered vertically.
fn draw_mini_wave(
    painter: &egui::Painter,
    rect: Rect,
    loops: &crate::audio_loops::LoopGridAtomics,
    idx: usize,
    col: Color32,
) {
    let mut wave = [0.0_f32; LOOP_WAVE_BINS];
    loops.pad_wave(idx, &mut wave);

    // Leave room for the badge (top) and length chips (bottom).
    let area = Rect::from_min_max(
        rect.min + Vec2::new(6.0, 24.0),
        rect.max - Vec2::new(6.0, 30.0),
    );
    if area.width() < 8.0 || area.height() < 6.0 {
        return;
    }

    let mid_y = area.center().y;
    let n = LOOP_WAVE_BINS as f32;
    let bar_w = (area.width() / n).max(1.0);
    for (i, &v) in wave.iter().enumerate() {
        let h = (v * area.height() * 0.5).max(0.5);
        let x = area.min.x + i as f32 * (area.width() / n);
        let bar = Rect::from_min_max(
            egui::pos2(x, mid_y - h),
            egui::pos2(x + bar_w * 0.8, mid_y + h),
        );
        painter.rect_filled(bar, CornerRadius::same(1), col);
    }
}
