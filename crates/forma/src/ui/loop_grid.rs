//! Live loop grid — 4×3 matrix of independent MIDI loops.
#![allow(dead_code)] // scaffolding — most items are wired up incrementally
//!
//! Each cell records live note events, quantizes start/stop to the bar boundary,
//! and plays back through the focused synth engine. The grid replaces the studio
//! dock when AppMode::Live is active and the user switches to loop-grid view.

use crate::SynthApp;
use eframe::egui;
use egui::{Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Vec2};

// ── Grid constants ────────────────────────────────────────────────────────────

pub const GRID_COLS: usize = 4;
pub const GRID_ROWS: usize = 3;
pub const GRID_SIZE: usize = GRID_COLS * GRID_ROWS;

// ── Data model ────────────────────────────────────────────────────────────────

/// A single MIDI event stored in a loop cell.
#[derive(Clone, Debug)]
pub struct LoopNote {
    /// Beat position within the loop (0.0 = loop start, length_beats = loop end).
    pub beat: f64,
    pub midi: u8,
    pub velocity: u8,
    /// Duration in beats. 0.0 for NoteOff (paired with a preceding NoteOn).
    pub duration_beats: f64,
}

/// Available preset loop lengths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopLen {
    One = 1,
    Two = 2,
    Four = 4,
}

impl LoopLen {
    pub fn bars(self) -> u32 {
        self as u32
    }

    pub fn label(self) -> &'static str {
        match self {
            LoopLen::One => "1",
            LoopLen::Two => "2",
            LoopLen::Four => "4",
        }
    }
}

/// The playback / recording state of one loop cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopState {
    /// No content recorded yet.
    Empty,
    /// Waiting for the next bar boundary to start recording.
    Armed,
    /// Actively recording MIDI events.
    Recording,
    /// Has content and is playing back.
    Playing,
    /// Has content but is stopped (waiting for next trigger).
    Stopped,
}

/// One cell in the 4×3 loop grid.
#[derive(Clone, Debug)]
pub struct LoopCell {
    pub state: LoopState,
    pub len: LoopLen,
    /// Recorded MIDI events. Timestamps are in beats relative to loop start.
    pub notes: Vec<LoopNote>,
    /// Playback cursor: current position in beats within the loop.
    pub playhead: f64,
    /// Beat position (within the global bar grid) at which recording started.
    pub record_start_beat: f64,
}

impl LoopCell {
    pub fn new() -> Self {
        Self {
            state: LoopState::Empty,
            len: LoopLen::Two,
            notes: Vec::new(),
            playhead: 0.0,
            record_start_beat: 0.0,
        }
    }

    /// Length of this loop in beats (bars × beats_per_bar).
    pub fn length_beats(&self, beats_per_bar: u8) -> f64 {
        self.len.bars() as f64 * beats_per_bar as f64
    }

    pub fn has_content(&self) -> bool {
        !self.notes.is_empty()
    }
}

// ── SynthApp state fields (declared here for clarity; added in main.rs) ──────
//
// pub(crate) loop_cells: [LoopCell; GRID_SIZE]
// pub(crate) loop_grid_active: bool   — true when the grid panel is shown

// ── UI ────────────────────────────────────────────────────────────────────────

impl SynthApp {
    /// Draw the full 4×3 loop grid inside the Live central panel.
    pub fn ui_loop_grid(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let available = ui.available_size();

        // Leave a small margin around the grid.
        let margin = theme.sp_sm;
        let inner_w = available.x - margin * 2.0;
        let inner_h = available.y - margin * 2.0;
        let cell_w = inner_w / GRID_COLS as f32;
        let cell_h = inner_h / GRID_ROWS as f32;

        // Collect interactions so we can mutate self after the painting pass.
        let mut trigger: Option<usize> = None;
        let mut set_len: Option<(usize, LoopLen)> = None;

        ui.add_space(margin);

        for row in 0..GRID_ROWS {
            ui.horizontal(|ui| {
                ui.add_space(margin);
                for col in 0..GRID_COLS {
                    let idx = row * GRID_COLS + col;
                    let cell = &self.loop_cells[idx];
                    let (cell_trigger, cell_len) =
                        draw_loop_cell(ui, idx, cell, cell_w, cell_h, &theme);
                    if cell_trigger {
                        trigger = Some(idx);
                    }
                    if let Some(l) = cell_len {
                        set_len = Some((idx, l));
                    }
                }
            });
        }

        // Apply mutations after the borrow of self.loop_cells ends.
        if let Some(idx) = trigger {
            self.loop_cell_trigger(idx);
        }
        if let Some((idx, len)) = set_len {
            self.loop_cells[idx].len = len;
        }
    }

    /// Handle a cell tap: state machine transition + quantized scheduling.
    fn loop_cell_trigger(&mut self, idx: usize) {
        let cell = &self.loop_cells[idx];
        match cell.state {
            LoopState::Empty | LoopState::Stopped => {
                if cell.has_content() {
                    // Stopped with content → arm to play on next bar.
                    self.loop_cells[idx].state = LoopState::Armed;
                } else {
                    // Empty → arm to record on next bar.
                    self.loop_cells[idx].state = LoopState::Armed;
                }
            }
            LoopState::Armed => {
                // Cancel arm.
                self.loop_cells[idx].state = if self.loop_cells[idx].has_content() {
                    LoopState::Stopped
                } else {
                    LoopState::Empty
                };
            }
            LoopState::Recording => {
                // Stop recording → will commit and start playing on next bar.
                self.loop_cells[idx].state = LoopState::Armed;
            }
            LoopState::Playing => {
                // Stop playing on next bar boundary.
                self.loop_cells[idx].state = LoopState::Armed;
            }
        }
    }

    /// Called every UI frame from `ui_live_view`. Advances playheads and handles
    /// bar-boundary transitions. `dt` is the frame delta time in seconds.
    pub fn tick_loop_grid(&mut self, dt: f32) {
        let bpm = self.global_bpm as f64;
        let beats_per_sec = bpm / 60.0;
        let beats_per_bar = self.metro_beats;
        let dt_beats = dt as f64 * beats_per_sec;

        // Current global beat position within the bar [0, beats_per_bar).
        let global_beat = self.metro_phase;
        // True when we just crossed a bar boundary this frame.
        let on_bar = {
            let prev = global_beat - dt_beats;
            prev < 0.0 || global_beat < prev // wrapped around
        };

        for cell in self.loop_cells.iter_mut() {
            match cell.state {
                LoopState::Armed => {
                    if on_bar {
                        if cell.has_content() {
                            // Transition Armed→Playing: reset playhead.
                            cell.playhead = 0.0;
                            cell.state = LoopState::Playing;
                        } else {
                            // Transition Armed→Recording.
                            cell.notes.clear();
                            cell.playhead = 0.0;
                            cell.record_start_beat = global_beat;
                            cell.state = LoopState::Recording;
                        }
                    }
                }
                LoopState::Recording => {
                    cell.playhead += dt_beats;
                    let loop_len = cell.length_beats(beats_per_bar);
                    // Auto-stop when we reach the end of the loop length.
                    if cell.playhead >= loop_len {
                        cell.playhead = 0.0;
                        cell.state = LoopState::Playing;
                    }
                }
                LoopState::Playing => {
                    cell.playhead += dt_beats;
                    let loop_len = cell.length_beats(beats_per_bar);
                    if cell.playhead >= loop_len {
                        cell.playhead -= loop_len; // wrap
                    }
                    // TODO: dispatch NoteOn/NoteOff events as playhead crosses them.
                }
                LoopState::Empty | LoopState::Stopped => {}
            }
        }
    }

    /// Record a NoteOn into all currently-recording cells.
    pub fn loop_grid_record_note_on(&mut self, midi: u8, velocity: u8) {
        let beats_per_bar = self.metro_beats;
        for cell in self.loop_cells.iter_mut() {
            if cell.state == LoopState::Recording {
                let len = cell.length_beats(beats_per_bar);
                let beat = cell.playhead.min(len - 0.001);
                cell.notes.push(LoopNote {
                    beat,
                    midi,
                    velocity,
                    duration_beats: 0.0, // filled in on NoteOff
                });
            }
        }
    }

    /// Record a NoteOff — fills in the duration of the matching open NoteOn.
    pub fn loop_grid_record_note_off(&mut self, midi: u8) {
        for cell in self.loop_cells.iter_mut() {
            if cell.state == LoopState::Recording {
                // Find the last open NoteOn for this pitch and fill its duration.
                for note in cell.notes.iter_mut().rev() {
                    if note.midi == midi && note.duration_beats == 0.0 {
                        note.duration_beats = (cell.playhead - note.beat).max(0.01);
                        break;
                    }
                }
            }
        }
    }
}

// ── Cell renderer ─────────────────────────────────────────────────────────────

/// Draw one loop cell. Returns `(triggered, new_len)`.
fn draw_loop_cell(
    ui: &mut egui::Ui,
    idx: usize,
    cell: &LoopCell,
    w: f32,
    h: f32,
    theme: &crate::ui::theme::SynthTheme,
) -> (bool, Option<LoopLen>) {
    let pad = 4.0;
    let size = Vec2::new(w - pad, h - pad);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let mut triggered = resp.clicked();
    let mut new_len: Option<LoopLen> = None;

    if !ui.is_rect_visible(rect) {
        return (triggered, new_len);
    }

    let painter = ui.painter_at(rect);
    let rounding = CornerRadius::same(theme.rounding_md as u8);

    // ── Background color by state ──────────────────────────────────────────
    let (bg, border_col, border_w) = match cell.state {
        LoopState::Empty => (
            theme.c(&theme.bg_sunken),
            theme.c(&theme.border),
            theme.stroke_ui,
        ),
        LoopState::Armed => (
            theme.c(&theme.bg_surface),
            theme.c(&theme.accent),
            theme.stroke_focus,
        ),
        LoopState::Recording => (
            // Warm tint for recording — use accent_hard_sync (warm orange/red).
            theme.c(&theme.accent_hard_sync).gamma_multiply(0.25),
            theme.c(&theme.accent_hard_sync),
            theme.stroke_focus,
        ),
        LoopState::Playing => (
            theme.c(&theme.accent).gamma_multiply(0.18),
            theme.c(&theme.accent),
            theme.stroke_focus,
        ),
        LoopState::Stopped => (
            theme.c(&theme.bg_surface),
            theme.c(&theme.border),
            theme.stroke_ui,
        ),
    };

    painter.rect_filled(rect, rounding, bg);
    painter.rect_stroke(rect, rounding, Stroke::new(border_w, border_col), StrokeKind::Inside);

    // ── Playhead progress bar (Recording / Playing) ───────────────────────
    if matches!(cell.state, LoopState::Recording | LoopState::Playing) {
        let beats_per_bar = 4u8; // fallback; real value comes from SynthApp
        let loop_len = cell.length_beats(beats_per_bar);
        let frac = (cell.playhead / loop_len).clamp(0.0, 1.0) as f32;
        let bar_rect = Rect::from_min_size(
            rect.min + Vec2::new(0.0, rect.height() - 4.0),
            Vec2::new(rect.width() * frac, 4.0),
        );
        let progress_col = match cell.state {
            LoopState::Recording => theme.c(&theme.accent_hard_sync),
            _ => theme.c(&theme.accent),
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

    // ── Mini piano-roll (Stopped / Playing with content) ─────────────────
    if cell.has_content() && !cell.notes.is_empty() {
        draw_mini_roll(&painter, rect, cell, theme);
    }

    // ── State badge (top-left) ────────────────────────────────────────────
    let badge = match cell.state {
        LoopState::Empty => "",
        LoopState::Armed => "●",
        LoopState::Recording => "⬤ REC",
        LoopState::Playing => "▶",
        LoopState::Stopped => "■",
    };
    if !badge.is_empty() {
        let badge_col = match cell.state {
            LoopState::Recording => theme.c(&theme.accent_hard_sync),
            LoopState::Playing => theme.c(&theme.accent),
            _ => theme.c(&theme.text_secondary),
        };
        painter.text(
            rect.min + Vec2::new(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            badge,
            theme.font_small(),
            badge_col,
        );
    }

    // ── Cell index label (top-right, dimmed) ──────────────────────────────
    painter.text(
        rect.max - Vec2::new(8.0, rect.height() - 8.0),
        egui::Align2::RIGHT_TOP,
        format!("{}", idx + 1),
        theme.font_micro(),
        theme.c(&theme.text_disabled),
    );

    // ── Length selector chips (bottom of cell) ────────────────────────────
    let chip_y = rect.max.y - 24.0;
    let chip_h = 18.0;
    let chip_w = 26.0;
    let chip_gap = 2.0;
    let chips_total = 3.0 * chip_w + 2.0 * chip_gap;
    let chip_x_start = rect.center().x - chips_total / 2.0;

    for (i, len_opt) in [LoopLen::One, LoopLen::Two, LoopLen::Four]
        .iter()
        .enumerate()
    {
        let chip_rect = Rect::from_min_size(
            egui::pos2(chip_x_start + i as f32 * (chip_w + chip_gap), chip_y),
            Vec2::new(chip_w, chip_h),
        );
        let chip_resp = ui.interact(chip_rect, egui::Id::new((idx, i)), Sense::click());
        if chip_resp.clicked() {
            // Prevent the cell body click from also firing when clicking a chip.
            triggered = false;
            if cell.state == LoopState::Empty || cell.state == LoopState::Stopped {
                new_len = Some(*len_opt);
            }
        }
        let active = cell.len == *len_opt;
        let chip_fill = if active {
            theme.c(&theme.accent).gamma_multiply(0.3)
        } else {
            Color32::TRANSPARENT
        };
        let chip_stroke_col = if active {
            theme.c(&theme.accent)
        } else {
            theme.c(&theme.border)
        };
        let chip_text_col = if active {
            theme.c(&theme.accent)
        } else {
            theme.c(&theme.text_disabled)
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
            len_opt.label(),
            theme.font_micro(),
            chip_text_col,
        );
    }

    (triggered, new_len)
}

/// Draw a tiny piano roll showing the notes in a cell.
fn draw_mini_roll(
    painter: &egui::Painter,
    rect: Rect,
    cell: &LoopCell,
    theme: &crate::ui::theme::SynthTheme,
) {
    if cell.notes.is_empty() {
        return;
    }

    // Leave room for the badges and chips.
    let roll_rect = Rect::from_min_max(
        rect.min + Vec2::new(4.0, 22.0),
        rect.max - Vec2::new(4.0, 30.0),
    );
    if roll_rect.height() < 8.0 || roll_rect.width() < 8.0 {
        return;
    }

    let beats_per_bar = 4u8;
    let loop_len = cell.length_beats(beats_per_bar) as f32;

    // MIDI note range visible in the roll.
    let min_pitch = cell.notes.iter().map(|n| n.midi).min().unwrap_or(60);
    let max_pitch = cell.notes.iter().map(|n| n.midi).max().unwrap_or(72);
    let pitch_range = (max_pitch - min_pitch).max(11) as f32; // at least one octave

    let note_col = theme.c(&theme.accent).gamma_multiply(0.7);

    for note in &cell.notes {
        if note.duration_beats <= 0.0 {
            continue;
        }
        let x0 = roll_rect.min.x + (note.beat as f32 / loop_len) * roll_rect.width();
        let x1 = (roll_rect.min.x
            + ((note.beat + note.duration_beats) as f32 / loop_len) * roll_rect.width())
        .min(roll_rect.max.x);
        let pitch_frac = (note.midi - min_pitch) as f32 / pitch_range;
        let y = roll_rect.max.y - pitch_frac * roll_rect.height();
        let note_h = (roll_rect.height() / pitch_range).clamp(2.0, 6.0);
        let note_rect = Rect::from_min_size(
            egui::pos2(x0, y - note_h / 2.0),
            Vec2::new((x1 - x0).max(2.0), note_h),
        );
        painter.rect_filled(note_rect, CornerRadius::same(1), note_col);
    }
}
