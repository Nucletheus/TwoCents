use eframe::egui::{self, RichText};
use crate::models::*;
use crate::db::*;
use crate::ui::widgets::*;
use crate::TwoCentsApp;


impl TwoCentsApp {
  #[allow(deprecated)]
  pub fn ui_import_review(&mut self, ctx: &egui::Context) {
    if !self.show_import_review {
      return;
    }
    if let Some(sort) = self.pending_import_sort.take() {
      self.import_sort = sort;
      self.import_grid_state.clear_selection();
      self.rebuild_sorted_import_indices();
    }
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("is_rendering_import_grid"), true));

    let screen_rect = ctx.screen_rect();
    let pad_x = if screen_rect.width() < 900.0 { 16.0 } else { 60.0 };
    let pad_y = if screen_rect.height() < 600.0 { 16.0 } else { 60.0 };
    let win_w = (screen_rect.width() - pad_x * 2.0).clamp(320.0, 1200.0);
    let win_h = (screen_rect.height() - pad_y * 2.0).clamp(300.0, 800.0);

    // Use title_bar(false) to suppress the draggable title bar and × close button.
    // Those interactive elements competed for keyboard focus with the grid cells,
    // causing TextEdit focus to be stolen on every frame (manifesting as
    // "one letter at a time" erasure during typing). We render our own header row.
    egui::Window::new("Statement Import Review")
      .title_bar(false)
      .resizable(false)
      .collapsible(false)
      .fixed_size([win_w, win_h])
      .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
      .frame(themed_modal_frame(ctx))
      .show(ctx, |ui| {
        // Escape handling: only close the modal if nothing is being edited.
        // If a cell is being edited, let the grid handle Escape first (cancel edit).
        if self.import_grid_state.edit_cell.is_none() {
          if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if self.import_grid_state.selection.is_some() {
              self.import_grid_state.clear_selection();
            } else {
              self.show_import_review = false;
              return;
            }
          }
        }

        // ── Header row (our own title bar replacement) ───────────────────────
        ui.horizontal(|ui| {
          ui.label(RichText::new("Statement Import Review").color(ui.visuals().text_color()));
          let remaining_width = ui.available_width();
          ui.allocate_ui_with_layout(
            egui::vec2(remaining_width, ui.spacing().interact_size.y),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
              if ui.button("Cancel Review").clicked() {
                self.show_import_review = false;
              }
              if ui.add(egui::Button::new("Save Reviewed Import").fill(ui.visuals().selection.bg_fill)).clicked() {
                self.save_import();
              }
            },
          );
        });
        ui.add_space(4.0);
        ui.label(RichText::new("Edits here are staged. Click 'Save Reviewed Import' to commit them to your spreadsheet.").color(ui.visuals().text_color()));
        ui.add_space(8.0);

        ui.label(
          "Click or drag in a column to select rows. Hold Shift and drag to pan. Type to edit; Enter applies to all selected rows; Tab or Enter picks autocomplete; Escape clears (Escape again cancels review).",
        );

        let category_candidates = self.cached_category_candidates.clone();
        let member_candidates = self.cached_member_candidates.clone();
        let vendor_candidates = self.cached_import_vendor_candidates.clone();
        let description_candidates = self.cached_import_description_candidates.clone();
        let mut autocomplete_selection = self.autocomplete_selection;


        egui::Frame::new()
          .fill(ui.visuals().window_fill)
          .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
          .corner_radius(6.0)
          .inner_margin(egui::Margin::same(6))
          .show(ui, |ui| {
            let old_spacing = ui.spacing().item_spacing;
            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
            
            let grid_res = crate::ui::grid::render_grid(
              ui,
              &mut self.import_rows,
              &self.cached_sorted_import_indices,
              &mut self.import_grid_state,
              &mut autocomplete_selection,
              &vendor_candidates,
              &category_candidates,
              &member_candidates,
              &description_candidates,
              &self.members,
              &self.categories,
              "import_cell",
              false,
            );

            self.import_grid_state.active_cell = grid_res.active_cell;

            if let Some(col) = grid_res.clicked_sort_column {
              let ascending = if self.import_sort.column == col {
                !self.import_sort.ascending
              } else {
                true
              };
              self.pending_import_sort = Some(ExpenseSort { column: col, ascending });
              ui.ctx().request_repaint();
            }

            if let Some((cat_id, name, color)) = grid_res.open_color_popup {
              self.category_color_popup = Some(CategoryColorPopup {
                id: cat_id,
                name,
                color,
              });
            }

            if let Some(rows_to_delete) = grid_res.force_delete_rows {
              self.delete_import_rows_by_indices(&rows_to_delete);
            } else if let Some(rows_to_delete) = grid_res.delete_rows {
              self.show_delete_import_confirm = true;
              self.delete_import_indices = rows_to_delete;
            }

            let pending_updates = grid_res.pending_field_updates;
            let pending_category_commits = grid_res.pending_category_commits;
            let _pending_member_commits = grid_res.pending_member_commits;

            for &(idx, field) in &pending_updates {
              if field == "amount" {
                if let Some(row) = self.import_rows.get_mut(idx) {
                  if let Some(cents) = parse_amount_cents(&row.amount_input) {
                    row.amount_cents = cents;
                  }
                }
              }
            }
            if !pending_category_commits.is_empty() {
              self.categories = load_categories(&self.conn, self.household_id).unwrap_or_default();
            }

            if self.import_grid_state.drag.is_some() {
              ui.ctx().request_repaint();
            }
            ui.spacing_mut().item_spacing = old_spacing;
          });
        self.autocomplete_selection = autocomplete_selection;
      });

    ctx.data_mut(|d| {
      d.remove_temp::<bool>(egui::Id::new("is_rendering_import_grid"));
    });
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
        self.rebuild_sorted_import_indices();
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
    self.rebuild_sorted_import_indices();
    self.import_grid_state.clear_selection();
    self.log(format!("[import] removed {} row(s)", sorted_indices.len()));
  }



  fn sync_import_amounts(&mut self) {
    for row in &mut self.import_rows {
      if let Some(cents) = parse_amount_cents(&row.amount_input) {
        row.amount_cents = cents;
      }
    }
  }


}
