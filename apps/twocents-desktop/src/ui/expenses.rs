use eframe::egui::{self, RichText};
use rusqlite::params;
use crate::models::*;

use crate::db::*;
use crate::ui::widgets::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_expenses(&mut self, ui: &mut egui::Ui) {
    let prev_edit_cell = self.expense_grid_edit_cell;
    let toolbar_width = ui.available_width();
    ui.horizontal(|ui| {
      ui.set_width(toolbar_width);
      ui.heading(RichText::new("Household Expense Spreadsheet").color(ui.visuals().strong_text_color()));
      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if self.csv_import_rx.is_some() {
          let _ = ui.button("Importing...").on_hover_text("Waiting for file selection");
        } else if ui.button("Import CSV Statement").clicked() {
          self.import_csv();
        }
        if ui.button("Settings").clicked() {
          self.show_category_settings = true;
        }
      });
    });
    ui.label(
      "Click or drag in a column to select rows. Hold Shift and drag to pan. Type to edit; Enter applies to all selected rows; Tab or Enter picks autocomplete and moves right; Escape clears.",
    );
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
      self.clear_expense_grid_selection();
    }
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
      if let Some(sel) = &self.expense_grid_selection {
        if let Some(&row) = sel.rows.first() {
          self.expense_grid_edit_cell = Some((sel.column, row));
        }
      }
    }
    if self.expense_grid_edit_cell.is_none() && self.expense_grid_selection.is_some() {
      let is_ctrl_del = ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Delete) || input.consume_key(egui::Modifiers::CTRL, egui::Key::Delete));
      if is_ctrl_del {
        if let Some(sel) = &self.expense_grid_selection {
          let rows_to_delete = sel.rows.clone();
          self.delete_expenses_by_indices(&rows_to_delete);
        }
      } else {
        let is_del = ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
        if is_del {
          if let Some(sel) = &self.expense_grid_selection {
            self.show_delete_expense_confirm = true;
            self.delete_expense_indices = sel.rows.clone();
          }
        }
      }
    }
    expense_grid_selection_status(ui, &self.expense_grid_selection);

    ui.add_space(4.0);
    let mut pending_updates: Vec<(usize, &'static str)> = Vec::new();
    let mut pending_category_commits: Vec<usize> = Vec::new();
    let mut pending_member_commits: Vec<usize> = Vec::new();
    let category_candidates = self.cached_category_candidates.clone();
    let member_candidates = self.cached_member_candidates.clone();
    let vendor_candidates = self.cached_vendor_candidates.clone();
    let description_candidates = self.cached_description_candidates.clone();
    let mut autocomplete_selection = self.autocomplete_selection;

    let old_spacing = ui.spacing().item_spacing;
    ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
    let expense_columns = spreadsheet_columns(ui, 100.0, 90.0, 100.0, 200.0, 160.0);
    let expense_sort = self.expense_sort;
    let sorted_indices = self.cached_sorted_expense_indices.clone();
    let mut pending_grid_focus: Option<(GridColumn, usize)> = None;
    self.active_expense_cell = None;
    self.apply_pending_grid_keyboard(
      &sorted_indices,
      &mut pending_updates,
      &mut pending_category_commits,
      &mut pending_member_commits,
    );
    if let Some(target) = self.pending_grid_focus_target.take() {
      pending_grid_focus = Some(target);
    }
    let mut expense_scroll_y = self.expense_grid_scroll_offset;
    let grid_res = crate::ui::grid::render_shared_grid(
      ui,
      &mut self.expenses,
      &sorted_indices,
      &mut self.expense_grid_selection,
      &mut self.expense_grid_drag,
      &mut self.expense_grid_edit_cell,
      &mut self.expense_grid_edit_original,
      &mut self.expense_grid_typeahead,
      &mut expense_scroll_y,
      &mut autocomplete_selection,
      &vendor_candidates,
      &category_candidates,
      &member_candidates,
      &description_candidates,
      &self.members,
      &self.categories,
      expense_columns.to_vec(),
      "expense_cell",
      true,
    );

    self.expense_grid_scroll_offset = expense_scroll_y;
    self.active_expense_cell = grid_res.active_cell;

    if let Some((cat_id, name, color)) = grid_res.open_color_popup {
      self.category_color_popup = Some(CategoryColorPopup {
        id: cat_id,
        name,
        color,
      });
    }

    pending_updates.extend(grid_res.pending_field_updates);
    pending_category_commits.extend(grid_res.pending_category_commits);
    pending_member_commits.extend(grid_res.pending_member_commits);

    if let Some((column, idx)) = pending_grid_focus {
      self.expense_grid_selection = Some(GridSelection {
        column,
        rows: vec![idx],
      });
      self.expense_grid_edit_cell = Some((column, idx));
      request_expense_cell_focus(ui, column, idx);
    } else if let Some((column, idx)) = self.expense_grid_edit_cell {
      request_expense_cell_focus(ui, column, idx);
    }
    if self.expense_grid_drag.is_some() {
      ui.ctx().request_repaint();
    }
    ui.spacing_mut().item_spacing = old_spacing;
    let mut sort_changed = false;
    if self.expense_sort != expense_sort {
      self.expense_sort = expense_sort;
      sort_changed = true;
    }
    self.autocomplete_selection = autocomplete_selection;

    pending_updates.sort_unstable();
    pending_updates.dedup();
    pending_category_commits.sort_unstable();
    pending_category_commits.dedup();
    pending_member_commits.sort_unstable();
    pending_member_commits.dedup();

    self.flush_expense_grid_commits(
      &pending_updates,
      &pending_category_commits,
      &pending_member_commits,
    );

    let pending_edit = ui.ctx().data_mut(|d| {
      d.remove_temp::<Option<(GridColumn, usize)>>(egui::Id::new("pending_edit_cell"))
    });
    if let Some(Some(cell)) = pending_edit {
      self.expense_grid_edit_cell = Some(cell);
      ui.ctx().request_repaint();
    }
    if sort_changed || self.expense_grid_edit_cell != prev_edit_cell {
      self.rebuild_sorted_expense_indices();
    }
  }

  pub fn flush_expense_grid_commits(
    &mut self,
    pending_updates: &[(usize, &'static str)],
    pending_category_commits: &[usize],
    pending_member_commits: &[usize],
  ) {
    if pending_updates.is_empty() && pending_category_commits.is_empty() && pending_member_commits.is_empty() {
      return;
    }

    let categories = self.categories.clone();
    let parents = category_parent_map(&categories);
    let mut category_indices: Vec<usize> = pending_category_commits.to_vec();
    category_indices.sort_unstable();
    category_indices.dedup();
    let mut member_indices: Vec<usize> = pending_member_commits.to_vec();
    member_indices.sort_unstable();
    member_indices.dedup();

    if self.conn.execute("BEGIN IMMEDIATE", []).is_err() {
      self.log("[error] begin transaction failed");
      return;
    }

    let mut ok = true;
    let mut chart_dirty = false;

    for &(idx, field) in pending_updates {
      let Some(row) = self.expenses.get_mut(idx) else {
        continue;
      };
      if field == "member" {
        row.member = row.member.trim().to_string();
      }
      if field == "date" {
        row.date = format_date(&row.date).unwrap_or_else(|| row.date.clone());
      }
      if field == "amount" {
        if let Some(cents) = parse_amount_cents(&row.amount_input) {
          row.amount_cents = cents;
          row.amount_input = money(cents).replace('$', "");
        }
      }
      match update_expense_row(&self.conn, row, field) {
        Ok(()) => chart_dirty = true,
        Err(err) => {
          ok = false;
          self.log(format!("[error] save failed: {err}"));
        }
      }
    }

    for idx in category_indices {
      let Some(row) = self.expenses.get_mut(idx) else {
        continue;
      };
      row.category = row.category.trim().to_string();
      if row.category.is_empty() {
        match self.conn.execute(
          "UPDATE expenses SET category = '' WHERE id = ?1 AND household_id = ?2",
          params![row.id, self.household_id],
        ) {
          Ok(_) => chart_dirty = true,
          Err(err) => {
            ok = false;
            self.log(format!("[error] save failed: {err}"));
          }
        }
        continue;
      }
      let typed = row.category.clone();
      let Some(matched) = find_category_by_label(&categories, &typed) else {
        row.category.clear();
        self.log(format!(
          "[categories] '{typed}' is not in Settings — pick a category from the list"
        ));
        continue;
      };
      row.category = matched.full_label(&parents);
      let category = row.category.clone();
      let expense_id = row.id;
      match self.conn.execute(
        "UPDATE expenses SET category = ?1 WHERE id = ?2 AND household_id = ?3",
        params![category, expense_id, self.household_id],
      ) {
        Ok(_) => chart_dirty = true,
        Err(err) => {
          ok = false;
          self.log(format!("[error] save failed: {err}"));
        }
      }
    }

    for idx in member_indices {
      let Some(row) = self.expenses.get_mut(idx) else {
        continue;
      };
      row.member = row.member.trim().to_string();
      let member = row.member.clone();
      let expense_id = row.id;
      match self.conn.execute(
        "UPDATE expenses SET member = ?1 WHERE id = ?2 AND household_id = ?3",
        params![member, expense_id, self.household_id],
      ) {
        Ok(_) => chart_dirty = true,
        Err(err) => {
          ok = false;
          self.log(format!("[error] member save failed: {err}"));
        }
      }
    }

    if ok {
      if self.conn.execute("COMMIT", []).is_err() {
        let _ = self.conn.execute("ROLLBACK", []);
        self.log("[error] commit transaction failed");
      } else if chart_dirty {
        self.chart_dirty = true;
      }
    } else {
      let _ = self.conn.execute("ROLLBACK", []);
    }
  }
}

