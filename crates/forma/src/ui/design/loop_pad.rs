//! Loop pad — design-system component for the live loop grid.
//!
//! # Visual layer model (bottom → top)
//! 1. Base fill          — always `bg_surface`; gives pads clear presence on `bg_app`
//! 2. State overlay      — RGBA semi-transparent color tint; opacity drives state legibility
//! 3. Waveform           — peak-envelope bars in the middle zone
//! 4. Progress bar       — 6 px pill at the bottom edge (ArmedRec sweep / rec & play head)
//! 5. Badge              — top-left state label in `font_body`
//! 6. Countdown          — large centered beat-countdown for ArmedRec only
//! 7. Length chips       — bottom-center 1/2/4 bar selectors
//! 8. Hold progress bar  — top edge, sweeps red over 3s → clears pad
//! 9. Border             — painted last so it's always crisp on top of every layer
//!
//! # Hold-to-delete
//! Hold the primary button for 3 s to clear a pad. A red bar sweeps across the
//! top edge as feedback. Releasing before 3 s cancels the hold. Right-click also
//! clears immediately (power-user shortcut).

use crate::audio_loops::LOOP_WAVE_BINS;
use crate::ui::theme::SynthTheme;
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

/// Pad state passed to the component. Mirrors `PadState` but is pure data with
/// no audio-thread atomics, so the component stays testable and composable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopPadState {
    Empty,
    ArmedRec,
    Recording,
    ArmedPlay,
    Playing,
    ArmedStop,
    Stopped,
}

/// All input data the pad component needs — computed upstream, no side effects.
pub struct LoopPadProps<'a> {
    /// Pad state (drives color, badge, animations).
    pub state: LoopPadState,
    /// Whether a recording exists on this pad (shows waveform).
    pub has_content: bool,
    /// Playback / record head, 0..1 (used for progress bar when active).
    pub progress: f32,
    /// Currently selected loop length in bars (shown in length chips).
    pub cur_len_bars: u32,
    /// Peak-envelope data for the mini waveform, 0..1 per bin.
    pub wave: &'a [f32; LOOP_WAVE_BINS],
    /// Progress within the current beat, 0..1 (drives armed-state pulse).
    pub beat_frac: f32,
    /// Which beat is active (0 = downbeat).
    pub beat_idx: usize,
    /// Beats per bar (for the ArmedRec countdown).
    pub beats: u32,
    /// Bar progress 0..1 (the ArmedRec sweep bar).
    pub bar_pos: f32,
    /// Unique egui id salt so each pad's interactions are independent.
    pub id_salt: usize,
}

/// Interaction returned by [`draw_loop_pad`].
pub enum LoopPadAction {
    None,
    Tap,
    Clear,
    SetLen(u32),
}

const LEN_PRESETS: [u32; 3] = [1, 2, 4];
const PROGRESS_BAR_H: f32 = 6.0;
const CHIP_H: f32 = 16.0;
const CHIP_W: f32 = 24.0;
const CHIP_GAP: f32 = 3.0;
const HOLD_DELETE_SECS: f32 = 3.0;
const HOLD_BAR_H: f32 = 4.0;

// Sentinel stored in egui temp memory once the hold-delete fires, preventing
// re-trigger until the user releases the button and presses again.
const HOLD_FIRED: f64 = f64::MIN;

/// Draw one loop pad. Returns the interaction if any occurred this frame.
pub fn draw_loop_pad(
    ui: &mut egui::Ui,
    size: Vec2,
    props: &LoopPadProps<'_>,
    theme: &SynthTheme,
) -> LoopPadAction {
    // click_and_drag keeps the widget active while held, which is required for
    // is_pointer_button_down_on() to stay true during a long-press hold.
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());

    // ── Hold-to-delete state ──────────────────────────────────────────────────
    let hold_id = egui::Id::new(("pad_hold", props.id_salt));
    let now = ui.input(|i| i.time);
    let is_pressed = resp.is_pointer_button_down_on();
    let hold_stored: Option<f64> = ui.data(|d| d.get_temp(hold_id));

    // Decode stored sentinel vs real start time.
    let already_fired = hold_stored == Some(HOLD_FIRED);
    let hold_elapsed = match hold_stored {
        Some(t) if t != HOLD_FIRED => ((now - t) as f32).max(0.0),
        _ => 0.0,
    };
    let hold_progress = (hold_elapsed / HOLD_DELETE_SECS).clamp(0.0, 1.0);
    let is_holding = is_pressed && hold_elapsed > 0.02 && !already_fired;

    if !ui.is_rect_visible(rect) {
        if !is_pressed {
            ui.data_mut(|d| d.remove::<f64>(hold_id));
        }
        return LoopPadAction::None;
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(theme.rounding_md as u8);

    // ── Token colours ──────────────────────────────────────────────────────────
    let bg_surface = theme.c(&theme.bg_surface);
    let accent = theme.c(&theme.accent);
    let rec = theme.c(&theme.transport_rec);
    let hold_col = theme.c(&theme.accent_hold);
    let border_dim = theme.c(&theme.border);
    let text_sec = theme.c(&theme.text_secondary);
    let text_dis = theme.c(&theme.text_disabled);

    // Power-decay pulse: bright flash at beat onset, gone by next beat.
    let pulse = (1.0_f32 - props.beat_frac).powf(2.2);

    // ── State-derived visual parameters ───────────────────────────────────────
    // While holding, override the normal state visuals with a red danger tint.
    let (state_color, overlay_alpha, border_col, border_w) = if is_holding {
        let alpha = (20.0 + hold_progress * 90.0) as u8;
        let bw = theme.stroke_ui + hold_progress * 2.5;
        (rec, alpha, rec, bw)
    } else {
        match props.state {
            LoopPadState::Empty => {
                (Color32::TRANSPARENT, 0u8, border_dim, theme.stroke_ui)
            }
            LoopPadState::ArmedRec => {
                let alpha = (35.0 + 55.0 * pulse) as u8;
                let bw = theme.stroke_active + theme.stroke_focus * pulse;
                (rec, alpha, rec, bw)
            }
            LoopPadState::Recording => {
                (rec, 65, rec, theme.stroke_active)
            }
            LoopPadState::ArmedPlay => {
                let alpha = (25.0 + 45.0 * pulse) as u8;
                let bw = theme.stroke_focus + theme.stroke_focus * pulse;
                (accent, alpha, accent, bw)
            }
            LoopPadState::Playing => {
                (accent, 55, accent, theme.stroke_active)
            }
            LoopPadState::ArmedStop => {
                let alpha = (25.0 + 45.0 * pulse) as u8;
                let bw = theme.stroke_focus + theme.stroke_focus * pulse;
                (hold_col, alpha, hold_col, bw)
            }
            LoopPadState::Stopped => {
                let stopped_border = Color32::from_rgb(
                    (border_dim.r() as u32 + accent.r() as u32 / 5).min(255) as u8,
                    (border_dim.g() as u32 + accent.g() as u32 / 5).min(255) as u8,
                    (border_dim.b() as u32 + accent.b() as u32 / 5).min(255) as u8,
                );
                (Color32::TRANSPARENT, 0, stopped_border, theme.stroke_ui)
            }
        }
    };

    // ── Hover brightening ─────────────────────────────────────────────────────
    let hover_extra: u8 = if resp.hovered() && !is_holding { 10 } else { 0 };

    // ── Layer 1: base fill ────────────────────────────────────────────────────
    let base = Color32::from_rgb(
        bg_surface.r().saturating_add(hover_extra),
        bg_surface.g().saturating_add(hover_extra),
        bg_surface.b().saturating_add(hover_extra),
    );
    painter.rect_filled(rect, rounding, base);

    // ── Layer 2: state overlay (RGBA) ─────────────────────────────────────────
    if overlay_alpha > 0 && state_color != Color32::TRANSPARENT {
        painter.rect_filled(
            rect,
            rounding,
            Color32::from_rgba_unmultiplied(
                state_color.r(),
                state_color.g(),
                state_color.b(),
                overlay_alpha,
            ),
        );
    }

    // ── Layer 3: mini waveform ────────────────────────────────────────────────
    if props.has_content {
        let wave_col = match props.state {
            LoopPadState::Playing => accent.gamma_multiply(0.85),
            LoopPadState::Recording => rec.gamma_multiply(0.75),
            _ => text_dis.gamma_multiply(1.8),
        };
        draw_pad_waveform(&painter, rect, props.wave, wave_col);
    }

    // ── Layer 4: progress bar ─────────────────────────────────────────────────
    if !is_holding {
        let bar_fill = match props.state {
            LoopPadState::ArmedRec => Some((props.bar_pos, rec)),
            LoopPadState::Recording => Some((props.progress, rec)),
            LoopPadState::Playing => Some((props.progress, accent)),
            _ => None,
        };
        if let Some((fill, col)) = bar_fill {
            draw_progress_bar(&painter, rect, fill, col, theme);
        }
    }

    // ── Layer 5: badge (top-left) ─────────────────────────────────────────────
    let (badge_text, badge_col) = if is_holding {
        ("HOLD TO DEL", rec)
    } else {
        match props.state {
            LoopPadState::Empty => ("TAP TO REC", text_sec),
            LoopPadState::ArmedRec => ("ARM REC", rec),
            LoopPadState::Recording => ("REC", rec),
            LoopPadState::ArmedPlay => ("ARM PLAY", accent),
            LoopPadState::Playing => ("PLAYING", accent),
            LoopPadState::ArmedStop => ("ARM STOP", hold_col),
            LoopPadState::Stopped => ("STOPPED", text_sec),
        }
    };
    painter.text(
        rect.min + Vec2::new(8.0, 7.0),
        egui::Align2::LEFT_TOP,
        badge_text,
        theme.font_body(),
        badge_col,
    );

    // ── Layer 6: ArmedRec beat countdown ──────────────────────────────────────
    if props.state == LoopPadState::ArmedRec && !is_holding {
        let beats_left = props.beats.saturating_sub(props.beat_idx as u32).max(1);
        let alpha_f = 0.45 + 0.55 * pulse;
        painter.text(
            rect.center() - Vec2::new(0.0, 8.0),
            egui::Align2::CENTER_CENTER,
            format!("{beats_left}"),
            egui::FontId::proportional(36.0),
            Color32::from_rgba_unmultiplied(rec.r(), rec.g(), rec.b(), (alpha_f * 255.0) as u8),
        );
    }

    // ── Layer 7: length chips ─────────────────────────────────────────────────
    let editable = !is_holding && matches!(props.state, LoopPadState::Empty | LoopPadState::Stopped);
    let chips_total = 3.0 * CHIP_W + 2.0 * CHIP_GAP;
    let chip_y = rect.max.y - PROGRESS_BAR_H - CHIP_H - 4.0;
    let chip_x0 = rect.center().x - chips_total * 0.5;

    let mut chip_action = LoopPadAction::None;
    for (i, &bars) in LEN_PRESETS.iter().enumerate() {
        let chip_rect = Rect::from_min_size(
            Pos2::new(chip_x0 + i as f32 * (CHIP_W + CHIP_GAP), chip_y),
            Vec2::new(CHIP_W, CHIP_H),
        );
        let chip_resp = ui.interact(
            chip_rect,
            egui::Id::new(("looplen", props.id_salt, i)),
            Sense::click(),
        );
        if chip_resp.clicked() && editable {
            chip_action = LoopPadAction::SetLen(bars);
        }

        let selected = props.cur_len_bars == bars;
        let (chip_bg, chip_border, chip_text) = if is_holding {
            (Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::TRANSPARENT)
        } else if selected {
            (
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 40),
                accent,
                accent,
            )
        } else if editable {
            (Color32::TRANSPARENT, border_dim, text_dis)
        } else {
            (Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::TRANSPARENT)
        };

        painter.rect_filled(chip_rect, CornerRadius::same(3), chip_bg);
        painter.rect_stroke(
            chip_rect,
            CornerRadius::same(3),
            Stroke::new(1.0, chip_border),
            StrokeKind::Inside,
        );
        painter.text(
            chip_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{bars}"),
            theme.font_micro(),
            chip_text,
        );
    }

    // ── Layer 8: hold progress bar (top edge) ─────────────────────────────────
    if is_holding && hold_progress > 0.01 {
        let track = Rect::from_min_size(rect.min, Vec2::new(rect.width(), HOLD_BAR_H));
        painter.rect_filled(
            track,
            CornerRadius { nw: rounding.nw, ne: rounding.ne, sw: 0, se: 0 },
            Color32::from_rgba_unmultiplied(rec.r(), rec.g(), rec.b(), 35),
        );
        let fill_w = rect.width() * hold_progress;
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, HOLD_BAR_H));
        let fill_rounding = if hold_progress >= 0.98 {
            CornerRadius { nw: rounding.nw, ne: rounding.ne, sw: 0, se: 0 }
        } else {
            CornerRadius { nw: rounding.nw, ne: 0, sw: 0, se: 0 }
        };
        painter.rect_filled(fill_rect, fill_rounding, rec);
    }

    // ── Layer 9: border (always on top) ───────────────────────────────────────
    painter.rect_stroke(
        rect,
        rounding,
        Stroke::new(border_w, border_col),
        StrokeKind::Inside,
    );

    // ── Update hold memory state ───────────────────────────────────────────────
    if is_pressed && hold_stored.is_none() {
        // Started pressing — record start time.
        ui.data_mut(|d| d.insert_temp(hold_id, now));
    } else if !is_pressed {
        // Released — always clear (whether normal or fired).
        ui.data_mut(|d| d.remove::<f64>(hold_id));
    }

    // Keep repainting during a hold so the progress bar animates smoothly.
    if is_pressed {
        ui.ctx().request_repaint();
    }

    // ── Determine action ──────────────────────────────────────────────────────
    if hold_progress >= 1.0 && !already_fired {
        // Long hold completed — set fired sentinel and clear the pad.
        ui.data_mut(|d| d.insert_temp(hold_id, HOLD_FIRED));
        LoopPadAction::Clear
    } else if matches!(chip_action, LoopPadAction::SetLen(_)) {
        chip_action
    } else if resp.secondary_clicked() {
        LoopPadAction::Clear
    } else if resp.clicked() && hold_elapsed < 0.5 && !already_fired {
        // Short tap (< 500 ms) — normal interaction.
        LoopPadAction::Tap
    } else {
        LoopPadAction::None
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn draw_pad_waveform(
    painter: &Painter,
    rect: Rect,
    wave: &[f32; LOOP_WAVE_BINS],
    col: Color32,
) {
    let area = Rect::from_min_max(
        rect.min + Vec2::new(6.0, 26.0),
        rect.max - Vec2::new(6.0, PROGRESS_BAR_H + CHIP_H + 10.0),
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
            Pos2::new(x, mid_y - h),
            Pos2::new(x + bar_w * 0.85, mid_y + h),
        );
        painter.rect_filled(bar, CornerRadius::same(1), col);
    }
}

fn draw_progress_bar(
    painter: &Painter,
    rect: Rect,
    fill: f32,
    col: Color32,
    theme: &SynthTheme,
) {
    let r = CornerRadius {
        sw: theme.rounding_md as u8,
        se: theme.rounding_md as u8,
        nw: 0,
        ne: 0,
    };
    let track = Rect::from_min_size(
        rect.min + Vec2::new(0.0, rect.height() - PROGRESS_BAR_H),
        Vec2::new(rect.width(), PROGRESS_BAR_H),
    );
    painter.rect_filled(
        track,
        r,
        Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), 25),
    );
    if fill > 0.001 {
        let fill_rect = Rect::from_min_size(
            track.min,
            Vec2::new(rect.width() * fill.clamp(0.0, 1.0), PROGRESS_BAR_H),
        );
        painter.rect_filled(fill_rect, r, col);
    }
}
