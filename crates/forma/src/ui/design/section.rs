//! SectionHeader component — Layer 3.
//!
//! Per `04-components.md` §SectionHeader: title text + optional right-aligned
//! slot. Standard panel-section header. Title uses `font_heading`
//! with `text_primary`; the right slot accepts any closure (toggle, chip,
//! button, etc.).

use egui::{Response, Ui};

use crate::ui::theme::SynthTheme;

/// Render a section header. The optional `right_slot` runs inside a
/// right-aligned sub-ui, so callers can drop in a toggle / chip / button.
pub fn section_header<R>(
    ui: &mut Ui,
    title: &str,
    theme: &SynthTheme,
    right_slot: Option<impl FnOnce(&mut Ui) -> R>,
) -> Response {
    let resp = ui
        .horizontal(|ui| {
            ui.add_space(theme.sp_md);
            ui.label(
                egui::RichText::new(title)
                    .font(theme.font_heading())
                    .color(theme.c(&theme.text_primary)),
            );
            if let Some(slot) = right_slot {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(theme.sp_md);
                    slot(ui);
                });
            }
        })
        .response;
    resp
}
