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

use super::{knob::knob, KnobSize, Tier};
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
}
