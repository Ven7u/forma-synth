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

use crate::audio_loops::{beat_state_from_bar_phase, segment_fill, PadState, LOOP_PADS, LOOP_WAVE_BINS};
use crate::ui::design::loop_pad::{
    draw_loop_pad, LoopPadAction, LoopPadProps, LoopPadState,
};
use crate::SynthApp;
use eframe::egui;
use egui::{CornerRadius, Sense, Vec2};

pub const GRID_COLS: usize = 4;
pub const GRID_ROWS: usize = 3;

/// Height reserved for the beat-dot bar between column headers and pad rows.
const BEATS_H: f32 = 24.0;

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
        // BEATS_H + extra gap replace the single gap that was between header and rows.
        let cell_h = ((available.y
            - margin * 2.0
            - header_h
            - gap
            - BEATS_H
            - gap
            - gap * (GRID_ROWS as f32 - 1.0))
            / GRID_ROWS as f32)
            .max(0.0);

        let track_names: Vec<String> = self.track_names.to_vec();
        let focused_track = self.focused_track;

        let accent = theme.c(&theme.accent);
        let text_sec = theme.c(&theme.text_secondary);

        // Beat state — use the audio-thread bar_phase when the transport is running so
        // the animation is sample-accurately aligned with actual recording start times.
        // Fall back to the UI-thread metro clock only when the transport is stopped
        // (e.g., user is on the Live screen but hasn't started any loops yet).
        let beats = self.metro_beats as u32;
        let (beat_idx, beat_frac, bar_pos) = if self.loop_audio.is_running() {
            let bar_phase = self.loop_audio.bar_phase();
            let (bi, bf) = beat_state_from_bar_phase(bar_phase, beats);
            (bi, bf, bar_phase)
        } else {
            let bi = self.metro_phase as usize % beats.max(1) as usize;
            let bf = self.metro_phase.fract() as f32;
            let bp = (self.metro_phase as f32 / beats as f32).clamp(0.0, 1.0);
            (bi, bf, bp)
        };

        // Collected interactions (applied after the render pass).
        let mut tap: Option<usize> = None;
        let mut clear: Option<usize> = None;
        let mut set_len: Option<(usize, u32)> = None;

        ui.add_space(margin);

        // ── Column headers ─────────────────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.add_space(margin + row_label_w);
            #[allow(clippy::needless_range_loop)]
            for col in 0..GRID_COLS {
                let name = track_names.get(col).map(|s| s.as_str()).unwrap_or("");
                let header_label = format!("T{}  {}", col + 1, name);
                let header_col = if col == focused_track { accent } else { text_sec };
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

        // ── Beat timeline strip ────────────────────────────────────────────────
        // The full grid width is divided into N equal beat-slots. The active slot
        // fills left→right showing intra-beat progress; past slots are full-bright;
        // future slots are dim. This gives continuous position feedback — you see
        // both WHICH beat and HOW FAR through it you are, just like a DAW timeline.
        ui.horizontal(|ui| {
            ui.add_space(margin + row_label_w);
            let grid_w = cell_w * GRID_COLS as f32 + gap * (GRID_COLS as f32 - 1.0);
            let (strip_rect, _) =
                ui.allocate_exact_size(Vec2::new(grid_w, BEATS_H), Sense::hover());
            let painter = ui.painter_at(strip_rect);

            let accent_beat = theme.c(&theme.accent_beat);
            let seg_gap = 3.0_f32;
            let bar_h = 6.0_f32;
            let dot_r = 3.5_f32;
            let seg_w = (grid_w - seg_gap * (beats - 1) as f32) / beats as f32;
            let bar_y = strip_rect.center().y + dot_r + 2.0;

            for b in 0..beats {
                let seg_x = strip_rect.min.x + b as f32 * (seg_w + seg_gap);
                let full_col = if b == 0 { accent } else { accent_beat };
                let dim_col = full_col.gamma_multiply(0.15);
                let fill = segment_fill(b as usize, beat_idx, beat_frac);

                // ── Dot marker above each segment ──────────────────────────
                let dot_cx = seg_x + seg_w * 0.5;
                let dot_cy = strip_rect.center().y - dot_r - 2.0;
                let dot_col = if fill > 0.0 { full_col } else { dim_col };
                let dot_r_draw = if b == 0 { dot_r + 1.5 } else { dot_r };
                painter.circle_filled(egui::pos2(dot_cx, dot_cy), dot_r_draw, dot_col);

                // ── Segment fill bar ───────────────────────────────────────
                let fill_frac = fill;
                // Dim background track
                let track_rect = egui::Rect::from_min_size(
                    egui::pos2(seg_x, bar_y),
                    Vec2::new(seg_w, bar_h),
                );
                painter.rect_filled(
                    track_rect,
                    CornerRadius::same(bar_h as u8 / 2),
                    dim_col,
                );
                // Filled portion
                if fill_frac > 0.0 {
                    let fill_rect = egui::Rect::from_min_size(
                        egui::pos2(seg_x, bar_y),
                        Vec2::new(seg_w * fill_frac, bar_h),
                    );
                    painter.rect_filled(
                        fill_rect,
                        CornerRadius::same(bar_h as u8 / 2),
                        full_col,
                    );
                }
            }
        });

        ui.add_space(gap);

        // ── Grid rows ───────────────────────────────────────────────────────────
        for row in 0..GRID_ROWS {
            ui.horizontal(|ui| {
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
                    let pad_state = self.loop_audio.pad_state(idx);
                    let mut wave = [0.0f32; LOOP_WAVE_BINS];
                    self.loop_audio.pad_wave(idx, &mut wave);

                    let props = LoopPadProps {
                        state: pad_state_to_ui(pad_state),
                        has_content: self.loop_audio.pad_has_content(idx),
                        progress: self.loop_audio.pad_progress(idx),
                        cur_len_bars: self.loop_audio.pad_len_bars(idx),
                        wave: &wave,
                        beat_frac,
                        beat_idx,
                        beats,
                        bar_pos,
                        id_salt: idx,
                    };
                    let action = draw_loop_pad(ui, Vec2::new(cell_w, cell_h), &props, &theme);
                    match action {
                        LoopPadAction::None => {}
                        LoopPadAction::Tap => tap = Some(idx),
                        LoopPadAction::Clear => clear = Some(idx),
                        LoopPadAction::SetLen(bars) => set_len = Some((idx, bars)),
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

        // Repaint while the transport is running (beat dots + progress bars animate).
        if self.loop_audio.is_running() {
            ui.ctx().request_repaint();
        }
    }
}

fn pad_state_to_ui(s: PadState) -> LoopPadState {
    match s {
        PadState::Empty => LoopPadState::Empty,
        PadState::ArmedRec => LoopPadState::ArmedRec,
        PadState::Recording => LoopPadState::Recording,
        PadState::ArmedPlay => LoopPadState::ArmedPlay,
        PadState::Playing => LoopPadState::Playing,
        PadState::ArmedStop => LoopPadState::ArmedStop,
        PadState::Stopped => LoopPadState::Stopped,
    }
}
