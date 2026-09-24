use crate::db::*;
use crate::models::*;
use crate::ui::components::section_header;
use crate::ui::theme;
use crate::ui::widgets::*;
use crate::TwoCentsApp;
use eframe::egui::{self, Color32, Id, RichText};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Free-function helpers — used by the shared grid renderer (no &self needed)
// ---------------------------------------------------------------------------

pub fn member_color_for(members: &[HouseholdMember], name: &str) -> Color32 {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Color32::TRANSPARENT;
    }
    members
        .iter()
        .find(|m| m.name.eq_ignore_ascii_case(trimmed))
        .map(|m| m.color)
        .unwrap_or(Color32::TRANSPARENT)
}

pub fn category_color_for(categories: &[Category], label: &str) -> Color32 {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Color32::TRANSPARENT;
    }
    find_category_by_label(categories, trimmed)
        .map(|c| c.color)
        .unwrap_or(Color32::TRANSPARENT)
}

// Visuals, but for inline actions in a settings list a subtle outlined
// button reads cleaner than the default filled one. `primary` switches
// to the accent fill.
pub fn styled_button(ui: &mut egui::Ui, label: &str, primary: bool) -> egui::Response {
    if primary {
        let accent = crate::ui::theme::accent();
        let fg = theme::contrast_text(accent);
        ui.add(
            egui::Button::new(egui::RichText::new(label).color(fg))
                .fill(accent)
                .stroke(egui::Stroke::new(crate::ui::theme_tokens::BORDER_W, accent))
                .corner_radius(crate::ui::theme_tokens::RADIUS_SM),
        )
    } else {
        let fill = crate::ui::theme::bg_secondary();
        let stroke = crate::ui::theme::border();
        let fg = crate::ui::theme::fg_primary();
        ui.add(
            egui::Button::new(egui::RichText::new(label).color(fg))
                .fill(fill)
                .stroke(egui::Stroke::new(crate::ui::theme_tokens::BORDER_W, stroke))
                .corner_radius(crate::ui::theme_tokens::RADIUS_SM),
        )
    }
}

// contrast picking lives in theme::contrast_text (single copy).

/// Renders the category picker dropdown menu and returns the picked label, if any.
/// Extracted from TwoCentsApp::ui_category_picker_menu so it can be called without &self.
pub fn category_picker_menu_ui(
    ui: &mut egui::Ui,
    categories: &[Category],
    current: &str,
) -> Option<String> {
    let parents = category_parent_map(categories);
    let mut parent_ids: Vec<i64> = categories
        .iter()
        .filter(|c| c.parent_id.is_none())
        .map(|c| c.id)
        .collect();
    parent_ids.sort_by(|a, b| {
        let na = categories
            .iter()
            .find(|c| c.id == *a)
            .map(|c| c.name.as_str())
            .unwrap_or("");
        let nb = categories
            .iter()
            .find(|c| c.id == *b)
            .map(|c| c.name.as_str())
            .unwrap_or("");
        na.to_lowercase().cmp(&nb.to_lowercase())
    });
    let mut picked: Option<String> = None;
    let mut parent_has_children = HashSet::new();
    for category in categories {
        if let Some(pid) = category.parent_id {
            parent_has_children.insert(pid);
        }
    }
    for parent_id in parent_ids {
        let Some(parent) = categories.iter().find(|c| c.id == parent_id) else {
            continue;
        };
        if !parent_has_children.contains(&parent.id) {
            let label = parent.name.clone();
            if category_picker_row(
                ui,
                &label,
                parent.color,
                current.eq_ignore_ascii_case(&label),
                0.0,
            ) {
                picked = Some(label);
                ui.close();
            }
        } else {
            category_picker_header_row(ui, &parent.name, parent.color);
            let mut sub_ids: Vec<i64> = categories
                .iter()
                .filter(|c| c.parent_id == Some(parent_id))
                .map(|c| c.id)
                .collect();
            sub_ids.sort_by(|a, b| {
                let na = categories
                    .iter()
                    .find(|c| c.id == *a)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                let nb = categories
                    .iter()
                    .find(|c| c.id == *b)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                na.to_lowercase().cmp(&nb.to_lowercase())
            });
            for sub_id in sub_ids {
                let Some(sub) = categories.iter().find(|c| c.id == sub_id) else {
                    continue;
                };
                let label = sub.full_label(&parents);
                if category_sub_picker_row(
                    ui,
                    &sub.name,
                    current.eq_ignore_ascii_case(&label),
                    16.0,
                ) {
                    picked = Some(label);
                    ui.close();
                }
            }
            ui.separator();
        }
    }
    picked
}
impl TwoCentsApp {
    fn reload_color_state(&mut self) {
        let household_id = self.household_id;
        self.categories = load_categories(&self.conn, household_id).unwrap_or_default();
        self.members = load_household_members(&self.conn, household_id).unwrap_or_default();
        self.rebuild_cached_candidates();
    }

    fn set_member_color(&mut self, member_id: i64, color: Color32) {
        self.flush_deferred_expense_commits();
        let household_id = self.household_id;
        let pending = self
            .household_color_pending
            .clone()
            .filter(|pending| !pending.category && pending.target_id == member_id);
        if self.household_color_pending.is_some() && pending.is_none() {
            self.household_color_pending = None;
        }
        let result = if let Some(pending) = pending {
            update_household_undo_action(
                &mut self.conn,
                household_id,
                pending.action_id,
                pending.before.clone(),
                &pending.last_after,
                |tx| update_member_color_db(tx, household_id, member_id, color),
                |before, after| UndoAction::MemberColor { before, after },
            )
        } else {
            let before = match capture_household_state(&self.conn, household_id) {
                Ok(state) => state,
                Err(error) => {
                    self.set_transient_status(error.to_string());
                    return;
                }
            };
            let result = run_household_action(
                &mut self.conn,
                household_id,
                |tx| update_member_color_db(tx, household_id, member_id, color),
                |before, after| UndoAction::MemberColor { before, after },
            );
            if let Ok(Some(action_id)) = &result {
                if let Ok(after) = capture_household_state(&self.conn, household_id) {
                    self.household_color_pending = Some(PendingColorUndo {
                        action_id: *action_id,
                        target_id: member_id,
                        category: false,
                        before,
                        last_after: after,
                    });
                }
            }
            result
        };
        match result {
            Ok(Some(action_id)) => {
                self.reload_color_state();
                if let Some(pending) = &mut self.household_color_pending {
                    if pending.action_id == action_id {
                        if let Ok(state) = capture_household_state(&self.conn, household_id) {
                            pending.last_after = state;
                        }
                    }
                }
            }
            Ok(None) => {
                self.household_color_pending = None;
                self.reload_color_state();
            }
            Err(error) => self.set_transient_status(error.to_string()),
        }
    }

    pub fn open_member_color_popup(&mut self, id: i64, name: String, color: Color32) {
        self.household_color_pending = None;
        self.member_color_popup = Some(MemberColorPopup { id, name, color });
    }

    pub fn ui_member_color_popup(&mut self, ctx: &egui::Context) {
        let mut popup_state = self.member_color_popup.clone();
        let mut color_changed: Option<(i64, Color32)> = None;
        let mut close_requested = false;
        if let Some(popup) = &mut popup_state {
            let title = format!("Member color: {}", popup.name);
            egui::Window::new(title)
                .title_bar(false)
                .id(Id::new("member_color_popup"))
                .collapsible(false)
                .resizable(false)
                .default_width(320.0)
                .frame(crate::ui::components::modal())
                .show(ctx, |ui| {
                    // Heading + muted description, same shape as the Import /
                    // Duplicates / Settings modals; the custom header bar replaces
                    // the OS title bar so modals look consistent across the app.
                    ui.horizontal(|ui| {
                        crate::ui::components::heading_md(ui, "Pick a color");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if window_close_button(ui).clicked() {
                                close_requested = true;
                            }
                        });
                    });
                    crate::ui::components::label_muted(
                        ui,
                        &format!("This color identifies {} in the expense grid.", popup.name),
                    );
                    ui.add_space(crate::ui::theme_tokens::SPACE_3);
                    if accent_color_picker_ui(ui, &mut popup.color) {
                        color_changed = Some((popup.id, popup.color));
                    }
                });
        }
        if let Some((id, color)) = color_changed {
            self.set_member_color(id, color);
        }
        if close_requested || ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.household_color_pending = None;
            self.member_color_popup = None;
        } else {
            self.member_color_popup = popup_state;
        }
    }

    fn set_category_color(&mut self, category_id: i64, color: Color32) {
        self.flush_deferred_expense_commits();
        let household_id = self.household_id;
        let pending = self
            .household_color_pending
            .clone()
            .filter(|pending| pending.category && pending.target_id == category_id);
        if self.household_color_pending.is_some() && pending.is_none() {
            self.household_color_pending = None;
        }
        let result = if let Some(pending) = pending {
            update_household_undo_action(
                &mut self.conn,
                household_id,
                pending.action_id,
                pending.before.clone(),
                &pending.last_after,
                |tx| update_category_color_cascade_db(tx, household_id, category_id, color),
                |before, after| UndoAction::CategoryColor { before, after },
            )
        } else {
            let before = match capture_household_state(&self.conn, household_id) {
                Ok(state) => state,
                Err(error) => {
                    self.set_transient_status(error.to_string());
                    return;
                }
            };
            let result = run_household_action(
                &mut self.conn,
                household_id,
                |tx| update_category_color_cascade_db(tx, household_id, category_id, color),
                |before, after| UndoAction::CategoryColor { before, after },
            );
            if let Ok(Some(action_id)) = &result {
                if let Ok(after) = capture_household_state(&self.conn, household_id) {
                    self.household_color_pending = Some(PendingColorUndo {
                        action_id: *action_id,
                        target_id: category_id,
                        category: true,
                        before,
                        last_after: after,
                    });
                }
            }
            result
        };
        match result {
            Ok(Some(action_id)) => {
                self.reload_color_state();
                if let Some(pending) = &mut self.household_color_pending {
                    if pending.action_id == action_id {
                        if let Ok(state) = capture_household_state(&self.conn, household_id) {
                            pending.last_after = state;
                        }
                    }
                }
            }
            Ok(None) => {
                self.household_color_pending = None;
                self.reload_color_state();
            }
            Err(error) => self.set_transient_status(error.to_string()),
        }
    }

    fn set_category_excluded(&mut self, category_id: i64, excluded: bool) {
        let household_id = self.household_id;
        let result = self.run_household_mutation(
            |tx| {
                tx.execute(
                    "UPDATE categories SET excluded = ?1 WHERE id = ?2 AND household_id = ?3",
                    rusqlite::params![excluded as i64, category_id, household_id],
                )?;
                Ok(())
            },
            |before, after| UndoAction::CategoryExclusion { before, after },
        );
        if let Err(error) = result {
            self.set_transient_status(error);
        }
    }

    pub(crate) fn rename_category_with_undo(
        &mut self,
        category_id: i64,
        name: &str,
    ) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Enter a category name first.".to_string());
        }
        let household_id = self.household_id;
        let result = self.run_household_mutation(
            |tx| rename_category_db(tx, household_id, category_id, &name),
            |before, after| UndoAction::CategoryRename { before, after },
        )?;
        if result {
            self.set_transient_status("Category renamed.");
        }
        Ok(())
    }

    pub(crate) fn delete_category_with_undo(&mut self, category_id: i64) -> Result<(), String> {
        let is_income_root = self
            .categories
            .iter()
            .find(|category| category.id == category_id)
            .is_some_and(|category| category.name.eq_ignore_ascii_case(INCOME_PARENT));
        if is_income_root {
            return Err("The Income category cannot be deleted.".to_string());
        }
        let household_id = self.household_id;
        let result = self.run_household_mutation(
            |tx| delete_category_from_db(tx, household_id, category_id),
            |before, after| UndoAction::CategoryDelete { before, after },
        )?;
        if result {
            self.set_transient_status("Category deleted.");
        }
        Ok(())
    }

    fn add_parent_category(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(());
        }
        let household_id = self.household_id;
        self.run_household_mutation(
            |tx| add_parent_category_db(tx, household_id, name),
            |before, after| UndoAction::CategoryAdd { before, after },
        )?;
        Ok(())
    }

    fn add_subcategory(&mut self, parent_id: i64, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(());
        }
        let household_id = self.household_id;
        self.run_household_mutation(
            |tx| add_subcategory_db(tx, household_id, parent_id, name),
            |before, after| UndoAction::CategoryAdd { before, after },
        )?;
        Ok(())
    }

    pub fn open_category_color_popup(&mut self, category_id: i64, name: String, color: Color32) {
        self.household_color_pending = None;
        self.category_color_popup = Some(CategoryColorPopup {
            id: category_id,
            name,
            color,
        });
    }

    fn ui_category_settings_list(&mut self, ui: &mut egui::Ui) {
        let categories = self.categories.clone();
        let parents = category_parent_map(&categories);
        let parent_ids = sorted_parent_category_ids(&categories);

        let mut delete_target: Option<i64> = None;
        let mut start_rename: Option<(i64, String)> = None;
        let mut commit_rename: Option<(i64, String)> = None;
        let mut cancel_rename = false;
        let mut start_add_sub: Option<i64> = None;
        let mut commit_sub = false;
        let mut cancel_sub = false;
        let mut excluded_toggles: Vec<(i64, bool)> = Vec::new();

        for parent_id in &parent_ids {
            let Some(parent) = categories.iter().find(|c| c.id == *parent_id) else {
                continue;
            };
            ui.horizontal(|ui| {
                if color_swatch_button(ui, parent.color).clicked() {
                    self.open_category_color_popup(parent.id, parent.name.clone(), parent.color);
                }
                ui.label(
                    RichText::new(&parent.name)
                        .strong()
                        .color(crate::ui::theme::accent()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Delete")
                                    .small()
                                    .color(crate::ui::theme::fg_primary()),
                            )
                            .fill(crate::ui::theme::bg_hover())
                            .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::border())),
                        )
                        .clicked()
                        && !parent.name.eq_ignore_ascii_case(INCOME_PARENT)
                    {
                        delete_target = Some(parent.id);
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Rename")
                                    .small()
                                    .color(crate::ui::theme::fg_primary()),
                            )
                            .fill(crate::ui::theme::bg_hover())
                            .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::border())),
                        )
                        .clicked()
                        && !parent.name.eq_ignore_ascii_case(INCOME_PARENT)
                    {
                        start_rename = Some((parent.id, parent.name.clone()));
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("+ Sub")
                                    .small()
                                    .color(crate::ui::theme::fg_primary()),
                            )
                            .fill(crate::ui::theme::bg_hover())
                            .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::border())),
                        )
                        .clicked()
                    {
                        start_add_sub = Some(parent.id);
                    }
                    // user-set exclusion — children inherit via the tree
                    // walk; excluded rows drop out of budgets/analytics/settlements.
                    let mut parent_excluded = parent.excluded;
                    if ui.checkbox(&mut parent_excluded, "Excluded").changed() {
                        excluded_toggles.push((parent.id, parent_excluded));
                    }
                });
            });

            if self.editing_category_id == Some(parent.id) {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.editing_category_name)
                            .id(egui::Id::new(("editing_category_name", parent.id)))
                            .desired_width(180.0),
                    );
                    if response.lost_focus()
                        || (response.has_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                    {
                        commit_rename = Some((parent.id, self.editing_category_name.clone()));
                    }
                    if ui.small_button("Cancel").clicked()
                        || ((response.has_focus() || response.lost_focus())
                            && ui.input(|input| input.key_pressed(egui::Key::Escape)))
                    {
                        cancel_rename = true;
                    }
                });
            }

            // Inline subcategory input row
            if self.adding_subcategory_to == Some(parent.id) {
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.inline_subcategory_name)
                            .id(egui::Id::new(("inline_sub_input_field", parent.id)))
                            .hint_text("Subcategory name")
                            .desired_width(180.0),
                    );
                    if styled_button(ui, "Add", false).clicked()
                        || text_field_enter_pressed(ui, &response)
                    {
                        commit_sub = true;
                    }
                    if ui.small_button("Cancel").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        cancel_sub = true;
                    }
                });
            }

            let mut sub_ids: Vec<i64> = categories
                .iter()
                .filter(|category| category.parent_id == Some(parent.id))
                .map(|category| category.id)
                .collect();
            sub_ids.sort_by(|a, b| {
                let name_a = categories
                    .iter()
                    .find(|c| c.id == *a)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                let name_b = categories
                    .iter()
                    .find(|c| c.id == *b)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                name_a.to_lowercase().cmp(&name_b.to_lowercase())
            });

            for sub_id in sub_ids {
                let Some(sub) = categories.iter().find(|c| c.id == sub_id) else {
                    continue;
                };
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    if color_swatch_button(ui, sub.color).clicked() {
                        self.open_category_color_popup(sub.id, sub.full_label(&parents), sub.color);
                    }
                    ui.label(
                        RichText::new(&sub.name)
                            .color(crate::ui::theme::fg_primary())
                            .small(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Delete")
                                        .small()
                                        .color(crate::ui::theme::fg_primary()),
                                )
                                .fill(crate::ui::theme::bg_hover())
                                .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::border())),
                            )
                            .clicked()
                        {
                            delete_target = Some(sub.id);
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Rename")
                                        .small()
                                        .color(crate::ui::theme::fg_primary()),
                                )
                                .fill(crate::ui::theme::bg_hover())
                                .stroke(egui::Stroke::new(1.0_f32, crate::ui::theme::border())),
                            )
                            .clicked()
                        {
                            start_rename = Some((sub.id, sub.name.clone()));
                        }
                        let mut sub_excluded = sub.excluded;
                        if ui.checkbox(&mut sub_excluded, "Excluded").changed() {
                            excluded_toggles.push((sub.id, sub_excluded));
                        }
                    });
                });
                if self.editing_category_id == Some(sub.id) {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.editing_category_name)
                                .id(egui::Id::new(("editing_category_name", sub.id)))
                                .desired_width(180.0),
                        );
                        if response.lost_focus()
                            || (response.has_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                        {
                            commit_rename = Some((sub.id, self.editing_category_name.clone()));
                        }
                        if ui.small_button("Cancel").clicked()
                            || ((response.has_focus() || response.lost_focus())
                                && ui.input(|input| input.key_pressed(egui::Key::Escape)))
                        {
                            cancel_rename = true;
                        }
                    });
                }
            }
            ui.separator();
        }

        for (category_id, excluded) in excluded_toggles {
            self.set_category_excluded(category_id, excluded);
        }

        if let Some((category_id, name)) = start_rename {
            self.editing_category_id = Some(category_id);
            self.editing_category_name = name;
            ui.ctx().memory_mut(|memory| {
                memory.request_focus(egui::Id::new(("editing_category_name", category_id)))
            });
        }
        if let Some((category_id, name)) = commit_rename {
            if cancel_rename {
                self.editing_category_id = None;
                self.editing_category_name.clear();
            } else {
                match self.rename_category_with_undo(category_id, &name) {
                    Ok(()) => {
                        self.editing_category_id = None;
                        self.editing_category_name.clear();
                    }
                    Err(error) => self.set_transient_status(error),
                }
            }
        } else if cancel_rename {
            self.editing_category_id = None;
            self.editing_category_name.clear();
        }
        if let Some(parent_id) = start_add_sub {
            self.adding_subcategory_to = Some(parent_id);
            self.inline_subcategory_name.clear();
            // The TextEdit renders next frame — egui assigns focus to it then, so
            // the user can type immediately.
            ui.ctx().memory_mut(|mem| {
                mem.request_focus(egui::Id::new(("inline_sub_input_field", parent_id)))
            });
        }
        if commit_sub {
            if let Some(parent_id) = self.adding_subcategory_to {
                let name = self.inline_subcategory_name.trim().to_string();
                if !name.is_empty() {
                    match self.add_subcategory(parent_id, &name) {
                        Ok(()) => {
                            self.adding_subcategory_to = None;
                            self.inline_subcategory_name.clear();
                        }
                        Err(error) => self.set_transient_status(error),
                    }
                }
            }
        }
        if cancel_sub {
            self.adding_subcategory_to = None;
            self.inline_subcategory_name.clear();
        }
        if let Some(category_id) = delete_target {
            if let Some(category) = categories
                .iter()
                .find(|category| category.id == category_id)
            {
                let mut descendant_ids = vec![category_id];
                let mut descendant_index = 0;
                while descendant_index < descendant_ids.len() {
                    let parent_id = descendant_ids[descendant_index];
                    for child in categories
                        .iter()
                        .filter(|candidate| candidate.parent_id == Some(parent_id))
                    {
                        if !descendant_ids.contains(&child.id) {
                            descendant_ids.push(child.id);
                        }
                    }
                    descendant_index += 1;
                }
                self.open_destructive_confirmation(DestructiveConfirm::Category {
                    id: category.id,
                    label: category.full_label(&parents),
                    descendant_count: descendant_ids.len().saturating_sub(1),
                });
            }
        }
    }

    // the old floating "Settings" window now lives inline on the
    // Household tab, wrapped in a household_panel by the caller.
    pub fn ui_category_settings_section(&mut self, ui: &mut egui::Ui) {
        let mut add_parent = false;
        let mut add_response: Option<egui::Response> = None;
        ui.horizontal(|ui| {
            let parent_name_response = ui.add(
                egui::TextEdit::singleline(&mut self.new_parent_category_name)
                    .id(settings_new_parent_name_id())
                    .hint_text("e.g. Home — press Enter")
                    .desired_width(220.0),
            );
            if styled_button(ui, "Add parent", false).clicked()
                || text_field_enter_pressed(ui, &parent_name_response)
            {
                add_parent = true;
                add_response = Some(parent_name_response);
            }
        });
        ui.add_space(crate::ui::theme_tokens::SPACE_4);
        ui.separator();
        section_header(ui, "Your categories");
        egui::ScrollArea::vertical()
            .id_salt("category_settings_scroll")
            .auto_shrink([false, false])
            .max_height(320.0)
            .show(ui, |ui| {
                self.ui_category_settings_list(ui);
            });
        if add_parent {
            let name = self.new_parent_category_name.trim().to_string();
            if !name.is_empty() {
                match self.add_parent_category(&name) {
                    Ok(()) => {
                        self.new_parent_category_name.clear();
                        if let Some(response) = add_response {
                            response.request_focus();
                        }
                    }
                    Err(error) => self.set_transient_status(error),
                }
            }
        }
    }

    pub fn ui_category_color_popup(&mut self, ctx: &egui::Context) {
        let mut popup_state = self.category_color_popup.clone();
        let mut color_changed: Option<(i64, Color32)> = None;
        let mut close_requested = false;
        if let Some(popup) = &mut popup_state {
            let title = format!("Category color: {}", popup.name);
            egui::Window::new(title)
                .title_bar(false)
                .id(Id::new("category_color_popup"))
                .collapsible(false)
                .resizable(false)
                .default_width(320.0)
                .frame(crate::ui::components::modal())
                .show(ctx, |ui| {
                    // heading + muted description, Notion hierarchy. Same
                    // shape as the member-color picker and the other modals.
                    ui.horizontal(|ui| {
                        crate::ui::components::heading_md(ui, "Pick a color");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if window_close_button(ui).clicked() {
                                close_requested = true;
                            }
                        });
                    });
                    crate::ui::components::label_muted(
                        ui,
                        &format!("This color identifies {} in the expense grid.", popup.name),
                    );
                    ui.add_space(crate::ui::theme_tokens::SPACE_3);
                    if accent_color_picker_ui(ui, &mut popup.color) {
                        color_changed = Some((popup.id, popup.color));
                    }
                });
        }
        if let Some((id, color)) = color_changed {
            self.set_category_color(id, color);
        }
        if close_requested || ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.household_color_pending = None;
            self.category_color_popup = None;
        } else {
            self.category_color_popup = popup_state;
        }
    }

    pub fn ui_delete_confirmations(&mut self, ctx: &egui::Context) {
        if let Some(confirmation) = self.destructive_confirm.clone() {
            let mut open = true;
            let mut confirmed = false;
            let (title, message, action_label) = match &confirmation {
                DestructiveConfirm::Category {
                    label,
                    descendant_count,
                    ..
                } => {
                    let scope = if *descendant_count == 0 {
                        "this category".to_string()
                    } else {
                        format!("this category and {descendant_count} subcategory/subcategories")
                    };
                    (
                        "Delete category?",
                        format!(
                            "Delete {scope} ({label})? Linked expenses, vendor rules, and split rows will be cleared. You can restore this with Ctrl+Z."
                        ),
                        "Delete",
                    )
                }
                DestructiveConfirm::Member { name, .. } => (
                    "Remove member?",
                    format!(
                        "Remove {name}? Their expenses will be unassigned and split rows removed. You can restore this with Ctrl+Z."
                    ),
                    "Remove",
                ),
                DestructiveConfirm::Settlement {
                    label,
                    include_subcategories,
                    descendant_count,
                } => {
                    let scope = if *include_subcategories && *descendant_count > 0 {
                        format!("{label} and {descendant_count} subcategory/subcategories")
                    } else {
                        label.clone()
                    };
                    (
                        "Clear settlement splits?",
                        format!(
                            "Clear all split percentages for {scope}? You can restore this with Ctrl+Z."
                        ),
                        "Clear",
                    )
                }
            };
            let text_width = ctx.fonts_mut(|fonts| {
                let title_width = fonts
                    .layout_no_wrap(
                        title.to_string(),
                        egui::FontId::proportional(15.0),
                        theme::fg_primary(),
                    )
                    .size()
                    .x;
                let message_width = fonts
                    .layout_no_wrap(
                        message.clone(),
                        egui::FontId::proportional(14.0),
                        theme::fg_primary(),
                    )
                    .size()
                    .x;
                title_width.max(message_width)
            });
            let content_width = text_width.max(220.0).clamp(220.0, 400.0);
            let frame = egui::Frame::window(&ctx.global_style())
                .fill(ctx.global_style().visuals.panel_fill)
                .stroke(egui::Stroke::new(2.0_f32, theme::error()))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(10, 16));
            egui::Area::new(Id::new((
                "destructive_confirmation_compact",
                self.destructive_dialog_generation,
            )))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(content_width);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if window_close_button(ui).clicked() {
                            open = false;
                        }
                    });
                    ui.vertical_centered(|ui| {
                        crate::ui::components::heading_md(ui, title);
                        ui.add_space(8.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(message.clone())
                                    .font(egui::FontId::proportional(14.0))
                                    .color(crate::ui::theme::fg_primary()),
                            )
                            .wrap(),
                        );
                        ui.add_space(16.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if styled_button(ui, "Cancel", false).clicked() {
                                open = false;
                            }
                            ui.add_space(12.0);
                            let style = crate::ui::components::button_danger(ui);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(action_label).color(style.text_color),
                                    )
                                    .fill(style.fill)
                                    .stroke(style.stroke),
                                )
                                .clicked()
                            {
                                confirmed = true;
                            }
                        });
                    });
                });
            });
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                open = false;
            }
            if !open {
                self.destructive_confirm = None;
            } else if confirmed {
                self.destructive_confirm = None;
                if let Err(error) = self.confirm_destructive_action(confirmation) {
                    self.set_transient_status(error);
                }
            }
        }

        if self.show_delete_expense_confirm {
            let mut open = self.show_delete_expense_confirm;
            let row_count = self.delete_expense_indices.len();

            egui::Window::new("Confirm Deletion")
                .order(egui::Order::Foreground)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .default_width(320.0)
                .frame(
                    egui::Frame::window(&ctx.global_style())
                        .fill(ctx.global_style().visuals.panel_fill)
                        .stroke(egui::Stroke::new(2.0_f32, theme::error()))
                        .corner_radius(10.0)
                        .inner_margin(egui::Margin::symmetric(16, 16)),
                )
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if window_close_button(ui).clicked() {
                            open = false;
                        }
                    });
                    ui.vertical_centered(|ui| {
                        // Heading + secondary text instead of a lone warning label.
                        crate::ui::components::heading_md(ui, "Confirm Deletion");
                        ui.add_space(crate::ui::theme_tokens::SPACE_2);
                        ui.label(
                            RichText::new(format!(
                "Are you sure you want to permanently delete the selected {} expense row(s)?",
                row_count
              ))
                            .font(egui::FontId::proportional(14.0))
                            .color(crate::ui::theme::fg_primary()),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("This action cannot be undone.")
                                .font(egui::FontId::proportional(11.0))
                                .color(crate::ui::theme::fg_secondary()),
                        );
                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            ui.columns(2, |cols| {
                                // Delete on the left per user preference.
                                cols[0].vertical_centered(|ui| {
                                    let err = theme::error();
                                    let on_err = theme::contrast_text(err);
                                    let confirm_btn = egui::Button::new(
                                        RichText::new("Delete").strong().color(on_err),
                                    )
                                    .fill(err);
                                    if ui.add(confirm_btn).clicked() {
                                        self.show_delete_expense_confirm = false;
                                        let targets =
                                            std::mem::take(&mut self.delete_expense_indices);
                                        self.delete_expenses_by_indices(&targets);
                                    }
                                });
                                cols[1].vertical_centered(|ui| {
                                    if styled_button(ui, "Cancel", false).clicked() {
                                        self.show_delete_expense_confirm = false;
                                        self.delete_expense_indices.clear();
                                    }
                                });
                            });
                        });
                    });
                });
            self.show_delete_expense_confirm = open && self.show_delete_expense_confirm;
        }

        if self.show_delete_import_confirm {
            let mut open = self.show_delete_import_confirm;
            let row_count = self.delete_import_indices.len();

            egui::Window::new("Remove staged rows?")
                .order(egui::Order::Foreground)
                .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_width(320.0)
        .frame(
          egui::Frame::window(&ctx.global_style())
            .fill(ctx.global_style().visuals.panel_fill)
            .stroke(egui::Stroke::new(2.0_f32, ctx.global_style().visuals.selection.bg_fill))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(16, 16)),
        )
        .show(ctx, |ui| {
          ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            if window_close_button(ui).clicked() {
              open = false;
            }
          });
          ui.vertical_centered(|ui| {
            ui.label(
              RichText::new("Remove staged rows")
                .font(egui::FontId::proportional(18.0))
                .strong()
                .color(crate::ui::theme::accent()),
            );
            ui.add_space(8.0);
            ui.label(
              RichText::new(format!(
                "Are you sure you want to remove the selected {} row(s) from the import review list?",
                row_count
              ))
              .font(egui::FontId::proportional(14.0))
              .color(crate::ui::theme::fg_primary()),
            );
            ui.add_space(6.0);
            ui.label(
              RichText::new("These rows will not be saved into your database.")
                .font(egui::FontId::proportional(11.0))
                .color(crate::ui::theme::fg_secondary()),
            );
            ui.add_space(16.0);

            ui.horizontal(|ui| {
              ui.columns(2, |cols| {
                // Remove on the left per user preference.
                cols[0].vertical_centered(|ui| {
                  let sel_bg = crate::ui::theme::accent();
                  let on_sel = theme::contrast_text(sel_bg);
                  let confirm_btn = egui::Button::new(
                    RichText::new("Remove").strong().color(on_sel),
                  )
                  .fill(sel_bg);
                  if ui.add(confirm_btn).clicked() {
                    self.show_delete_import_confirm = false;
                    let targets = std::mem::take(&mut self.delete_import_indices);
                    self.delete_import_rows_by_indices(&targets);
                  }
                });
                cols[1].vertical_centered(|ui| {
                  if ui.button("Cancel").clicked() {
                    self.show_delete_import_confirm = false;
                    self.delete_import_indices.clear();
                  }
                });
              });
            });
          });
        });
            self.show_delete_import_confirm = open && self.show_delete_import_confirm;
        }
    }
}

fn settings_new_parent_name_id() -> Id {
    Id::new("settings_new_parent_category_name")
}

/// egui's title-bar X is a ~16px hit target — too precise to click.
/// This paints a 28×24 close glyph with a full-rect click area. Windows drop
/// `.open()` and render this in their content instead. The X is drawn with
/// two line segments — the ✕ text glyph is missing from egui's default fonts
/// (rendered as a blank square).
pub fn window_close_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 24.0), egui::Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, crate::ui::theme::bg_hover());
    }
    let fg = if response.hovered() {
        crate::ui::theme::fg_primary()
    } else {
        crate::ui::theme::fg_secondary()
    };
    let c = rect.center();
    let r = 5.0;
    ui.painter().line_segment(
        [egui::pos2(c.x - r, c.y - r), egui::pos2(c.x + r, c.y + r)],
        egui::Stroke::new(1.5_f32, fg),
    );
    ui.painter().line_segment(
        [egui::pos2(c.x - r, c.y + r), egui::pos2(c.x + r, c.y - r)],
        egui::Stroke::new(1.5_f32, fg),
    );
    response.on_hover_text("Close")
}

fn grid_cell_active(response: &egui::Response) -> bool {
    response.has_focus() || response.lost_focus()
}

fn text_field_enter_pressed(ui: &egui::Ui, response: &egui::Response) -> bool {
    if !grid_cell_active(response) {
        return false;
    }
    ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
}
