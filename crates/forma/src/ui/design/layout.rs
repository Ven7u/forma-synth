//! Layer 2 — SynthUi extension trait.
//!
//! Adds synth-specific layout helpers and component entry points to
//! `egui::Ui`. Panels never construct widgets directly; they call
//! `ui.synth_knob(...)`, `ui.knob_row(|ui| ...)`, etc.
//!
//! Phase 2 implements `synth_knob` and `knob_row`. The remaining methods
//! (`synth_toggle`, `chip_row`, `section_header`, ...) land in Phase 5+
//! when the first migrated panel exercises them.

use egui::{Response, Ui};

use super::{
    chip::chip_selector,
    fader::{fader, FaderOrientation, FaderSize},
    knob::knob,
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
