use eframe::egui::{self, Color32, RichText};
use egui_extras::TableBuilder;
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
    let table_height = ui.available_height().max(120.0);
    let mut autocomplete_selection = self.autocomplete_selection;

    let old_spacing = ui.spacing().item_spacing;
    ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
    let expense_columns = spreadsheet_columns(ui, 100.0, 90.0, 100.0, 200.0, 160.0);
    let category_menu_w = category_picker_menu_width(ui, &self.categories, 200.0);
    let member_menu_w = MEMBER_PICKER_MIN_WIDTH;
    let member_count = self.members.len();
    let member_scroll = if member_count > 10 {
      Some(member_picker_max_height(member_count))
    } else {
      None
    };
    let mut expense_sort = self.expense_sort;
    let sorted_indices = self.cached_sorted_expense_indices.clone();
    let mut row_drag_bands: Vec<(usize, f32, f32)> = Vec::new();
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
    #[allow(deprecated)]
    let expense_scroll = egui::ScrollArea::vertical()
      .id_salt("expense_grid_scroll")
      .max_height(table_height)
      .auto_shrink([false, false])
      .drag_to_scroll(false)
      .scroll_offset(egui::vec2(0.0, expense_scroll_y))
      .show(ui, |ui| {
        ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
        TableBuilder::new(ui)
      .striped(true)
      .resizable(true)
      .vscroll(false)
      .min_scrolled_height(120.0)
      .max_scroll_height(table_height)
      .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
      .column(expense_columns[0].clone())
      .column(expense_columns[1].clone())
      .column(expense_columns[2].clone())
      .column(expense_columns[3].clone())
      .column(expense_columns[4].clone())
      .column(expense_columns[5].clone())
      .header(GRID_HEADER_HEIGHT, |mut header| {
        header.col(|ui| sortable_grid_header(ui, "Date", ExpenseSortColumn::Date, &mut expense_sort));
        header.col(|ui| sortable_grid_header(ui, "Amount", ExpenseSortColumn::Amount, &mut expense_sort));
        header.col(|ui| sortable_grid_header(ui, "Member", ExpenseSortColumn::Member, &mut expense_sort));
        header.col(|ui| sortable_grid_header(ui, "Category", ExpenseSortColumn::Category, &mut expense_sort));
        header.col(|ui| sortable_grid_header(ui, "Vendor", ExpenseSortColumn::Vendor, &mut expense_sort));
        header.col(|ui| sortable_grid_header(ui, "Description", ExpenseSortColumn::Description, &mut expense_sort));
      })
      .body(|mut body| {
        for &idx in &sorted_indices {
          body.row(GRID_ROW_HEIGHT, |mut row| {
            row.col(|ui| {
              // Paint row separator at bottom of cell
              let cell_rect = ui.max_rect();
              ui.painter().line_segment(
                [cell_rect.left_bottom(), egui::pos2(cell_rect.right() + 2000.0, cell_rect.bottom())],
                egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
              );
              let column = GridColumn::Date;
              let row_rect = ui.max_rect();
              row_drag_bands.push((idx, row_rect.top(), row_rect.bottom()));
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
              let _selected = grid_row_selected(&self.expense_grid_selection, column, idx);
              ui.horizontal(|ui| {
                let editing = grid_cell_editing(&self.expense_grid_edit_cell, column, idx);
                if editing {
                  self.apply_expense_grid_typeahead(column, idx);
                }
                let (_response, date_changed) = ui_grid_date_edit(
                  ui,
                  &mut self.expenses[idx].date,
                  &mut self.expense_grid_edit_original,
                  &mut self.expense_grid_edit_cell,
                  expense_cell_id(column, idx),
                  editing,
                );
                let press_enter = editing && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if date_changed || press_enter {
                  pending_updates.push((idx, "date"));
                  self.expense_grid_edit_cell = None;
                  self.expense_grid_edit_original = None;
                  self.commit_expense_cell(
                    column,
                    idx,
                    &mut pending_updates,
                    &mut pending_category_commits,
                    &mut pending_member_commits,
                  );
                }
                if editing && expense_cell_has_focus(ui.ctx(), column, idx) {
                  self.active_expense_cell = Some((column, idx));
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
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
              let _selected = grid_row_selected(&mut self.expense_grid_selection, column, idx);
              ui.horizontal(|ui| {
                let editing = grid_cell_editing(&mut self.expense_grid_edit_cell, column, idx);
                if editing {
                  self.apply_expense_grid_typeahead(column, idx);
                  let amount_response = ui_grid_text_edit(
                    ui,
                    &mut self.expenses[idx].amount_input,
                    &mut self.expense_grid_edit_original,
                    &mut self.expense_grid_edit_cell,
                    expense_cell_id(column, idx),
                    true,
                  );
                  if amount_response.changed() {
                    self.expenses[idx].amount_input.retain(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '-');
                    // Collapse any extra decimal points: keep only the first one
                    let s = self.expenses[idx].amount_input.clone();
                    let mut seen_dot = false;
                    self.expenses[idx].amount_input = s.chars().filter(|&c| {
                      if c == '.' {
                        if seen_dot { return false; }
                        seen_dot = true;
                      }
                      true
                    }).collect();
                    pending_updates.push((idx, "amount"));
                  }
                  if expense_cell_has_focus(ui.ctx(), column, idx) {
                    self.active_expense_cell = Some((column, idx));
                  }
                  if grid_text_field_committed(ui, &amount_response, egui::Rect::NOTHING) {
                    let orig = self.expense_grid_edit_original.take().unwrap_or_default();
                    if let Some(cents) = parse_amount_cents(&self.expenses[idx].amount_input) {
                        self.expenses[idx].amount_input = money(cents).replace('$', "");
                        self.expense_grid_edit_cell = None;
                        self.commit_expense_cell(
                          column,
                          idx,
                          &mut pending_updates,
                          &mut pending_category_commits,
                          &mut pending_member_commits,
                        );
                    } else {
                        self.expenses[idx].amount_input = orig;
                        self.expense_grid_edit_cell = None;
                    }
                  }
                } else {
                  let display_value = if let Some(cents) = parse_amount_cents(&self.expenses[idx].amount_input) {
                    money(cents)
                  } else {
                    self.expenses[idx].amount_input.clone()
                  };
                  let mut temp = display_value;
                  ui_grid_text_edit(
                    ui,
                    &mut temp,
                    &mut self.expense_grid_edit_original,
                    &mut self.expense_grid_edit_cell,
                    expense_cell_id(column, idx),
                    false,
                  );
                }
              });
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
            });
            row.col(|ui| {
              let column = GridColumn::Member;
              let member_bg = self.member_color(&self.expenses[idx].member);
              if member_bg.a() > 0 && member_bg != Color32::TRANSPARENT {
                ui.painter().rect_filled(ui.max_rect(), 0.0, member_bg);
              }
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
              let _selected = grid_row_selected(&self.expense_grid_selection, column, idx);
              let mut member_chevron_rect = egui::Rect::NOTHING;
              ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let editing = grid_cell_editing(&self.expense_grid_edit_cell, column, idx);
                if editing {
                  self.apply_expense_grid_typeahead(column, idx);
                }
                if editing && self.expense_grid_edit_original.is_none() {
                  self.expense_grid_edit_original = Some(self.expenses[idx].member.clone());
                }
                let member_bg = self.member_color(&self.expenses[idx].member);
                let cell = member_cell_ui(
                  ui,
                  &mut self.expenses[idx].member,
                  member_bg,
                  expense_cell_id(column, idx),
                  editing,
                );
                if editing && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                  if let Some(orig) = self.expense_grid_edit_original.take() {
                    self.expenses[idx].member = orig;
                  }
                  self.expense_grid_edit_cell = None;
                  cell.text.surrender_focus();
                }
                member_chevron_rect = cell.chevron.rect;
                if editing && expense_cell_has_focus(ui.ctx(), column, idx) {
                  self.active_expense_cell = Some((column, idx));
                }
                if editing {
                  if let Some(picked) = show_cell_autocomplete_popup(
                    ui,
                    &cell.text,
                    &self.expenses[idx].member,
                    &member_candidates,
                    &mut autocomplete_selection,
                  ) {
                    self.expenses[idx].member = picked;
                    self.commit_expense_cell(
                      column,
                      idx,
                      &mut pending_updates,
                      &mut pending_category_commits,
                      &mut pending_member_commits,
                    );
                  }
                }
                let mut member_pick = false;
                let mut picked_member_value: Option<String> = None;
                let current_member = self.expenses[idx].member.trim().to_string();
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
                if member_pick {
                  if let Some(value) = picked_member_value {
                    self.expenses[idx].member = value;
                  }
                  self.commit_expense_cell(
                    column,
                    idx,
                    &mut pending_updates,
                    &mut pending_category_commits,
                    &mut pending_member_commits,
                  );
                } else if editing && grid_text_field_committed(ui, &cell.text, egui::Rect::NOTHING) {
                  self.expense_grid_edit_cell = None;
                  self.commit_expense_cell(
                    column,
                    idx,
                    &mut pending_updates,
                    &mut pending_category_commits,
                    &mut pending_member_commits,
                  );
                }
              });
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
            });
            row.col(|ui| {
              let column = GridColumn::Category;
              let cat_bg = self.category_color(&self.expenses[idx].category);
              if cat_bg.a() > 0 && cat_bg != Color32::TRANSPARENT {
                ui.painter().rect_filled(ui.max_rect(), 0.0, cat_bg);
              }
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
              let _selected = grid_row_selected(&self.expense_grid_selection, column, idx);
              let mut category_chevron_rect = egui::Rect::NOTHING;
              ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let editing = grid_cell_editing(&self.expense_grid_edit_cell, column, idx);
                if editing {
                  self.apply_expense_grid_typeahead(column, idx);
                }
                if editing && self.expense_grid_edit_original.is_none() {
                  self.expense_grid_edit_original = Some(self.expenses[idx].category.clone());
                }
                let cat_bg = self.category_color(&self.expenses[idx].category);
                let cell = category_cell_ui(
                  ui,
                  &mut self.expenses[idx].category,
                  cat_bg,
                  expense_cell_id(column, idx),
                  editing,
                );
                if editing && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                  if let Some(orig) = self.expense_grid_edit_original.take() {
                    self.expenses[idx].category = orig;
                  }
                  self.expense_grid_edit_cell = None;
                  cell.text.surrender_focus();
                }
                category_chevron_rect = cell.chevron.rect;
                if editing && expense_cell_has_focus(ui.ctx(), column, idx) {
                  self.active_expense_cell = Some((column, idx));
                }
                if editing {
                  if let Some(picked) = show_cell_autocomplete_popup(
                    ui,
                    &cell.text,
                    &self.expenses[idx].category,
                    &category_candidates,
                    &mut autocomplete_selection,
                  ) {
                    self.expenses[idx].category = picked;
                    self.commit_expense_cell(
                      column,
                      idx,
                      &mut pending_updates,
                      &mut pending_category_commits,
                      &mut pending_member_commits,
                    );
                  }
                }
                let mut combobox_pick = false;
                let mut picked_category_value: Option<String> = None;
                grid_chevron_picker_popup(ui, &cell.chevron, category_menu_w, Some(280.0), |ui| {
                  if let Some(label) = self.ui_category_picker_menu(ui, &self.expenses[idx].category) {
                    picked_category_value = Some(label);
                    combobox_pick = true;
                  }
                });
                let category_name = self.expenses[idx].category.trim().to_string();
                cell.text.context_menu(|ui| {
                  if category_name.is_empty() {
                    ui.label(RichText::new("Pick a category from the list").small().weak());
                    return;
                  }
                  if let Some(category) = find_category_by_label(&self.categories, &category_name) {
                    if ui.button("Edit color…").clicked() {
                      self.open_category_color_popup(category.id, category_name, cat_bg);
                      ui.close();
                    }
                  }
                });
                if combobox_pick {
                  if let Some(value) = picked_category_value {
                    self.expenses[idx].category = value;
                  }
                  self.commit_expense_cell(
                    column,
                    idx,
                    &mut pending_updates,
                    &mut pending_category_commits,
                    &mut pending_member_commits,
                  );
                } else if editing && grid_text_field_committed(ui, &cell.text, egui::Rect::NOTHING) {
                  self.expense_grid_edit_cell = None;
                  self.commit_expense_cell(
                    column,
                    idx,
                    &mut pending_updates,
                    &mut pending_category_commits,
                    &mut pending_member_commits,
                  );
                }
              });
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
            });
            row.col(|ui| {
              let column = GridColumn::Vendor;
              let _row_rect = ui.max_rect();
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
              let _selected = grid_row_selected(&mut self.expense_grid_selection, column, idx);
              ui.horizontal(|ui| {
                let editing = grid_cell_editing(&mut self.expense_grid_edit_cell, column, idx);
                if editing {
                  self.apply_expense_grid_typeahead(column, idx);
                }
                let response = ui_grid_text_edit(
                  ui,
                  &mut self.expenses[idx].vendor,
                  &mut self.expense_grid_edit_original,
                  &mut self.expense_grid_edit_cell,
                  expense_cell_id(column, idx),
                  editing,
                );
                if editing && expense_cell_has_focus(ui.ctx(), column, idx) {
                  self.active_expense_cell = Some((column, idx));
                }
                if editing {
                  if let Some(picked) = show_cell_autocomplete_popup(
                    ui,
                    &response,
                    &mut self.expenses[idx].vendor,
                    &vendor_candidates,
                    &mut autocomplete_selection,
                  ) {
                    self.expenses[idx].vendor = picked;
                    self.commit_expense_cell(
                      column,
                      idx,
                      &mut pending_updates,
                      &mut pending_category_commits,
                      &mut pending_member_commits,
                    );
                  }
                }
                if response.changed() {
                  pending_updates.push((idx, "vendor"));
                }
                if editing && grid_text_field_committed(ui, &response, egui::Rect::NOTHING) {
                  self.expense_grid_edit_cell = None;
                  self.commit_expense_cell(
                    column,
                    idx,
                    &mut pending_updates,
                    &mut pending_category_commits,
                    &mut pending_member_commits,
                  );
                }
              });
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
            });
            row.col(|ui| {
              let column = GridColumn::Description;
              let _row_rect = ui.max_rect();
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
              let _selected = grid_row_selected(&self.expense_grid_selection, column, idx);
              ui.horizontal(|ui| {
                let editing = grid_cell_editing(&mut self.expense_grid_edit_cell, column, idx);
                if editing {
                  self.apply_expense_grid_typeahead(column, idx);
                }
                let response = ui_grid_text_edit(
                  ui,
                  &mut self.expenses[idx].description,
                  &mut self.expense_grid_edit_original,
                  &mut self.expense_grid_edit_cell,
                  expense_cell_id(column, idx),
                  editing,
                );
                if editing && expense_cell_has_focus(ui.ctx(), column, idx) {
                  self.active_expense_cell = Some((column, idx));
                }
                if editing {
                  if let Some(picked) = show_cell_autocomplete_popup(
                    ui,
                    &response,
                    &self.expenses[idx].description,
                    &description_candidates,
                    &mut autocomplete_selection,
                  ) {
                    self.expenses[idx].description = picked;
                    self.commit_expense_cell(
                      column,
                      idx,
                      &mut pending_updates,
                      &mut pending_category_commits,
                      &mut pending_member_commits,
                    );
                  }
                }
                if response.changed() {
                  pending_updates.push((idx, "description"));
                }
                if editing && grid_text_field_committed(ui, &response, egui::Rect::NOTHING) {
                  self.expense_grid_edit_cell = None;
                  self.commit_expense_cell(
                    column,
                    idx,
                    &mut pending_updates,
                    &mut pending_category_commits,
                    &mut pending_member_commits,
                  );
                }
              });
              process_grid_column_cell(
                ui,
                column,
                idx,
                &mut self.expense_grid_selection,
                &mut self.expense_grid_drag,
                &mut self.expense_grid_edit_cell,
              );
            });
          });
        }
      });
      });
    expense_scroll_y = expense_scroll.state.offset.y;
    grid_apply_shift_hand_pan(ui.ctx(), &mut expense_scroll_y);
    self.expense_grid_scroll_offset = expense_scroll_y;
    let _ = grid_try_start_edit_from_typing(
      ui,
      &self.expense_grid_selection,
      &mut self.expense_grid_edit_cell,
      &mut self.expense_grid_typeahead,
    );
    if let Some((column, idx)) = self.expense_grid_edit_cell {
      if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
        let multi = self
          .expense_grid_selection
          .as_ref()
          .is_some_and(|selection| selection.column == column && selection.rows.len() > 1);
        if multi && !self.expense_autocomplete_active(column, idx) {
          self.commit_expense_cell(
            column,
            idx,
            &mut pending_updates,
            &mut pending_category_commits,
            &mut pending_member_commits,
          );
        }
      }
    }
    grid_snap_drag_to_pointer_y(
      ui,
      &row_drag_bands,
      &sorted_indices,
      &mut self.expense_grid_drag,
      &mut self.expense_grid_selection,
    );
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

