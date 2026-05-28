use eframe::egui::{self, Color32, Id, RichText};
use egui_extras::TableBuilder;
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
        let review_table_height = (ui.available_height() - 96.0).max(180.0);
        let mut autocomplete_selection = self.autocomplete_selection;
        egui::Frame::new()
          .fill(ui.visuals().window_fill)
          .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
          .corner_radius(6.0)
          .inner_margin(egui::Margin::same(6))
          .show(ui, |ui| {
            let old_spacing = ui.spacing().item_spacing;
            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
            let import_columns = spreadsheet_columns(ui, 95.0, 85.0, 90.0, 180.0, 140.0);
            let category_menu_w = category_picker_menu_width(ui, &self.categories, 180.0);
            let member_menu_w = MEMBER_PICKER_MIN_WIDTH;
            let member_count = self.members.len();
            let member_scroll = if member_count > 10 {
              Some(member_picker_max_height(member_count))
            } else {
              None
            };
            let import_indices: Vec<usize> = (0..self.import_rows.len()).collect();
            let mut row_drag_bands: Vec<(usize, f32, f32)> = Vec::new();
            
            let _import_active_cell: Option<(GridColumn, usize)> = None;
            let mut import_scroll_y = self.import_grid_scroll_offset;
            #[allow(deprecated)]
            let import_scroll = egui::ScrollArea::vertical()
              .id_salt("import_grid_scroll")
              .max_height(review_table_height)
              .auto_shrink([false, false])
              .drag_to_scroll(false)
              .scroll_offset(egui::vec2(0.0, import_scroll_y))
              .show(ui, |ui| {
                ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
                TableBuilder::new(ui)
              .striped(false)
              .resizable(true)
              .vscroll(false)
              .min_scrolled_height(120.0)
              .max_scroll_height(review_table_height)
              .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
              .column(import_columns[0].clone())
              .column(import_columns[1].clone())
              .column(import_columns[2].clone())
              .column(import_columns[3].clone())
              .column(import_columns[4].clone())
              .column(import_columns[5].clone())
              .header(GRID_HEADER_HEIGHT, |mut header| {
                header.col(|ui| { grid_header(ui, "Date"); });
                header.col(|ui| { grid_header(ui, "Amount"); });
                header.col(|ui| { grid_header(ui, "Member"); });
                header.col(|ui| { grid_header(ui, "Category"); });
                header.col(|ui| { grid_header(ui, "Vendor"); });
                header.col(|ui| { grid_header(ui, "Description"); });
              })
              .body(|mut body| {
                for idx in 0..self.import_rows.len() {
                  body.row(GRID_ROW_HEIGHT, |mut row| {
                    row.col(|ui| {
                      let column = GridColumn::Date;
              let row_rect = ui.max_rect();
              row_drag_bands.push((idx, row_rect.top(), row_rect.bottom()));
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.import_grid_selection,
                &mut self.import_grid_drag,
                &mut self.import_grid_edit_cell,
              );
                      let _selected = grid_row_selected(&self.import_grid_selection, column, idx);
              ui.horizontal(|ui| {
let editing = grid_cell_editing(&self.import_grid_edit_cell, column, idx);
                        if editing {
                          self.apply_import_grid_typeahead(column, idx);
                        }
                        let (_response, date_changed) = ui_grid_date_edit(
                          ui,
                          &mut self.import_rows[idx].date,
                          &mut self.expense_grid_edit_original,
                          &mut self.import_grid_edit_cell,
                          import_cell_id(column, idx),
                          editing,
                        );
                        let press_enter = editing && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if date_changed || press_enter {
                          pending_amount_updates.push(idx);
                          self.import_grid_edit_cell = None;
                          self.expense_grid_edit_original = None;
                          let targets = self.import_commit_targets(column, idx);
                          self.apply_import_field_value(&targets, "date", idx);
                        }
                      });
                    });
                    row.col(|ui| {
                      let column = GridColumn::Amount;
              let _row_rect = ui.max_rect();
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.import_grid_selection,
                &mut self.import_grid_drag,
                &mut self.import_grid_edit_cell,
              );
                      let _selected = grid_row_selected(&self.import_grid_selection, column, idx);
              ui.horizontal(|ui| {
let editing = grid_cell_editing(&self.import_grid_edit_cell, column, idx);
                        if editing {
                          self.apply_import_grid_typeahead(column, idx);
                          let amount_response = ui_grid_text_edit(
                            ui,
                            &mut self.import_rows[idx].amount_input,
                            &mut self.expense_grid_edit_original,
                            &mut self.import_grid_edit_cell,
                            import_cell_id(column, idx),
                            true,
                          );
                          if amount_response.changed() {
                            self.import_rows[idx].amount_input.retain(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '-');
                            pending_amount_updates.push(idx);
                          }
                          if grid_field_committed(&amount_response) {
                            let orig = self.expense_grid_edit_original.take().unwrap_or_default();
                            if let Some(cents) = parse_amount_cents(&self.import_rows[idx].amount_input) {
                                self.import_rows[idx].amount_input = money(cents).replace('$', "");
                                self.import_grid_edit_cell = None;
                                let targets = self.import_commit_targets(column, idx);
                                self.apply_import_field_value(&targets, "amount", idx);
                            } else {
                                self.import_rows[idx].amount_input = orig;
                                self.import_grid_edit_cell = None;
                            }
                          }
                        } else {
                          let display_value = if let Some(cents) = parse_amount_cents(&self.import_rows[idx].amount_input) {
                            money(cents)
                          } else {
                            self.import_rows[idx].amount_input.clone()
                          };
                          let mut temp = display_value;
                          ui_grid_text_edit(
                            ui,
                            &mut temp,
                            &mut self.expense_grid_edit_original,
                            &mut self.import_grid_edit_cell,
                            import_cell_id(column, idx),
                            false,
                          );
                        }
                      });
                      process_grid_column_cell(
                      ui,
                      column,
                      idx,
                      &mut self.import_grid_selection,
                      &mut self.import_grid_drag,
                      &mut self.import_grid_edit_cell,
                    );
                    });
                    row.col(|ui| {
                      let column = GridColumn::Member;
                      let member_bg = self.member_color(&self.import_rows[idx].member);
                      if member_bg.a() > 0 && member_bg != Color32::TRANSPARENT {
                        ui.painter().rect_filled(ui.max_rect(), 0.0, member_bg);
                      }
                      process_grid_column_cell(
                        ui,
                        column,
                        idx,
                        &mut self.import_grid_selection,
                        &mut self.import_grid_drag,
                        &mut self.import_grid_edit_cell,
                      );
                      let _selected = grid_row_selected(&self.import_grid_selection, column, idx);
                      let mut member_chevron_rect = egui::Rect::NOTHING;
              ui.horizontal(|ui| {
ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        let editing = grid_cell_editing(&self.import_grid_edit_cell, column, idx);
                        if editing {
                          self.apply_import_grid_typeahead(column, idx);
                        }
                        if editing && self.expense_grid_edit_original.is_none() {
                          self.expense_grid_edit_original = Some(self.import_rows[idx].member.clone());
                        }
                        let member_bg = self.member_color(&self.import_rows[idx].member);
                        let cell = member_cell_ui(
                          ui,
                          &mut self.import_rows[idx].member,
                          member_bg,
                          import_cell_id(column, idx),
                          editing,
                        );
                        if editing && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                          if let Some(orig) = self.expense_grid_edit_original.take() {
                            self.import_rows[idx].member = orig;
                          }
                          self.import_grid_edit_cell = None;
                          cell.text.surrender_focus();
                        }
                        member_chevron_rect = cell.chevron.rect;
                        let (picked_member, member_popup_rect) = if editing {
                          category_autocomplete_popup(
                          ui,
                          &cell.text,
                          &mut self.import_rows[idx].member,
                          &member_candidates,
                          &mut autocomplete_selection,
                          )
                        } else {
                          (false, egui::Rect::NOTHING)
                        };
                        let mut member_pick = false;
                        let mut picked_member_value: Option<String> = None;
                        let current_member = self.import_rows[idx].member.trim().to_string();
                        grid_chevron_picker_popup(ui, &cell.chevron, member_menu_w, member_scroll, |ui| {
                          if member_none_row_selectable(ui, current_member.is_empty()) {
                            picked_member_value = Some(String::new());
                            member_pick = true;
                            ui.close();
                          }
                          ui.separator();
                          for member in &self.members {
                            if member_row_selectable(ui, member, member.name.eq_ignore_ascii_case(&current_member)) {
                              picked_member_value = Some(member.name.clone());
                              member_pick = true;
                              ui.close();
                            }
                          }
                        });
                        let field_committed =
                          grid_text_field_committed(ui, &cell.text, member_popup_rect);
                        if picked_member || member_pick || field_committed {
                          let targets = self.import_commit_targets(column, idx);
                          let value =
                            picked_member_value.unwrap_or_else(|| self.import_rows[idx].member.clone());
                          self.apply_import_member_value(&targets, &value);
                          pending_import_member_commits.extend(targets);
                        }
                      });
                      process_grid_column_cell(
                      ui,
                      column,
                      idx,
                      &mut self.import_grid_selection,
                      &mut self.import_grid_drag,
                      &mut self.import_grid_edit_cell,
                    );
                    });
                    row.col(|ui| {
                      let column = GridColumn::Category;
                      let cat_bg = self.category_color(&self.import_rows[idx].category);
                      if cat_bg.a() > 0 && cat_bg != Color32::TRANSPARENT {
                        ui.painter().rect_filled(ui.max_rect(), 0.0, cat_bg);
                      }
                      process_grid_column_cell(
                        ui,
                        column,
                        idx,
                        &mut self.import_grid_selection,
                        &mut self.import_grid_drag,
                        &mut self.import_grid_edit_cell,
                      );
                      let _selected = grid_row_selected(&self.import_grid_selection, column, idx);
                      let mut category_chevron_rect = egui::Rect::NOTHING;
              ui.horizontal(|ui| {
ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        let editing = grid_cell_editing(&self.import_grid_edit_cell, column, idx);
                        if editing {
                          self.apply_import_grid_typeahead(column, idx);
                        }
                        if editing && self.expense_grid_edit_original.is_none() {
                          self.expense_grid_edit_original = Some(self.import_rows[idx].category.clone());
                        }
                        let cat_bg = self.category_color(&self.import_rows[idx].category);
                        let cell = category_cell_ui(
                          ui,
                          &mut self.import_rows[idx].category,
                          cat_bg,
                          import_cell_id(column, idx),
                          editing,
                        );
                        if editing && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                          if let Some(orig) = self.expense_grid_edit_original.take() {
                            self.import_rows[idx].category = orig;
                          }
                          self.import_grid_edit_cell = None;
                          cell.text.surrender_focus();
                        }
                        category_chevron_rect = cell.chevron.rect;
                        let (picked_category, category_popup_rect) = if editing {
                          category_autocomplete_popup(
                          ui,
                          &cell.text,
                          &mut self.import_rows[idx].category,
                          &category_candidates,
                          &mut autocomplete_selection,
                          )
                        } else {
                          (false, egui::Rect::NOTHING)
                        };
                        let mut combobox_pick = false;
                        let mut picked_category_value: Option<String> = None;
                        grid_chevron_picker_popup(ui, &cell.chevron, category_menu_w, Some(280.0), |ui| {
                          if let Some(label) = self.ui_category_picker_menu(ui, &self.import_rows[idx].category) {
                            picked_category_value = Some(label);
                            combobox_pick = true;
                          }
                        });
                        let field_committed =
                          grid_text_field_committed(ui, &cell.text, category_popup_rect);
                        if picked_category || combobox_pick || field_committed {
                          let targets = self.import_commit_targets(column, idx);
                          let mut value = picked_category_value
                            .unwrap_or_else(|| self.import_rows[idx].category.clone());
                          if let Some(matched) = find_category_by_label(&self.categories, &value) {
                            let parents = category_parent_map(&self.categories);
                            value = matched.full_label(&parents);
                          } else if !value.trim().is_empty() {
                            value.clear();
                            self.log("[import] category must exist in Settings");
                          }
                          self.apply_import_category_value(&targets, &value);
                          self.register_categories_for_import_rows(&targets);
                        }
                      });
                      process_grid_column_cell(
                      ui,
                      column,
                      idx,
                      &mut self.import_grid_selection,
                      &mut self.import_grid_drag,
                      &mut self.import_grid_edit_cell,
                    );
                    });
                    row.col(|ui| {
                      let column = GridColumn::Vendor;
              let _row_rect = ui.max_rect();
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.import_grid_selection,
                &mut self.import_grid_drag,
                &mut self.import_grid_edit_cell,
              );
                      let _selected = grid_row_selected(&self.import_grid_selection, column, idx);
              ui.horizontal(|ui| {
let editing = grid_cell_editing(&self.import_grid_edit_cell, column, idx);
                        if editing {
                          self.apply_import_grid_typeahead(column, idx);
                        }
                        let response = ui_grid_text_edit(
                          ui,
                          &mut self.import_rows[idx].vendor,
                          &mut self.expense_grid_edit_original,
                          &mut self.import_grid_edit_cell,
                          import_cell_id(column, idx),
                          editing,
                        );
                        let (picked_vendor, vendor_popup_rect) = if editing {
                          autocomplete_with_popup(
                          ui,
                          &response,
                          &mut self.import_rows[idx].vendor,
                          &vendor_candidates,
                          &mut autocomplete_selection,
                          )
                        } else {
                          (false, egui::Rect::NOTHING)
                        };
                        if picked_vendor || grid_text_field_committed(ui, &response, vendor_popup_rect) {
                          let targets = self.import_commit_targets(column, idx);
                          self.apply_import_field_value(&targets, "vendor", idx);
                        }
                      });
                      process_grid_column_cell(
                      ui,
                      column,
                      idx,
                      &mut self.import_grid_selection,
                      &mut self.import_grid_drag,
                      &mut self.import_grid_edit_cell,
                    );
                    });
                    row.col(|ui| {
                      let column = GridColumn::Description;
              let _row_rect = ui.max_rect();
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.import_grid_selection,
                &mut self.import_grid_drag,
                &mut self.import_grid_edit_cell,
              );
                      let _selected = grid_row_selected(&self.import_grid_selection, column, idx);
              ui.horizontal(|ui| {
let editing = grid_cell_editing(&self.import_grid_edit_cell, column, idx);
                        if editing {
                          self.apply_import_grid_typeahead(column, idx);
                        }
                        let response = ui_grid_text_edit(
                          ui,
                          &mut self.import_rows[idx].description,
                          &mut self.expense_grid_edit_original,
                          &mut self.import_grid_edit_cell,
                          import_cell_id(column, idx),
                          editing,
                        );
                        let (picked_description, description_popup_rect) = if editing {
                          autocomplete_with_popup(
                          ui,
                          &response,
                          &mut self.import_rows[idx].description,
                          &description_candidates,
                          &mut autocomplete_selection,
                          )
                        } else {
                          (false, egui::Rect::NOTHING)
                        };
                        if picked_description
                          || grid_text_field_committed(ui, &response, description_popup_rect)
                        {
                          let targets = self.import_commit_targets(column, idx);
                          self.apply_import_field_value(&targets, "description", idx);
                        }
                      });
                      process_grid_column_cell(
                      ui,
                      column,
                      idx,
                      &mut self.import_grid_selection,
                      &mut self.import_grid_drag,
                      &mut self.import_grid_edit_cell,
                    );
                    });
                  });
                }
              });
              });
            import_scroll_y = import_scroll.state.offset.y;
            grid_apply_shift_hand_pan(ui.ctx(), &mut import_scroll_y);
            self.import_grid_scroll_offset = import_scroll_y;
            let _ = grid_try_start_edit_from_typing(
              ui,
              &self.import_grid_selection,
              &mut self.import_grid_edit_cell,
              &mut self.import_grid_typeahead,
            );
            if let Some((column, idx)) = self.import_grid_edit_cell {
              if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                if self.import_autocomplete_active(column, idx) {
                  self.tab_fill_import_cell(column, idx, autocomplete_selection);
                }
                self.commit_import_grid_cell(
                  column,
                  idx,
                  &mut pending_amount_updates,
                  &mut pending_import_member_commits,
                );
              }
              request_import_cell_focus(ui, column, idx);
            }
            grid_snap_drag_to_pointer_y(
              ui,
              &row_drag_bands,
              &import_indices,
              &mut self.import_grid_drag,
              &mut self.import_grid_selection,
            );
            if self.import_grid_drag.is_some() {
              ui.ctx().request_repaint();
            }
            finish_grid_drag(&mut self.import_grid_drag, ui);
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

  fn import_autocomplete_candidates_for_column(&self, column: GridColumn) -> Vec<String> {
    match column {
      GridColumn::Member => self.cached_member_candidates.clone(),
      GridColumn::Category => self.cached_category_candidates.clone(),
      GridColumn::Vendor => self.cached_import_vendor_candidates.clone(),
      GridColumn::Description => self.cached_import_description_candidates.clone(),
      GridColumn::Date | GridColumn::Amount => Vec::new(),
    }
  }

  fn import_autocomplete_active(&self, column: GridColumn, row: usize) -> bool {
    let Some(import_row) = self.import_rows.get(row) else {
      return false;
    };
    let candidates = self.import_autocomplete_candidates_for_column(column);
    let value = match column {
      GridColumn::Member => &import_row.member,
      GridColumn::Category => &import_row.category,
      GridColumn::Vendor => &import_row.vendor,
      GridColumn::Description => &import_row.description,
      GridColumn::Date | GridColumn::Amount => return false,
    };
    !autocomplete_suggestions_list(value, &candidates).is_empty()
  }

  fn tab_fill_import_cell(&mut self, column: GridColumn, row: usize, selected_index: usize) {
    let candidates = self.import_autocomplete_candidates_for_column(column);
    let Some(import_row) = self.import_rows.get_mut(row) else {
      return;
    };
    let value = match column {
      GridColumn::Member => &mut import_row.member,
      GridColumn::Category => &mut import_row.category,
      GridColumn::Vendor => &mut import_row.vendor,
      GridColumn::Description => &mut import_row.description,
      GridColumn::Date | GridColumn::Amount => return,
    };
    let suggestions = autocomplete_suggestions_list(value, &candidates);
    if let Some(suggestion) = suggestions.get(selected_index) {
      *value = suggestion.clone();
    }
  }

  fn commit_import_grid_cell(
    &mut self,
    column: GridColumn,
    row: usize,
    pending_amount_updates: &mut Vec<usize>,
    pending_import_member_commits: &mut Vec<usize>,
  ) {
    self.expense_grid_edit_original = None;
    let targets = self.import_commit_targets(column, row);
    match column {
      GridColumn::Date => {
        self.apply_import_field_value(&targets, "date", row);
      }
      GridColumn::Amount => {
        self.apply_import_field_value(&targets, "amount", row);
        pending_amount_updates.extend(targets);
      }
      GridColumn::Member => {
        let value = self
          .import_rows
          .get(row)
          .map(|import_row| import_row.member.clone())
          .unwrap_or_default();
        self.apply_import_member_value(&targets, &value);
        pending_import_member_commits.extend(targets);
      }
      GridColumn::Category => {
        let mut value = self
          .import_rows
          .get(row)
          .map(|import_row| import_row.category.clone())
          .unwrap_or_default();
        if let Some(matched) = find_category_by_label(&self.categories, &value) {
          let parents = category_parent_map(&self.categories);
          value = matched.full_label(&parents);
        } else if !value.trim().is_empty() {
          value.clear();
          self.log("[import] category must exist in Settings");
        }
        self.apply_import_category_value(&targets, &value);
        self.register_categories_for_import_rows(&targets);
      }
      GridColumn::Vendor => {
        self.apply_import_field_value(&targets, "vendor", row);
        self.rebuild_import_cached_candidates();
      }
      GridColumn::Description => {
        self.apply_import_field_value(&targets, "description", row);
        self.rebuild_import_cached_candidates();
      }
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
  }

  fn apply_import_grid_typeahead(&mut self, column: GridColumn, row: usize) {
    let Some(ch) = self.import_grid_typeahead.take() else {
      return;
    };
    let Some(import_row) = self.import_rows.get_mut(row) else {
      return;
    };
    match column {
      GridColumn::Date => {
        import_row.date.clear();
        import_row.date.push(ch);
      }
      GridColumn::Amount => {
        import_row.amount_input.clear();
        import_row.amount_input.push(ch);
      }
      GridColumn::Member => {
        import_row.member.clear();
        import_row.member.push(ch);
      }
      GridColumn::Category => {
        import_row.category.clear();
        import_row.category.push(ch);
      }
      GridColumn::Vendor => {
        import_row.vendor.clear();
        import_row.vendor.push(ch);
        self.rebuild_import_cached_candidates();
      }
      GridColumn::Description => {
        import_row.description.clear();
        import_row.description.push(ch);
        self.rebuild_import_cached_candidates();
      }
    }
  }

  fn import_commit_targets(&self, column: GridColumn, active_row: usize) -> Vec<usize> {
    grid_commit_targets(&self.import_grid_selection, column, active_row)
  }

  fn apply_import_member_value(&mut self, indices: &[usize], value: &str) {
    let value = value.trim().to_string();
    for &idx in indices {
      if let Some(row) = self.import_rows.get_mut(idx) {
        row.member = value.clone();
      }
    }
  }

  fn apply_import_category_value(&mut self, indices: &[usize], value: &str) {
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

  fn apply_import_field_value(&mut self, indices: &[usize], field: &str, source_idx: usize) {
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
