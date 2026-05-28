use eframe::egui::{self, Id, RichText};
use crate::models::*;
use crate::db::*;
use crate::ui::widgets::*;
use crate::TwoCentsApp;


impl TwoCentsApp {
  pub fn ui_import_modal(&mut self, ctx: &egui::Context) {
    if !self.show_import_review {
      return;
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("is_rendering_import_grid"), true));
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
      if self.import_grid_selection.is_some() {
        self.clear_import_grid_selection();
      } else {
        self.show_import_review = false;
        return;
      }
    }

    let mut open = self.show_import_review;
    egui::Window::new("Review Statement Import")
      .open(&mut open)
      .resizable(true)
      .default_size([1060.0, 620.0])
      .frame(
        egui::Frame::window(&ctx.global_style())
          .fill(ctx.global_style().visuals.panel_fill)
          .stroke(egui::Stroke::new(2.0, ctx.global_style().visuals.selection.bg_fill))
          .corner_radius(10.0)
          .inner_margin(egui::Margin::symmetric(14, 12)),
      )
      .show(ctx, |ui| {
        egui::Frame::new()
          .fill(ui.visuals().window_fill)
          .stroke(egui::Stroke::new(1.0, ui.visuals().selection.bg_fill))
          .corner_radius(6.0)
          .inner_margin(egui::Margin::symmetric(10, 6))
          .show(ui, |ui| {
            ui.horizontal(|ui| {
              ui.label(RichText::new("Import Review").strong().color(ui.visuals().strong_text_color()));
              ui.separator();
              ui.label(RichText::new("Edits here are staged until Save Reviewed Import.").color(ui.visuals().text_color()));
              ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel Review").clicked() {
                  self.show_import_review = false;
                }
                if ui.add(egui::Button::new("Submit Import").fill(ui.visuals().selection.bg_fill)).clicked() {
                  self.save_import();
                }
              });
            });
          });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
          ui.label("Bulk category:");
          grid_text_edit_cell(ui, &mut self.bulk_category, Id::new("bulk_category"), true);
          if ui.button("Apply").clicked() {
            self.apply_bulk_category();
          }
          let mut clicked_label = None;
          for label in &self.cached_category_candidates {
            if ui.small_button(label).clicked() {
              clicked_label = Some(label.clone());
            }
          }
          if let Some(label) = clicked_label {
            self.bulk_category = label;
            self.apply_bulk_category();
          }
        });
        ui.label(
          "Click or drag in a column to select rows. Hold Shift and drag to pan. Type to edit; Enter applies to all selected rows; Tab or Enter picks autocomplete; Escape clears (Escape again closes review).",
        );
        if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
          if let Some(sel) = &self.import_grid_selection {
            if let Some(&row) = sel.rows.first() {
              self.import_grid_edit_cell = Some((sel.column, row));
            }
          }
        }
        if self.import_grid_edit_cell.is_none() && self.import_grid_selection.is_some() {
          let is_ctrl_del = ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Delete) || input.consume_key(egui::Modifiers::CTRL, egui::Key::Delete));
          if is_ctrl_del {
            if let Some(sel) = &self.import_grid_selection {
              let rows_to_delete = sel.rows.clone();
              self.delete_import_rows_by_indices(&rows_to_delete);
            }
          } else {
            let is_del = ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
            if is_del {
              if let Some(sel) = &self.import_grid_selection {
                self.show_delete_import_confirm = true;
                self.delete_import_indices = sel.rows.clone();
              }
            }
          }
        }

        let mut pending_amount_updates = Vec::new();
        let mut pending_import_member_commits = Vec::new();
        let category_candidates = self.cached_category_candidates.clone();
        let member_candidates = self.cached_member_candidates.clone();
        let vendor_candidates = self.cached_import_vendor_candidates.clone();
        let description_candidates = self.cached_import_description_candidates.clone();
        let mut autocomplete_selection = self.autocomplete_selection;

        let mut pending_updates = Vec::new();
        let mut pending_category_commits = Vec::new();
        let mut pending_member_commits = Vec::new();
        let mut pending_grid_focus = None;

        let import_indices: Vec<usize> = (0..self.import_rows.len()).collect();

        self.active_import_cell = None;
        self.apply_pending_import_grid_keyboard(
          &import_indices,
          &mut pending_updates,
          &mut pending_category_commits,
          &mut pending_member_commits,
        );
        if let Some(target) = self.pending_import_grid_focus_target.take() {
          pending_grid_focus = Some(target);
        }

        egui::Frame::new()
          .fill(ui.visuals().window_fill)
          .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
          .corner_radius(6.0)
          .inner_margin(egui::Margin::same(6))
          .show(ui, |ui| {
            let old_spacing = ui.spacing().item_spacing;
            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
            let import_columns = spreadsheet_columns(ui, 95.0, 85.0, 90.0, 180.0, 140.0);
            let mut import_scroll_y = self.import_grid_scroll_offset;
            
            let grid_res = crate::ui::grid::render_shared_grid(
              ui,
              &mut self.import_rows,
              &import_indices,
              &mut self.import_grid_selection,
              &mut self.import_grid_drag,
              &mut self.import_grid_edit_cell,
              &mut self.import_grid_edit_original,
              &mut self.import_grid_typeahead,
              &mut import_scroll_y,
              &mut autocomplete_selection,
              &vendor_candidates,
              &category_candidates,
              &member_candidates,
              &description_candidates,
              &self.members,
              &self.categories,
              import_columns.to_vec(),
              "import_cell",
              false,
            );

            self.import_grid_scroll_offset = import_scroll_y;
            self.active_import_cell = grid_res.active_cell;

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

            // Propagate any in-memory field commits made in the shared grid.
            // Since commit target replication happens inside render_shared_grid, we just need to ensure
            // the DB/staged changes are synced. In the import review modal, changes are kept in memory
            // but bulk field syncs (`apply_import_field_value`, etc.) are triggered upon grid commit events.
            for (idx, field) in pending_updates {
              let col = match field {
                "date" => GridColumn::Date,
                "amount" => GridColumn::Amount,
                "vendor" => GridColumn::Vendor,
                "description" => GridColumn::Description,
                _ => GridColumn::Vendor,
              };
              let targets = self.import_commit_targets(col, idx);
              self.apply_import_field_value(&targets, field, idx);
              if field == "amount" {
                pending_amount_updates.extend(targets);
              }
            }

            for idx in pending_category_commits {
              let targets = self.import_commit_targets(GridColumn::Category, idx);
              let val = self.import_rows.get(idx).map(|r| r.category.clone()).unwrap_or_default();
              self.apply_import_category_value(&targets, &val);
              self.register_categories_for_import_rows(&targets);
            }

            for idx in pending_member_commits {
              let targets = self.import_commit_targets(GridColumn::Member, idx);
              let val = self.import_rows.get(idx).map(|r| r.member.clone()).unwrap_or_default();
              self.apply_import_member_value(&targets, &val);
              pending_import_member_commits.extend(targets);
            }

            if let Some((column, idx)) = pending_grid_focus {
              self.import_grid_selection = Some(GridSelection {
                column,
                rows: vec![idx],
              });
              self.import_grid_edit_cell = Some((column, idx));
              request_import_cell_focus(ui, column, idx);
            } else if let Some((column, idx)) = self.import_grid_edit_cell {
              request_import_cell_focus(ui, column, idx);
            }
            ui.spacing_mut().item_spacing = old_spacing;
          });
        self.autocomplete_selection = autocomplete_selection;
        pending_amount_updates.sort_unstable();
        pending_amount_updates.dedup();
        pending_import_member_commits.sort_unstable();
        pending_import_member_commits.dedup();
        let _ = pending_import_member_commits;

        for idx in pending_amount_updates {
          if let Some(row) = self.import_rows.get_mut(idx) {
            if let Some(cents) = parse_amount_cents(&row.amount_input) {
              row.amount_cents = cents;
            }
          }
        }

        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
          if ui.button("Cancel").clicked() {
            self.show_import_review = false;
          }
          if ui.add(egui::Button::new("Save Reviewed Import").fill(ui.visuals().selection.bg_fill)).clicked() {
            self.save_import();
          }
        });
      });
     self.show_import_review = open && self.show_import_review;
 
     let pending_import_edit = ctx.data_mut(|d| {
       d.remove_temp::<bool>(egui::Id::new("is_rendering_import_grid"));
       d.remove_temp::<Option<(GridColumn, usize)>>(egui::Id::new("pending_import_edit_cell"))
     });
     if let Some(Some(cell)) = pending_import_edit {
       self.import_grid_edit_cell = Some(cell);
       ctx.request_repaint();
     }
  }



  fn rebuild_import_cached_candidates(&mut self) {
    self.cached_import_vendor_candidates = unique_nonempty_values(self.import_rows.iter().map(|row| row.vendor.as_str()));
    self.cached_import_description_candidates = unique_nonempty_values(self.import_rows.iter().map(|row| row.description.as_str()));
  }

  pub fn import_csv(&mut self) {
    if self.csv_import_rx.is_some() {
      return;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    self.csv_import_rx = Some(rx);
    std::thread::spawn(move || {
      let path = rfd::FileDialog::new()
        .set_title("Import statement CSV")
        .add_filter("CSV statements", &["csv"])
        .pick_file();
      let _ = tx.send(path);
    });
  }

  pub fn process_csv_file(&mut self, path: &std::path::Path) {
    match parse_csv_statement(&self.conn, self.household_id, path) {
      Ok(rows) if rows.is_empty() => self.log("[import] cancelled or no valid rows"),
      Ok(rows) => {
        let duplicates = duplicate_import_count(&self.conn, self.household_id, &rows);
        let ready = rows.iter().filter(|row| import_row_status(row) == "ready").count();
        self.import_rows = rows;
        self.rebuild_import_cached_candidates();
        self.show_import_review = true;
        self.log(format!(
          "[import] loaded {} rows ready={} possible_duplicates={}",
          self.import_rows.len(),
          ready,
          duplicates
        ));
      }
      Err(err) => self.log(format!("[error] import failed: {err}")),
    }
  }

  fn save_import(&mut self) {
    if self.import_rows.is_empty() {
      self.log("[import] nothing to save");
      return;
    }

    self.sync_import_amounts();
    let rows = self.import_rows.clone();
    match save_import_rows(&self.conn, self.household_id, &rows) {
      Ok(count) => {
        self.import_rows.clear();
        self.show_import_review = false;
        self.reload();
        self.log(format!("[import] saved {count} rows"));
      }
      Err(err) => self.log(format!("[error] import save failed: {err}")),
    }
  }

  pub fn delete_import_rows_by_indices(&mut self, indices: &[usize]) {
    let mut sorted_indices = indices.to_vec();
    sorted_indices.sort_by(|a, b| b.cmp(a));
    for &idx in &sorted_indices {
      if idx < self.import_rows.len() {
        self.import_rows.remove(idx);
      }
    }
    self.rebuild_import_cached_candidates();
    self.import_grid_selection = None;
    self.log(format!("[import] removed {} row(s)", sorted_indices.len()));
  }

  fn apply_bulk_category(&mut self) {
    let category = self.bulk_category.trim().to_string();
    if category.is_empty() {
      self.log("[bulk] no category entered");
      return;
    }
    for row in &mut self.import_rows {
      row.category = category.clone();
    }
    if find_category_by_label(&self.categories, &category).is_some() {
      self.reload();
    } else {
      self.log(format!("[bulk] unknown category '{category}' — add it in Settings first"));
    }
    self.log(format!(
      "[bulk] applied category '{}' to {} import rows",
      category,
      self.import_rows.len()
    ));
  }

  fn sync_import_amounts(&mut self) {
    for row in &mut self.import_rows {
      if let Some(cents) = parse_amount_cents(&row.amount_input) {
        row.amount_cents = cents;
      }
    }
  }

  fn clear_import_grid_selection(&mut self) {
    self.import_grid_selection = None;
    self.import_grid_drag = None;
    self.import_grid_edit_cell = None;
    self.import_grid_typeahead = None;
    self.active_import_cell = None;
  }



  pub(crate) fn import_commit_targets(&self, column: GridColumn, active_row: usize) -> Vec<usize> {
    grid_commit_targets(&self.import_grid_selection, column, active_row)
  }

  pub(crate) fn apply_import_member_value(&mut self, indices: &[usize], value: &str) {
    let value = value.trim().to_string();
    for &idx in indices {
      if let Some(row) = self.import_rows.get_mut(idx) {
        row.member = value.clone();
      }
    }
  }

  pub(crate) fn apply_import_category_value(&mut self, indices: &[usize], value: &str) {
    let value = value.trim().to_string();
    for &idx in indices {
      if let Some(row) = self.import_rows.get_mut(idx) {
        row.category = value.clone();
      }
    }
  }

  fn register_categories_for_import_rows(&mut self, indices: &[usize]) {
    let mut changed = false;
    for &idx in indices {
      let Some(row) = self.import_rows.get(idx) else {
        continue;
      };
      let category = row.category.trim();
      if category.is_empty() {
        continue;
      }
      if find_category_by_label(&self.categories, category).is_some() {
        changed = true;
      }
    }
    if changed {
      self.categories = load_categories(&self.conn, self.household_id).unwrap_or_default();
    }
  }

  pub(crate) fn apply_import_field_value(&mut self, indices: &[usize], field: &str, source_idx: usize) {
    let Some(source) = self.import_rows.get(source_idx).cloned() else {
      return;
    };
    for &idx in indices {
      let Some(row) = self.import_rows.get_mut(idx) else {
        continue;
      };
      match field {
        "date" => row.date = source.date.clone(),
        "amount" => {
          row.amount_input = source.amount_input.clone();
          row.amount_cents = source.amount_cents;
        }
        "vendor" => row.vendor = source.vendor.clone(),
        "description" => row.description = source.description.clone(),
        _ => {}
      }
    }
  }
}

fn import_cell_id(column: GridColumn, row: usize) -> Id {
  Id::new(("import_cell", format!("{column:?}"), row))
}

fn request_import_cell_focus(ui: &mut egui::Ui, column: GridColumn, row: usize) {
  request_grid_cell_focus(ui, import_cell_id(column, row));
}
