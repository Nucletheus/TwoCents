use crate::db::*;
use crate::models::*;
use crate::ui::popups::styled_button;
use crate::ui::widgets::*;
use crate::TwoCentsApp;
use eframe::egui;
use rusqlite::params;

impl TwoCentsApp {
    #[allow(deprecated)]
    pub fn ui_duplicate_review(&mut self, ctx: &egui::Context) {
        if !self.show_duplicate_review {
            return;
        }
        if let Some(sort) = self.pending_duplicate_sort.take() {
            self.duplicate_sort = sort;
            self.duplicate_grid_state.clear_selection();
            self.rebuild_sorted_duplicate_indices();
        }
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("is_rendering_duplicate_grid"), true));

        let screen_rect = ctx.content_rect();
        let pad_x = if screen_rect.width() < 900.0 {
            21.0
        } else {
            60.0
        };
        let pad_y = if screen_rect.height() < 600.0 {
            21.0
        } else {
            60.0
        };
        // fixed_size constrains the CONTENT; the modal frame wraps it in its own
        // inner margin + stroke. Subtract that chrome or the panel is wider than
        // the window (pad 42 < chrome 50 → the panel sat flush on both edges).
        let frame = themed_modal_frame(ctx);
        let chrome_x = (frame.inner_margin.left + frame.inner_margin.right) as f32
            + 2.0 * crate::ui::theme_tokens::BORDER_W;
        let chrome_y = (frame.inner_margin.top + frame.inner_margin.bottom) as f32
            + 2.0 * crate::ui::theme_tokens::BORDER_W;
        let win_w = (screen_rect.width() - pad_x * 2.0 - chrome_x).clamp(320.0, 1200.0);
        let win_h = (screen_rect.height() - pad_y * 2.0 - chrome_y).clamp(300.0, 800.0);

        egui::Window::new("Possible Duplicates Review")
      .order(egui::Order::Foreground)
      .title_bar(false)
      .resizable(false)
      .collapsible(false)
      .fixed_size([win_w, win_h])
      .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
      .frame(frame)
      .show(ctx, |ui| {
        // Escape handling: only close the modal if nothing is being edited.
        // If a cell is being edited, let the grid handle Escape first (cancel edit).
        if self.duplicate_grid_state.edit_cell.is_none() {
          if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if self.duplicate_grid_state.selection.is_some() {
              self.duplicate_grid_state.clear_selection();
            } else {
              self.show_duplicate_review = false;
              return;
            }
          }
        }

        // ── Header row ───────────────────────────────────────────────────────
        ui.horizontal(|ui| {
          // heading + secondary text, Notion hierarchy.
          ui.vertical(|ui| {
            crate::ui::components::heading_lg(ui, "Review Possible Duplicates");
            crate::ui::components::label_muted(
              ui,
              "Edits and deletions are staged in memory. Click 'Save and Resolve Duplicates' to write them to your database.",
            );
          });
          let remaining_width = ui.available_width();
          ui.allocate_ui_with_layout(
            egui::vec2(remaining_width, ui.spacing().interact_size.y),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
              if styled_button(ui, "Cancel", false).clicked() {
                self.show_duplicate_review = false;
              }
              if styled_button(ui, "Save and Resolve Duplicates", true).clicked() {
                self.save_duplicates();
              }
            },
          );
        });
        ui.add_space(4.0);
        crate::ui::components::label_muted(
          ui,
          "Use standard keys: Delete key to remove rows. Double click or press F2 to edit. Tab or Enter picks autocomplete.",
        );
        ui.add_space(8.0);

        let category_candidates = self.cached_category_candidates.clone();
        let member_candidates = self.cached_member_candidates.clone();
        let vendor_candidates = self.cached_vendor_candidates.clone();
        let description_candidates = self.cached_description_candidates.clone();
        let mut autocomplete_selection = self.autocomplete_selection;

        // shared grid_table_frame so the three grids have identical chrome.
        let frame_resp = crate::ui::components::grid_table_frame(ui)
          .show(ui, |ui| {
            let old_spacing = ui.spacing().item_spacing;
            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;

            let grid_res = crate::ui::grid::render_grid(
              ui,
              &mut self.duplicate_rows,
              &self.cached_sorted_duplicate_indices,
              &mut self.duplicate_grid_state,
              &mut autocomplete_selection,
              &vendor_candidates,
              &category_candidates,
              &member_candidates,
              &description_candidates,
              &self.members,
              &self.categories,
              "duplicate_cell",
              true,
            );

            self.duplicate_grid_state.active_cell = grid_res.active_cell;

            if let Some((cat_id, name, color)) = grid_res.open_color_popup {
              self.category_color_popup = Some(CategoryColorPopup {
                id: cat_id,
                name,
                color,
              });
            }

            if let Some(rows_to_delete) = grid_res.force_delete_rows {
              self.delete_duplicate_rows_by_indices(&rows_to_delete);
            } else if let Some(rows_to_delete) = grid_res.delete_rows {
              self.delete_duplicate_rows_by_indices(&rows_to_delete);
            }

            if self.duplicate_grid_state.drag.is_some() {
              ui.ctx().request_repaint();
            }

            ui.spacing_mut().item_spacing = old_spacing;

            if let Some(col) = grid_res.clicked_sort_column {
              let ascending = if self.duplicate_sort.column == col {
                !self.duplicate_sort.ascending
              } else {
                true
              };
              self.pending_duplicate_sort = Some(ExpenseSort { column: col, ascending });
              ui.ctx().request_repaint();
            }
            self.autocomplete_selection = autocomplete_selection;

            // Sync field updates back into staged duplicate_rows
            let mut pending_updates = grid_res.pending_field_updates;
            let mut pending_category_commits = grid_res.pending_category_commits;
            let mut pending_member_commits = grid_res.pending_member_commits;

            pending_updates.sort_unstable();
            pending_updates.dedup();
            pending_category_commits.sort_unstable();
            pending_category_commits.dedup();
            pending_member_commits.sort_unstable();
            pending_member_commits.dedup();

            let categories = self.categories.clone();
            let parents = category_parent_map(&categories);

            for &(idx, field) in &pending_updates {
              if let Some(row) = self.duplicate_rows.get_mut(idx) {
                if field == "member" {
                  row.member = row.member.trim().to_string();
                }
                if field == "date" {
                  row.date = format_date(&row.date).unwrap_or_else(|| row.date.clone());
                }
                if field == "amount" {
                  if let Some(magnitude) = parse_amount_cents(&row.amount_input) {
                    // sign is a function of category; excluded rows
                    // keep their real amount.
                    let sign = category_sign(&categories, &row.category);
                    row.amount_cents = if sign == 1 { magnitude } else { -magnitude };
                    row.amount_input = money(row.amount_cents).replace('$', "");
                  }
                }
              }
            }

            for idx in pending_category_commits {
              if let Some(row) = self.duplicate_rows.get_mut(idx) {
                row.category = row.category.trim().to_string();
                if !row.category.is_empty() {
                  let typed = row.category.clone();
                  if let Some(matched) = find_category_by_label(&categories, &typed) {
                    row.category = matched.full_label(&parents);
                    // sign is a function of category — re-derive
                    // on recategorize (same as grid/review commits), else
                    // resolving duplicates would save a stale sign. Excluded
                    // rows keep their real amount.
                    if let Some(magnitude) = parse_amount_cents(&row.amount_input) {
                      let sign = category_sign(&categories, &row.category);
                      row.amount_cents = if sign == 1 { magnitude } else { -magnitude };
                      row.amount_input = money(row.amount_cents).replace('$', "");
                    }
                  } else {
                    row.category.clear();
                  }
                }
              }
            }

            for idx in pending_member_commits {
              if let Some(row) = self.duplicate_rows.get_mut(idx) {
                row.member = row.member.trim().to_string();
              }
            }
          });
        // Frame stroke on top of the scrolled content (clip_rect_margin bleed).
        crate::ui::components::repaint_grid_frame_stroke(ui, frame_resp.response.rect);
      });
    }

    fn save_duplicates(&mut self) {
        if self.conn.execute("BEGIN IMMEDIATE", []).is_err() {
            return;
        }

        let mut ok = true;

        // 1. Delete all staged-for-deletion expenses
        for &id in &self.duplicate_deleted_ids {
            if let Err(err) = self.conn.execute(
                "DELETE FROM expenses WHERE id = ?1 AND household_id = ?2",
                params![id, self.household_id],
            ) {
                ok = false;
                break;
            }
        }

        // 2. Update remaining duplicates in the database
        if ok {
            let categories = self.categories.clone();
            let parents = category_parent_map(&categories);
            for row in &mut self.duplicate_rows {
                // Double check category label normalization
                let category = if row.category.trim().is_empty() {
                    String::new()
                } else if let Some(matched) = find_category_by_label(&categories, &row.category) {
                    matched.full_label(&parents)
                } else {
                    String::new()
                };

                if let Err(err) = self.conn.execute(
          "UPDATE expenses
           SET date = ?1, amount_cents = ?2, category = ?3, member = ?4, vendor = ?5, description = ?6
           WHERE id = ?7 AND household_id = ?8",
          params![
            row.date,
            row.amount_cents,
            category,
            row.member,
            row.vendor,
            row.description,
            row.id,
            self.household_id
          ]
        ) {
          ok = false;
                    break;
        }
            }
        }

        if ok {
            if self.conn.execute("COMMIT", []).is_err() {
                let _ = self.conn.execute("ROLLBACK", []);
            } else {
                self.duplicate_rows.clear();
                self.duplicate_deleted_ids.clear();
                self.show_duplicate_review = false;
                self.reload();
            }
        } else {
            let _ = self.conn.execute("ROLLBACK", []);
        }
    }

    pub fn delete_duplicate_rows_by_indices(&mut self, indices: &[usize]) {
        let mut sorted_indices = indices.to_vec();
        sorted_indices.sort_by(|a, b| b.cmp(a));
        for &idx in &sorted_indices {
            if idx < self.duplicate_rows.len() {
                let expense = self.duplicate_rows.remove(idx);
                self.duplicate_deleted_ids.insert(expense.id);
            }
        }
        self.rebuild_sorted_duplicate_indices();
        self.duplicate_grid_state.clear_selection();
    }
}
