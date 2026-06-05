use eframe::egui::{self, RichText};
use rusqlite::params;
use crate::models::*;

use crate::db::*;
use crate::ui::widgets::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_expenses(&mut self, ui: &mut egui::Ui) {
    if let Some(sort) = self.pending_expense_sort.take() {
      self.expense_sort = sort;
      self.expense_grid_state.clear_selection();
      self.rebuild_sorted_expense_indices();
    }
    let toolbar_width = ui.available_width();
    ui.horizontal_wrapped(|ui| {
      ui.set_max_width(toolbar_width);
      ui.heading(RichText::new("Household Expense Spreadsheet").color(ui.visuals().text_color()));
    });
    ui.horizontal_wrapped(|ui| {
      ui.set_max_width(toolbar_width);
      if self.csv_import_rx.is_some() {
        let _ = ui.button("Importing...").on_hover_text("Waiting for file selection");
      } else if ui.button("Import CSV Statement").clicked() {
        self.import_csv();
      }
      if ui.button("Duplicates").clicked() {
        self.find_duplicates();
      }
      if ui.button("Settings").clicked() {
        self.show_category_settings = true;
      }
    });
    ui.label(
      "Click or drag in a column to select rows. Hold Shift and drag to pan. Type to edit; Enter applies to all selected rows; Tab or Enter picks autocomplete and moves right; Escape clears.",
    );
    expense_grid_selection_status(ui, &self.expense_grid_state.selection);

    ui.add_space(4.0);
    let category_candidates = self.cached_category_candidates.clone();
    let member_candidates = self.cached_member_candidates.clone();
    let vendor_candidates = self.cached_vendor_candidates.clone();
    let description_candidates = self.cached_description_candidates.clone();
    let mut autocomplete_selection = self.autocomplete_selection;

    let old_spacing = ui.spacing().item_spacing;
    ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;

    let sorted_indices = self.cached_sorted_expense_indices.clone();

    let table_bg = ui.visuals().extreme_bg_color;
    let table_stroke = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let grid_res = egui::Frame::default()
      .fill(table_bg)
      .stroke(egui::Stroke::new(1.0, table_stroke))
      .corner_radius(8.0)
      .inner_margin(egui::Margin::same(2))
      .show(ui, |ui| {
        crate::ui::grid::render_grid(
          ui,
          &mut self.expenses,
          &sorted_indices,
          &mut self.expense_grid_state,
          &mut autocomplete_selection,
          &vendor_candidates,
          &category_candidates,
          &member_candidates,
          &description_candidates,
          &self.members,
          &self.categories,
          "expense_cell",
          true,
        )
      }).inner;

    self.expense_grid_state.active_cell = grid_res.active_cell;

    if let Some((cat_id, name, color)) = grid_res.open_color_popup {
      self.category_color_popup = Some(CategoryColorPopup {
        id: cat_id,
        name,
        color,
      });
    }

    if let Some(rows_to_delete) = grid_res.force_delete_rows {
      self.delete_expenses_by_indices(&rows_to_delete);
    } else if let Some(rows_to_delete) = grid_res.delete_rows {
      self.show_delete_expense_confirm = true;
      self.delete_expense_indices = rows_to_delete;
    }

    let mut pending_updates = grid_res.pending_field_updates;
    let mut pending_category_commits = grid_res.pending_category_commits;
    let mut pending_member_commits = grid_res.pending_member_commits;

    if self.expense_grid_state.drag.is_some() {
      ui.ctx().request_repaint();
    }
    ui.spacing_mut().item_spacing = old_spacing;
    if let Some(col) = grid_res.clicked_sort_column {
      let ascending = if self.expense_sort.column == col {
        !self.expense_sort.ascending
      } else {
        true
      };
      self.pending_expense_sort = Some(ExpenseSort { column: col, ascending });
      ui.ctx().request_repaint();
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

  pub fn find_duplicates(&mut self) {
    let results = {
      let mut stmt = match self.conn.prepare(
        "SELECT e1.id, e1.date, e1.amount_cents, COALESCE(e1.member, ''), e1.category, COALESCE(e1.vendor, ''), e1.description
         FROM expenses e1
         INNER JOIN (
             SELECT date, amount_cents, lower(COALESCE(vendor, '')) as l_vendor
             FROM expenses
             WHERE household_id = ?1
             GROUP BY date, amount_cents, lower(COALESCE(vendor, ''))
             HAVING COUNT(*) > 1
         ) e2 ON e1.date = e2.date 
             AND e1.amount_cents = e2.amount_cents 
             AND lower(COALESCE(e1.vendor, '')) = e2.l_vendor
         WHERE e1.household_id = ?1
         ORDER BY e1.date DESC, e1.amount_cents DESC, lower(COALESCE(e1.vendor, ''))"
      ) {
        Ok(stmt) => stmt,
        Err(err) => {
          eprintln!("[duplicates] Prepare query failed: {err}");
          return;
        }
      };

      let rows_res = stmt.query_map(params![self.household_id], |row| {
        let amount_cents: i64 = row.get(2)?;
        let amount_input = money(amount_cents).replace('$', "");
        Ok(Expense {
          id: row.get(0)?,
          date: row.get(1)?,
          amount_input,
          amount_cents,
          member: row.get(3)?,
          category: row.get(4)?,
          vendor: row.get(5)?,
          description: row.get(6)?,
        })
      });

      match rows_res {
        Ok(mapped_rows) => {
          let mut results = Vec::new();
          for r in mapped_rows {
            if let Ok(exp) = r {
              results.push(exp);
            }
          }
          Ok(results)
        }
        Err(err) => Err(err),
      }
    }; // stmt is dropped here!

    match results {
      Ok(res) => {
        self.duplicate_rows = res;
        self.duplicate_deleted_ids.clear();
        self.duplicate_grid_state.clear_selection();
        self.show_duplicate_review = true;
        self.rebuild_sorted_duplicate_indices();
        self.log(format!("[duplicates] found {} possible duplicates for review", self.duplicate_rows.len()));
      }
      Err(err) => {
        self.log(format!("[duplicates] Query execution failed: {err}"));
      }
    }
  }
}

