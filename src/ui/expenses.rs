use eframe::egui;
use rusqlite::params;
use crate::models::*;

use crate::db::*;
use crate::ui::popups::styled_button;
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
      crate::ui::components::heading_lg(ui, "Household Expense Spreadsheet");
    });
    ui.horizontal_wrapped(|ui| {
      ui.set_max_width(toolbar_width);
      // themed buttons instead of the default egui look. Subtle
      // outlined buttons read cleaner in a toolbar than filled defaults.
      if self.csv_import_rx.is_some() {
        let _ = styled_button(ui, "Importing...", false).on_hover_text("Waiting for file selection");
      } else if styled_button(ui, "Import CSV Statement", true).clicked() {
        self.import_csv();
      }
      if styled_button(ui, "Duplicates", false).clicked() {
        self.find_duplicates();
      }
    });
    crate::ui::components::label_muted(ui,
      "Click or drag in a column to select rows. Hold Shift and drag to pan. Type to edit; Enter applies to all selected rows; Tab or Enter picks autocomplete and moves right; Escape clears.");
    expense_grid_selection_status(ui, &self.expense_grid_state.selection);

    ui.add_space(4.0);
    // real empty state when the household has no expenses. The
    // table-render path stays the same for non-empty cases.
    if self.expenses.is_empty() {
      crate::ui::components::empty_state(
        ui,
        "No expenses yet",
        "Click Import CSV Statement to load a bank statement, or add an expense manually.",
      );
      return;
    }
    // clones guarded on editing — autocomplete only reads these
    // while a cell is open; the first frame of a new edit is the frame after
    // edit_cell is set, so the candidates are present when needed.
    let editing = self.expense_grid_state.edit_cell.is_some();
    let category_candidates = if editing { self.cached_category_candidates.clone() } else { Vec::new() };
    let member_candidates = if editing { self.cached_member_candidates.clone() } else { Vec::new() };
    let vendor_candidates = if editing { self.cached_vendor_candidates.clone() } else { Vec::new() };
    let description_candidates = if editing { self.cached_description_candidates.clone() } else { Vec::new() };
    let mut autocomplete_selection = self.autocomplete_selection;

    let old_spacing = ui.spacing().item_spacing;
    ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;

    // mem::take instead of a full O(N) Vec copy per frame —
    // nothing reads the cache while the grid render borrows it.
    let sorted_indices = std::mem::take(&mut self.cached_sorted_expense_indices);

    // shared grid_table_frame so the expense, import, and duplicates
    // grids have identical chrome (1px border, 6px radius, 2px inner margin).
    let grid_res = crate::ui::components::grid_table_frame(ui)
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
      });
    // Frame stroke on top of the scrolled content (clip_rect_margin bleed).
    crate::ui::components::repaint_grid_frame_stroke(ui, grid_res.response.rect);
    let grid_res = grid_res.inner;
    self.cached_sorted_expense_indices = sorted_indices;

    self.expense_grid_state.active_cell = grid_res.active_cell;

    if let Some((cat_id, name, color)) = grid_res.open_color_popup {
      self.category_color_popup = Some(CategoryColorPopup {
        id: cat_id,
        name,
        color,
      });
    }

    if let Some(rows_to_delete) = grid_res.force_delete_rows {
      // flush first — deferred edits reference indices that shift
      // once rows are removed.
      self.flush_deferred_expense_commits();
      self.delete_expenses_by_indices(&rows_to_delete);
    } else if let Some(rows_to_delete) = grid_res.delete_rows {
      self.flush_deferred_expense_commits();
      self.show_delete_expense_confirm = true;
      self.delete_expense_indices = rows_to_delete;
    }

    let mut pending_updates = grid_res.pending_field_updates;
    let mut pending_category_commits = grid_res.pending_category_commits;
    let mut pending_member_commits = grid_res.pending_member_commits;

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

    // DB commits used to run per keystroke — a synchronous
    // BEGIN/UPDATE/COMMIT every frame while typing. Accumulate this frame's
    // changes and flush 400ms after the last change, or immediately when the
    // editing cell closes. In-memory rows are already correct; the DB just
    // catches up.
    self.deferred_field_updates.append(&mut pending_updates);
    self.deferred_category_commits.append(&mut pending_category_commits);
    self.deferred_member_commits.append(&mut pending_member_commits);
    let pending_total = self.deferred_field_updates.len()
      + self.deferred_category_commits.len()
      + self.deferred_member_commits.len();
    if pending_total > 0 {
      if pending_total != self.expense_last_pending_len {
        self.expense_last_pending_len = pending_total;
        self.expense_flush_at = Some(std::time::Instant::now());
      }
      let idle = self.expense_flush_at
        .map_or(true, |t| t.elapsed() >= std::time::Duration::from_millis(400));
      if idle || self.expense_grid_state.edit_cell.is_none() {
        self.flush_deferred_expense_commits();
      }
    } else {
      self.expense_last_pending_len = 0;
      self.expense_flush_at = None;
    }

  }

  pub fn flush_deferred_expense_commits(&mut self) {
    if self.deferred_field_updates.is_empty()
      && self.deferred_category_commits.is_empty()
      && self.deferred_member_commits.is_empty()
    {
      return;
    }
    let field_updates = std::mem::take(&mut self.deferred_field_updates);
    let category_commits = std::mem::take(&mut self.deferred_category_commits);
    let member_commits = std::mem::take(&mut self.deferred_member_commits);
    self.flush_expense_grid_commits(&field_updates, &category_commits, &member_commits);
    self.expense_last_pending_len = 0;
    self.expense_flush_at = None;
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
            return;
    }

    let mut ok = true;

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
        // sign is a function of category — debit negative,
        // Income positive. Excluded rows keep their real amount.
        if let Some(magnitude) = parse_amount_cents(&row.amount_input) {
          let sign = category_sign(&categories, &row.category);
          row.amount_cents = if sign == 1 { magnitude } else { -magnitude };
          row.amount_input = money(row.amount_cents).replace('$', "");
        }
      }
      if field == "account" {
        // Resolve the typed name against existing accounts; unknown names
        // revert to the row's current account.
        if let Some(matched) = self.accounts.iter().find(|account| account.name.eq_ignore_ascii_case(row.account.trim())) {
          row.account_id = matched.id;
          row.account = matched.name.clone();
        } else if let Some(current) = self.accounts.iter().find(|account| account.id == row.account_id) {
          row.account = current.name.clone();
        }
      }
      match update_expense_row(&self.conn, row, field) {
        Ok(()) => {}
        Err(err) => {
          ok = false;
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
          Ok(_) => {}
          Err(err) => {
            ok = false;
                      }
        }
        continue;
      }
      let typed = row.category.clone();
      let Some(matched) = find_category_by_label(&categories, &typed) else {
        row.category.clear();
                continue;
      };
      row.category = matched.full_label(&parents);
      let category = row.category.clone();
      let expense_id = row.id;
      // category change re-derives the amount's sign —
      // recategorizing to/from Income flips credit/debit. Excluded rows
      // keep their real amount (flag filters aggregation only).
      let sign = category_sign(&categories, &category);
      let new_cents = if sign == 1 { row.amount_cents.abs() } else { -row.amount_cents.abs() };
      if new_cents != row.amount_cents {
        row.amount_cents = new_cents;
        row.amount_input = money(new_cents).replace('$', "");
        let _ = self.conn.execute(
          "UPDATE expenses SET amount_cents = ?1 WHERE id = ?2 AND household_id = ?3",
          params![new_cents, expense_id, self.household_id],
        );
      }
      match self.conn.execute(
        "UPDATE expenses SET category = ?1 WHERE id = ?2 AND household_id = ?3",
        params![category, expense_id, self.household_id],
      ) {
        Ok(_) => {
          // spreadsheet corrections teach the categorizer — upsert
          // the vendor rule so future imports use the fixed category. Skip
          // short vendors (< 3 chars) — they substring-match everything.
          if !row.vendor.is_empty() && row.vendor.trim().chars().count() >= 3 {
            let vendor = row.vendor.to_lowercase();
            let _ = self.conn.execute(
              "INSERT INTO vendor_category_rules (household_id, vendor_pattern, category) VALUES (?1, ?2, ?3)
               ON CONFLICT(household_id, vendor_pattern) DO UPDATE SET category = excluded.category",
              rusqlite::params![self.household_id, vendor, category],
            );
          }
        }
        Err(err) => {
          ok = false;
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
        Ok(_) => {}
        Err(err) => {
          ok = false;
                  }
      }
    }

    if ok {
      if self.conn.execute("COMMIT", []).is_err() {
        let _ = self.conn.execute("ROLLBACK", []);
              }
    } else {
      let _ = self.conn.execute("ROLLBACK", []);
    }
  }

  pub fn find_duplicates(&mut self) {
    let results = {
      let mut stmt = match self.conn.prepare(
        "SELECT e1.id, e1.date, e1.amount_cents, COALESCE(e1.member, ''), e1.category, COALESCE(e1.vendor, ''), e1.description, COALESCE(e1.account_id, 1), COALESCE(a.name, '')
         FROM expenses e1
         LEFT JOIN accounts a ON a.id = e1.account_id
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
          account_id: row.get(7)?,
          account: row.get(8)?,
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
              }
      Err(err) => {
              }
    }
  }
}

