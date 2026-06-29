use crate::patch::Patch;
use crate::ui::design::toggle::{toggle_button, ToggleSize};
use crate::ui::design::Tier;
use crate::ui::theme::SynthTheme;
use crate::SynthApp;
use eframe::egui;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Tag groups shown in the filter sidebar
// ---------------------------------------------------------------------------

const INSPIRED_BY_TAGS: &[&str] = &["eno", "pink-floyd", "frahm", "zimmer", "glass", "wagner"];

const CHARACTER_TAGS: &[&str] = &[
    "warm",
    "dark",
    "bright",
    "cold",
    "lush",
    "raw",
    "soft",
    "aggressive",
    "evolving",
    "drone",
    "pulsing",
    "rhythmic",
    "glitchy",
];

const TIMBRE_TAGS: &[&str] = &[
    "analog",
    "digital",
    "fm",
    "bell",
    "choir",
    "strings",
    "noise",
    "plucked",
    "long-release",
    "short-attack",
];

const GENRE_TAGS: &[&str] = &["electronic", "synthwave", "rock", "cinematic"];

// Fixed width of the left filter sidebar.
const SIDEBAR_W: f32 = 140.0;

// Fixed width of the category chip in each patch row (no layout shift).
const CATEGORY_COL_W: f32 = 80.0;

// ---------------------------------------------------------------------------

impl SynthApp {
    #[allow(dead_code)]
    pub fn ui_patch_bar(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let theme = &theme;

        ui.label(
            egui::RichText::new("PATCH")
                .font(theme.font_small())
                .strong()
                .color(theme.c(&theme.text_secondary)),
        );

        ui.add(egui::TextEdit::singleline(&mut self.patch_name).desired_width(120.0))
            .on_hover_text("Patch name. Used as filename when saving.");

        if ui
            .button(
                egui::RichText::new("SAVE")
                    .font(theme.font_body())
                    .color(theme.c(&theme.text_primary)),
            )
            .on_hover_text("Save current patch to a JSON file.")
            .clicked()
        {
            let p = self.capture_patch();
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("{}.json", p.name))
                .add_filter("Patch", &["json"])
                .save_file()
            {
                if let Ok(json) = serde_json::to_string_pretty(&p) {
                    let _ = std::fs::write(path, json);
                }
            }
        }

        if ui
            .button(
                egui::RichText::new("LOAD")
                    .font(theme.font_body())
                    .color(theme.c(&theme.text_primary)),
            )
            .on_hover_text("Load a patch from a JSON file.")
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Patch", &["json"])
                .pick_file()
            {
                if let Ok(json) = std::fs::read_to_string(path) {
                    if let Ok(p) = serde_json::from_str::<Patch>(&json) {
                        self.apply_patch(p);
                    }
                }
            }
        }

        let browser_col = if self.patch_browser_open {
            theme.c(&theme.accent)
        } else {
            theme.c(&theme.text_primary)
        };
        if ui
            .button(
                egui::RichText::new("PATCH")
                    .font(theme.font_body())
                    .color(browser_col),
            )
            .on_hover_text("Browse factory patches by category, tags, and favourites.")
            .clicked()
        {
            self.patch_browser_open = !self.patch_browser_open;
        }
    }

    pub fn ui_patch_browser(&mut self, ctx: &egui::Context) {
        if !self.patch_browser_open {
            return;
        }

        let mut open = self.patch_browser_open;
        egui::Window::new("Patch Library")
            .id(egui::Id::new("patch_browser_w1")) // versioned ID: resets cached size on change
            .open(&mut open)
            .resizable(true)
            .default_size([800.0, 900.0])
            .min_size([520.0, 400.0])
            .show(ctx, |ui| {
                self.patch_browser_inner(ui);
            });
        self.patch_browser_open = open;
    }

    fn patch_browser_inner(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme.clone();
        let accent = theme.c(&theme.accent);
        let text_sec = theme.c(&theme.text_secondary);
        let text_dis = theme.c(&theme.text_disabled);

        // ── Pre-compute derived data (immutable borrows only) ─────────────
        let categories: Vec<String> = {
            let mut cats = vec!["All".to_string()];
            let mut seen = std::collections::HashSet::new();
            for p in &self.patch_library {
                if seen.insert(p.category.clone()) {
                    cats.push(p.category.clone());
                }
            }
            cats.sort_by(|a, b| {
                if a == "All" {
                    std::cmp::Ordering::Less
                } else if b == "All" {
                    std::cmp::Ordering::Greater
                } else {
                    a.cmp(b)
                }
            });
            cats
        };

        let all_used_tags: HashSet<String> = self
            .patch_library
            .iter()
            .flat_map(|p| p.tags.iter().cloned())
            .collect();

        let search_lc = self.patch_search.to_lowercase();

        let filtered: Vec<usize> = self
            .patch_library
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                let cat_ok = self.patch_browser_category == "All"
                    || p.category == self.patch_browser_category;
                let search_ok = search_lc.is_empty()
                    || p.name.to_lowercase().contains(&search_lc)
                    || p.category.to_lowercase().contains(&search_lc)
                    || p.tags.iter().any(|t| t.contains(&search_lc));
                let tag_ok = self.patch_active_tags.is_empty()
                    || self.patch_active_tags.iter().all(|t| p.tags.contains(t));
                cat_ok && search_ok && tag_ok
            })
            .map(|(i, _)| i)
            .collect();

        // ── Mutation collectors (applied after rendering) ─────────────────
        let mut load_idx: Option<usize> = None;
        let mut toggle_fav: Option<String> = None;
        let mut click_category: Option<String> = None;
        let mut toggle_tag: Option<String> = None;
        let mut do_clear = false;

        // ── Top bar — full width, always one row ───────────────────────────
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("🔍")
                    .font(theme.font_body())
                    .color(text_sec),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.patch_search)
                    .hint_text("Search patches…")
                    .desired_width(200.0),
            );
            if !self.patch_search.is_empty() && ui.small_button("✕").clicked() {
                self.patch_search.clear();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                toggle_button(
                    ui,
                    &mut self.patch_load_fx,
                    "LOAD FX",
                    ToggleSize::Standard,
                    Tier::Tertiary,
                    &theme,
                    None,
                )
                .on_hover_text("When on, loading a patch also restores its FX chain.");

                if !self.patch_active_tags.is_empty()
                    && ui
                        .button(
                            egui::RichText::new("Clear filters")
                                .font(theme.font_small())
                                .color(theme.c(&theme.accent_hard_sync)),
                        )
                        .clicked()
                {
                    do_clear = true;
                }
            });
        });

        ui.separator();

        // Height the two-column body must fill. Capturing available height
        // here (after the top bar) and forcing the columns to it stops the
        // window auto-shrinking to the scroll-area content.
        let body_h = ui.available_height();

        // ── Two-column body ────────────────────────────────────────────────
        ui.horizontal(|ui| {
            // ── Left sidebar — fixed width, fully scrollable ───────────────
            ui.vertical(|ui| {
                ui.set_width(SIDEBAR_W);
                ui.set_height(body_h);
                egui::ScrollArea::vertical()
                    .id_salt("pb_sidebar")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(SIDEBAR_W);

                        // Category list
                        ui.label(
                            egui::RichText::new("CATEGORY")
                                .font(theme.font_micro())
                                .color(text_dis),
                        );
                        for cat in &categories {
                            let active = &self.patch_browser_category == cat;
                            let col = if active { accent } else { text_sec };
                            if ui
                                .selectable_label(
                                    active,
                                    egui::RichText::new(cat).font(theme.font_body()).color(col),
                                )
                                .clicked()
                            {
                                click_category = Some(cat.clone());
                            }
                        }

                        ui.add_space(theme.sp_sm);

                        // Tag groups — free function to avoid closure borrow conflict
                        let colors = SidebarColors {
                            accent,
                            text_sec,
                            text_dis,
                        };
                        for (group_label, group_tags) in &[
                            ("INSPIRED BY", INSPIRED_BY_TAGS),
                            ("CHARACTER", CHARACTER_TAGS),
                            ("TIMBRE", TIMBRE_TAGS),
                            ("GENRE", GENRE_TAGS),
                        ] {
                            if let Some(t) = sidebar_tag_group(
                                ui,
                                group_label,
                                group_tags,
                                &all_used_tags,
                                &self.patch_active_tags,
                                &colors,
                                &theme,
                            ) {
                                toggle_tag = Some(t);
                            }
                        }
                    });
            });

            ui.separator();

            // ── Right panel — patch count + scrollable list ────────────────
            ui.vertical(|ui| {
                ui.set_height(body_h);
                let total = self.patch_library.len();
                let showing = filtered.len();
                ui.label(
                    egui::RichText::new(format!("{showing} / {total} patches"))
                        .font(theme.font_small())
                        .color(text_sec),
                );
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("pb_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Favourites
                        let fav_indices: Vec<usize> = filtered
                            .iter()
                            .copied()
                            .filter(|&i| self.patch_favorites.contains(&self.patch_library[i].name))
                            .collect();

                        if !fav_indices.is_empty() {
                            ui.label(
                                egui::RichText::new("★  Favourites")
                                    .font(theme.font_small())
                                    .color(theme.c(&theme.accent_hold)),
                            );
                            for i in &fav_indices {
                                if let Some(action) =
                                    Self::patch_row(ui, &self.patch_library[*i], true, &theme)
                                {
                                    match action {
                                        PatchAction::Load => load_idx = Some(*i),
                                        PatchAction::ToggleFav(n) => toggle_fav = Some(n),
                                    }
                                }
                            }
                            ui.separator();
                        }

                        // Recent
                        let recent_indices: Vec<usize> = self
                            .patch_recent
                            .iter()
                            .filter_map(|name| {
                                filtered
                                    .iter()
                                    .copied()
                                    .find(|&i| &self.patch_library[i].name == name)
                            })
                            .take(6)
                            .collect();

                        if !recent_indices.is_empty() {
                            ui.label(
                                egui::RichText::new("⏱  Recent")
                                    .font(theme.font_small())
                                    .color(text_sec),
                            );
                            for i in &recent_indices {
                                let is_fav =
                                    self.patch_favorites.contains(&self.patch_library[*i].name);
                                if let Some(action) =
                                    Self::patch_row(ui, &self.patch_library[*i], is_fav, &theme)
                                {
                                    match action {
                                        PatchAction::Load => load_idx = Some(*i),
                                        PatchAction::ToggleFav(n) => toggle_fav = Some(n),
                                    }
                                }
                            }
                            ui.separator();
                        }

                        // All filtered
                        for i in &filtered {
                            let is_fav =
                                self.patch_favorites.contains(&self.patch_library[*i].name);
                            if let Some(action) =
                                Self::patch_row(ui, &self.patch_library[*i], is_fav, &theme)
                            {
                                match action {
                                    PatchAction::Load => load_idx = Some(*i),
                                    PatchAction::ToggleFav(n) => toggle_fav = Some(n),
                                }
                            }
                        }
                    });
            });
        });

        // ── Apply mutations ────────────────────────────────────────────────
        if do_clear {
            self.patch_active_tags.clear();
            self.patch_browser_category = "All".into();
        }
        if let Some(cat) = click_category {
            self.patch_browser_category = cat;
        }
        if let Some(tag) = toggle_tag {
            if self.patch_active_tags.contains(&tag) {
                self.patch_active_tags.remove(&tag);
            } else {
                self.patch_active_tags.insert(tag);
            }
        }
        if let Some(name) = toggle_fav {
            if self.patch_favorites.contains(&name) {
                self.patch_favorites.remove(&name);
            } else {
                self.patch_favorites.insert(name);
            }
        }
        if let Some(idx) = load_idx {
            let p = self.patch_library[idx].clone();
            self.apply_patch(p);
            self.patch_browser_open = false;
        }
    }

    /// One patch row. Fixed-width category column so the name never shifts.
    fn patch_row(
        ui: &mut egui::Ui,
        patch: &Patch,
        is_fav: bool,
        theme: &SynthTheme,
    ) -> Option<PatchAction> {
        let mut action = None;
        let text_sec = theme.c(&theme.text_secondary);
        let text_pri = theme.c(&theme.text_primary);
        let fav_col = theme.c(&theme.accent_hold);
        let no_fav_col = theme.c(&theme.text_disabled);

        ui.horizontal(|ui| {
            // Favourite star
            let star = if is_fav { "★" } else { "☆" };
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(star)
                            .font(theme.font_body())
                            .color(if is_fav { fav_col } else { no_fav_col }),
                    )
                    .frame(false),
                )
                .clicked()
            {
                action = Some(PatchAction::ToggleFav(patch.name.clone()));
            }

            // Category chip — fixed width prevents name column shift
            ui.add_sized(
                [CATEGORY_COL_W, theme.font_small().size + 4.0],
                egui::Label::new(
                    egui::RichText::new(format!("[{}]", patch.category))
                        .font(theme.font_small())
                        .color(text_sec),
                ),
            );

            // Patch name
            if ui
                .selectable_label(
                    false,
                    egui::RichText::new(&patch.name)
                        .font(theme.font_body())
                        .color(text_pri),
                )
                .clicked()
            {
                action = Some(PatchAction::Load);
            }

            // Tags — right-aligned
            if !patch.tags.is_empty() {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(patch.tags.join(" · "))
                            .font(theme.font_small())
                            .color(text_sec),
                    );
                });
            }
        });

        action
    }
}

struct SidebarColors {
    accent: egui::Color32,
    text_sec: egui::Color32,
    text_dis: egui::Color32,
}

/// Render one tag group in the sidebar as a vertical list.
/// Returns the tag that was clicked (if any).
fn sidebar_tag_group(
    ui: &mut egui::Ui,
    label: &str,
    tags: &[&str],
    all_used: &HashSet<String>,
    active_tags: &HashSet<String>,
    colors: &SidebarColors,
    theme: &SynthTheme,
) -> Option<String> {
    let relevant: Vec<&str> = tags
        .iter()
        .copied()
        .filter(|t| all_used.contains(*t))
        .collect();
    if relevant.is_empty() {
        return None;
    }

    let mut clicked = None;

    ui.label(
        egui::RichText::new(label)
            .font(theme.font_micro())
            .color(colors.text_dis),
    );
    for &tag in &relevant {
        let active = active_tags.contains(tag);
        let col = if active {
            colors.accent
        } else {
            colors.text_sec
        };
        if ui
            .selectable_label(
                active,
                egui::RichText::new(tag).font(theme.font_body()).color(col),
            )
            .clicked()
        {
            clicked = Some(tag.to_string());
        }
    }
    ui.add_space(theme.sp_sm);

    clicked
}

enum PatchAction {
    Load,
    ToggleFav(String),
}
