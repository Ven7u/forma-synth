//! Layer 2 — SynthUi extension trait.
//!
//! Adds synth-specific layout helpers and component entry points to
//! `egui::Ui`. Panels never construct widgets directly; they call
//! `ui.synth_knob(...)`, `ui.knob_row(|ui| ...)`, etc.
//!
//! Phase 2 implements `synth_knob` and `knob_row`. The remaining methods
//! (`synth_toggle`, `chip_row`, `section_header`, ...) land in Phase 5+
//! when the first migrated panel exercises them.

use egui::{CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use super::{
    chip::chip_selector,
    fader::{fader, FaderOrientation, FaderSize},
    knob::knob,
    level_meter::{level_meter, LevelMeterOrientation, LevelMeterSize},
    mini_bar::{MiniBar, MiniBarOrientation},
    section::section_header,
    step_pad::{step_pad, StepPadSize, StepState},
    toggle::{toggle_button, ToggleSize},
    KnobSize, Tier,
};
use crate::ui::frame::SynthFrame;
use crate::ui::theme::SynthTheme;

pub trait SynthUi {
    /// Render a tokenized knob. Picks dimensions from `size` and arc color
    /// from `tier`.
    fn synth_knob(
        &mut self,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        label: &str,
        theme: &SynthTheme,
        logarithmic: bool,
        size: KnobSize,
        tier: Tier,
    ) -> Response;

    /// Lay out a row of knobs (or any contents) with `sp_md` gaps. The
    /// closure receives a horizontal child UI; per-knob equal-width
    /// allocation will land when `knob_row` callers actually exist
    /// (Phase 5+). For now this is `ui.horizontal()` with the standard gap.
    fn knob_row<R>(&mut self, theme: &SynthTheme, content: impl FnOnce(&mut Ui) -> R) -> R;

    /// Render a step pad — sequencer / drum grid cell.
    fn synth_step_pad(
        &mut self,
        state: StepState,
        size: StepPadSize,
        theme: &SynthTheme,
    ) -> Response;

    /// Render a linear fader.
    fn synth_fader(
        &mut self,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        orientation: FaderOrientation,
        size: FaderSize,
        theme: &SynthTheme,
    ) -> Response;

    /// Render a level meter — peak / VU bar with three color zones and
    /// optional peak-hold line.
    fn synth_level_meter(
        &mut self,
        level: f32,
        peak_hold: f32,
        orientation: LevelMeterOrientation,
        size: LevelMeterSize,
        theme: &SynthTheme,
    ) -> Response;

    /// Render a binary on/off toggle button with a label.
    fn synth_toggle(
        &mut self,
        value: &mut bool,
        label: &str,
        size: ToggleSize,
        tier: Tier,
        theme: &SynthTheme,
        accent: Option<egui::Color32>,
    ) -> Response;

    /// Render a row of mutually-exclusive option chips.
    fn chip_selector<T: Copy + PartialEq>(
        &mut self,
        selected: &mut T,
        options: &[(T, &str)],
        theme: &SynthTheme,
        width: Option<f32>,
    ) -> Response;

    /// Render a labeled section header with an optional right-aligned slot
    /// (toggle, chip, button — caller provides the closure).
    fn section_header<R>(
        &mut self,
        title: &str,
        theme: &SynthTheme,
        right_slot: Option<impl FnOnce(&mut Ui) -> R>,
    ) -> Response;

    /// Render a SectionCard — a SynthFrame wrapping a labeled section.
    /// Tier::Primary uses the accent border (`SynthFrame::tier1`);
    /// Tier::Secondary uses the standard surface; Tier::Tertiary uses
    /// the sunken inset variant.
    fn section_card<R>(
        &mut self,
        title: &str,
        tier: Tier,
        theme: &SynthTheme,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> R;

    /// Render a TieredCard — three vertical zones, Tier 1 on top, Tier 3 on
    /// bottom. Each closure receives a child UI. Each zone is optional;
    /// passing `None` skips it without leaving an empty band.
    fn tiered_card<R1, R2, R3>(
        &mut self,
        theme: &SynthTheme,
        tier1: Option<impl FnOnce(&mut Ui) -> R1>,
        tier2: Option<impl FnOnce(&mut Ui) -> R2>,
        tier3: Option<impl FnOnce(&mut Ui) -> R3>,
    );
}

impl SynthUi for Ui {
    fn synth_knob(
        &mut self,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        label: &str,
        theme: &SynthTheme,
        logarithmic: bool,
        size: KnobSize,
        tier: Tier,
    ) -> Response {
        knob(self, value, range, label, theme, logarithmic, size, tier)
    }

    fn knob_row<R>(&mut self, theme: &SynthTheme, content: impl FnOnce(&mut Ui) -> R) -> R {
        self.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_md;
            content(ui)
        })
        .inner
    }

    fn synth_step_pad(
        &mut self,
        state: StepState,
        size: StepPadSize,
        theme: &SynthTheme,
    ) -> Response {
        step_pad(self, state, size, theme)
    }

    fn synth_fader(
        &mut self,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        orientation: FaderOrientation,
        size: FaderSize,
        theme: &SynthTheme,
    ) -> Response {
        fader(self, value, range, orientation, size, theme)
    }

    fn synth_level_meter(
        &mut self,
        level: f32,
        peak_hold: f32,
        orientation: LevelMeterOrientation,
        size: LevelMeterSize,
        theme: &SynthTheme,
    ) -> Response {
        level_meter(self, level, peak_hold, orientation, size, theme)
    }

    fn synth_toggle(
        &mut self,
        value: &mut bool,
        label: &str,
        size: ToggleSize,
        tier: Tier,
        theme: &SynthTheme,
        accent: Option<egui::Color32>,
    ) -> Response {
        toggle_button(self, value, label, size, tier, theme, accent)
    }

    fn chip_selector<T: Copy + PartialEq>(
        &mut self,
        selected: &mut T,
        options: &[(T, &str)],
        theme: &SynthTheme,
        width: Option<f32>,
    ) -> Response {
        chip_selector(self, selected, options, theme, width)
    }

    fn section_header<R>(
        &mut self,
        title: &str,
        theme: &SynthTheme,
        right_slot: Option<impl FnOnce(&mut Ui) -> R>,
    ) -> Response {
        section_header(self, title, theme, right_slot)
    }

    fn section_card<R>(
        &mut self,
        title: &str,
        tier: Tier,
        theme: &SynthTheme,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> R {
        let frame = match tier {
            Tier::Primary => SynthFrame::tier1(theme),
            Tier::Secondary => SynthFrame::section(theme),
            Tier::Tertiary => SynthFrame::inset(theme),
        };
        let mut out: Option<R> = None;
        frame.show(self, |ui| {
            ui.set_min_width(ui.available_width());
            section_header::<()>(ui, title, theme, None::<fn(&mut Ui)>);
            ui.add_space(theme.sp_xs);
            out = Some(content(ui));
        });
        out.expect("section_card content must run exactly once")
    }

    fn tiered_card<R1, R2, R3>(
        &mut self,
        theme: &SynthTheme,
        tier1: Option<impl FnOnce(&mut Ui) -> R1>,
        tier2: Option<impl FnOnce(&mut Ui) -> R2>,
        tier3: Option<impl FnOnce(&mut Ui) -> R3>,
    ) {
        let mut first = true;
        let mut zone = |ui: &mut Ui, f: Box<dyn FnOnce(&mut Ui)>| {
            if !first {
                ui.add_space(theme.sp_sm);
                ui.separator();
                ui.add_space(theme.sp_sm);
            }
            first = false;
            f(ui);
        };
        if let Some(f) = tier1 {
            zone(self, Box::new(move |ui| {
                let _ = f(ui);
            }));
        }
        if let Some(f) = tier2 {
            zone(self, Box::new(move |ui| {
                let _ = f(ui);
            }));
        }
        if let Some(f) = tier3 {
            zone(self, Box::new(move |ui| {
                let _ = f(ui);
            }));
        }
    }
}

/// FxModule pattern — per `05-patterns.md` §FxModule. A single effect
/// box in the FX chain. Renders:
/// - A fixed-min-width vertical container (caller can override via
///   `min_width`; defaults to 120 px).
/// - A header row with the effect name as a `synth_toggle` using the
///   FX-domain accent color as the active fill.
/// - The caller-provided `content` closure for the effect's parameter
///   widgets (sliders, chip selectors, etc.).
/// - When `*enabled` is false, the body is dimmed via `add_enabled_ui`.
///
/// Returns the header toggle's Response so the caller can detect the
/// click transition and run effect-specific side effects (e.g. resetting
/// tails on delay/reverb engage). The content closure's return value is
/// also propagated.
pub fn fx_module<R>(
    ui: &mut Ui,
    name: &str,
    color: egui::Color32,
    enabled: &mut bool,
    theme: &SynthTheme,
    content: impl FnOnce(&mut Ui) -> R,
) -> (Response, R) {
    let mut header_resp: Option<Response> = None;
    let mut body_out: Option<R> = None;

    ui.group(|ui| {
        // Fill the available width so a vertical stack of fx_modules reads
        // as a uniform column of cards.
        ui.set_min_width(ui.available_width());
        ui.vertical(|ui| {
            let resp = toggle_button(
                ui,
                enabled,
                name,
                ToggleSize::Standard,
                Tier::Secondary,
                theme,
                Some(color),
            );
            header_resp = Some(resp);

            ui.add_space(theme.sp_xxs);
            let was_enabled = *enabled;
            ui.add_enabled_ui(was_enabled, |ui| {
                body_out = Some(content(ui));
            });
        });
    });

    (
        header_resp.expect("fx_module always renders its header"),
        body_out.expect("fx_module always runs its content closure"),
    )
}

/// FaderColumn pattern — per `05-patterns.md` §FaderColumn. Composes a
/// label, a vertical Fader, and an optional LevelMeter into a mixer
/// channel strip. Returns the Fader's Response so callers can detect
/// drag changes and gate engine updates.
///
/// When `meter` is `Some((level, peak_hold))` the meter sits to the
/// right of the fader (Standard size, paired). When `None` only the
/// fader renders — used by the Studio mixer where there's a single
/// shared L/R meter rather than per-channel meters.
///
/// `size` controls the fader length. Defaults usefully to `Standard`
/// for typical channel strips, but real mixers (Ableton, Logic, hardware)
/// use the **same** fader size on channel strips and the master strip
/// so the row reads as visually balanced — distinguish the master via
/// `SynthFrame::tier1` and a paired LevelMeter, not via fader length.
///
/// `enabled` controls the label color only — pass `false` to dim it
/// (e.g. for an oscillator that's been toggled off).
pub fn fader_column(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    meter: Option<(f32, f32)>,
    enabled: bool,
    size: FaderSize,
    theme: &SynthTheme,
) -> Response {
    let mut response = None;
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(label)
                .font(theme.font_small())
                .color(if enabled {
                    theme.c(&theme.text_primary)
                } else {
                    theme.c(&theme.text_disabled)
                }),
        );
        ui.add_space(theme.sp_xxs);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme.sp_xxs;
            let resp = fader(
                ui,
                value,
                range,
                FaderOrientation::Vertical,
                size,
                theme,
            );
            response = Some(resp);
            if let Some((level, peak_hold)) = meter {
                level_meter(
                    ui,
                    level,
                    peak_hold,
                    LevelMeterOrientation::Vertical,
                    LevelMeterSize::Standard,
                    theme,
                );
            }
        });
    });
    response.expect("fader_column always renders the fader")
}

/// State + bindings for one column in the note sequencer. Aggregates
/// all the per-step fields so the `note_seq_step` pattern's signature
/// stays manageable.
pub struct NoteSeqStepState<'a> {
    /// On/off flag for this step.
    pub is_on: &'a mut bool,
    /// MIDI note (0–127). The pattern's pitch bar drives this via the
    /// caller-owned `drag_accum`.
    pub note: &'a mut u8,
    /// Pitch-bar drag accumulator — persisted between frames in the
    /// caller's state, reset to 0 on drag-stop by MiniBar.
    pub drag_accum: &'a mut f32,
    /// 0–127 step velocity.
    pub velocity: &'a mut u8,
    /// 0–100 step probability.
    pub probability: &'a mut u8,
    /// Pitch range to map the bar into (typically the SEQ_CHROMATIC bounds).
    pub midi_min: f32,
    pub midi_max: f32,
    /// True when the playhead is currently on this step (highlights it).
    pub is_current: bool,
    /// True when the step-entry record cursor is on this step
    /// (draws the `seq_rec_cursor` border).
    pub is_rec_cursor: bool,
}

/// What the user did on this step this frame.
#[derive(Default)]
pub struct NoteSeqStepEvents {
    /// User clicked the on/off pad — caller should toggle `is_on` in
    /// shared state.
    pub pad_clicked: bool,
    /// True if any of the four bars caused a value to change this frame
    /// (caller checks `*state.note`, `*state.velocity`, `*state.probability`
    /// for the new values).
    pub changed: bool,
}

/// NoteSeqStep pattern — composes the four widgets that make up one
/// column of the note sequencer:
///   ┌─────┐   ← pitch MiniBar (vertical, drag-delta, shows note name)
///   │ E3  │
///   └─────┘
///   ┌─────┐   ← on/off StepPad (custom rendered because of the
///   │     │     length-aware alpha + rec-cursor border)
///   └─────┘
///   ┌─────┐   ← velocity MiniBar (horizontal, absolute drag, value text)
///   └─────┘
///   ┌───┐     ← probability MiniBar (horizontal, absolute drag,
///   └───┘       3-zone color)
///
/// Returns the events; the caller is responsible for writing them
/// back into the shared sequencer state (under lock).
///
/// `note_label` is rendered into the pitch bar. The caller derives it
/// from `*state.note` (typically via `crate::ui::midi_note_name`) before
/// calling to keep borrow rules simple.
pub fn note_seq_step(
    ui: &mut Ui,
    state: NoteSeqStepState<'_>,
    note_label: &str,
    step_w: f32,
    bar_area_h: f32,
    theme: &SynthTheme,
) -> NoteSeqStepEvents {
    let mut events = NoteSeqStepEvents::default();
    let prev_note = *state.note;
    let prev_vel = *state.velocity;
    let prev_prob = *state.probability;

    ui.vertical(|ui| {
        ui.set_width(step_w);

        // ── Pitch bar ────────────────────────────────────────────────
        // Color picks up the playhead / on / off semantic from theme.
        let pitch_color = if state.is_current {
            theme.c(&theme.seq_current)
        } else if *state.is_on {
            theme.c(&theme.seq_note_bar_on)
        } else {
            theme.c(&theme.seq_note_bar_off)
        };
        let label_color = if *state.is_on {
            theme.c(&theme.text_primary)
        } else {
            theme.c(&theme.text_secondary)
        };

        let mut pitch_f = *state.note as f32;
        let pitch_resp = MiniBar::new(
            &mut pitch_f,
            state.midi_min..=state.midi_max,
            MiniBarOrientation::Vertical,
            Vec2::new(step_w, bar_area_h),
        )
        .fill(pitch_color)
        .bg(theme.c(&theme.bg_seq_bar))
        .label(note_label, theme.font_value(), label_color)
        .drag_delta(state.drag_accum, 0.3)
        .show(ui, theme);
        *state.note = pitch_f.round().clamp(0.0, 127.0) as u8;

        // Rec-cursor border drawn over the pitch bar's rect.
        if state.is_rec_cursor {
            ui.painter().rect_stroke(
                pitch_resp.rect,
                CornerRadius::same(theme.rounding_sm as u8),
                Stroke::new(theme.stroke_active, theme.c(&theme.seq_rec_cursor)),
                StrokeKind::Middle,
            );
        }

        // ── On/off step pad ──────────────────────────────────────────
        let pad_fill = if state.is_current {
            theme.c(&theme.seq_current)
        } else if *state.is_on {
            theme.c(&theme.seq_step_on)
        } else {
            theme.c(&theme.seq_step_off)
        };
        let pad_border = if state.is_current {
            theme.c(&theme.text_primary)
        } else {
            theme.c(&theme.border)
        };
        let (pad_resp, painter) = ui.allocate_painter(Vec2::new(step_w, 28.0), Sense::click());
        painter.rect_filled(
            pad_resp.rect,
            CornerRadius::same(theme.rounding_sm as u8),
            pad_fill,
        );
        painter.rect_stroke(
            pad_resp.rect,
            CornerRadius::same(theme.rounding_sm as u8),
            Stroke::new(theme.stroke_ui, pad_border),
            StrokeKind::Middle,
        );
        if pad_resp.clicked() {
            events.pad_clicked = true;
        }

        // ── Velocity bar ─────────────────────────────────────────────
        let vel_label = format!("{}", *state.velocity);
        let mut vel_f = *state.velocity as f32;
        MiniBar::new(
            &mut vel_f,
            0.0..=127.0,
            MiniBarOrientation::Horizontal,
            Vec2::new(step_w, 14.0),
        )
        .fill(theme.c(&theme.seq_velocity_bar))
        .label(vel_label, theme.font_micro(), theme.c(&theme.text_primary))
        .show(ui, theme);
        *state.velocity = vel_f.round().clamp(0.0, 127.0) as u8;

        // ── Probability bar ──────────────────────────────────────────
        let mut prob_f = *state.probability as f32;
        MiniBar::new(
            &mut prob_f,
            0.0..=100.0,
            MiniBarOrientation::Horizontal,
            Vec2::new(step_w, 10.0),
        )
        .zoned(
            50.0,
            100.0,
            theme.c(&theme.seq_prob_low),
            theme.c(&theme.seq_prob_mid),
            theme.c(&theme.seq_prob_high),
        )
        .show(ui, theme);
        *state.probability = prob_f.round().clamp(0.0, 100.0) as u8;
    });

    events.changed = *state.note != prev_note
        || *state.velocity != prev_vel
        || *state.probability != prev_prob;
    events
}
