/// Unified expense/import grid body renderer.
///
/// Both the Expense spreadsheet and the Import Review grid call this single function.
/// The only differences between the two call sites are:
///   - which row slice they pass (Expense vs ImportRow — both implement GridRow)
///   - which cell-ID prefix they use (avoids egui ID collisions)
///   - how they process the returned `GridResult` (DB commit vs in-memory update)
use eframe::egui::{self, Color32, RichText};
use egui_extras::TableBuilder;
use crate::models::*;
use crate::db::*;
use crate::ui::widgets::*;
use crate::ui::popups::{member_color_for, category_color_for, category_picker_menu_ui};

// ---------------------------------------------------------------------------
// Result type — carries everything the caller needs to commit after rendering
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct GridResult {
  /// (row_index, field_name) pairs for simple field writes
  pub pending_field_updates: Vec<(usize, &'static str)>,
  /// Row indices whose category field needs validated & committed
  pub pending_category_commits: Vec<usize>,
  /// Row indices whose member field was changed
  pub pending_member_commits: Vec<usize>,
  /// If Some, the caller should open a category color popup for (id, label, current_color)
  pub open_color_popup: Option<(i64, String, Color32)>,
  /// The cell that currently has keyboard focus (for status-bar display)
  pub active_cell: Option<(GridColumn, usize)>,
  /// Selected indices requested for deletion
  pub delete_rows: Option<Vec<usize>>,
  /// Selected indices requested for forced deletion (bypassing confirmation)
  pub force_delete_rows: Option<Vec<usize>>,
  /// If Some, the user clicked a column header to trigger sorting
  pub clicked_sort_column: Option<ExpenseSortColumn>,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn render_grid<R: GridRow>(
  ui: &mut egui::Ui,
  rows: &mut Vec<R>,
  sorted_indices: &[usize],
  state: &mut GridState,
  autocomplete_selection: &mut usize,
  // --- Candidate lists for autocomplete ---
  vendor_candidates: &[String],
  category_candidates: &[String],
  member_candidates: &[String],
  description_candidates: &[String],
  // --- People / category data ---
  members: &[HouseholdMember],
  categories: &[Category],

  // --- Unique string prefix so expense vs import cells don't share egui IDs ---
  cell_id_prefix: &'static str,
  // --- Whether to draw per-row separator lines (expense grid has them) ---
  show_row_separator: bool,
) -> GridResult {
  let mut result = GridResult::default();

  // ── KEYBOARD INTERACTION HOOKS ───────────────────────────────────────────
  // Only handle Escape at the grid level when NOT editing a cell.
  // When editing, the per-cell Escape handlers restore the original value and
  // cancel the edit — clearing selection here would preempt that logic.
  if state.edit_cell.is_none() {
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
      state.clear_selection();
    }
  }
  if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
    if let Some(sel) = &state.selection {
      if let Some(&row) = sel.rows.first() {
        state.edit_cell = Some((sel.column, row));
      }
    }
  }
  if state.edit_cell.is_none() && state.selection.is_some() {
    let is_ctrl_del = ui.input_mut(|input| {
      input.consume_key(egui::Modifiers::COMMAND, egui::Key::Delete)
        || input.consume_key(egui::Modifiers::CTRL, egui::Key::Delete)
    });
    if is_ctrl_del {
      if let Some(sel) = &state.selection {
        result.force_delete_rows = Some(sel.rows.clone());
      }
    } else {
      let is_del = ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
      if is_del {
        if let Some(sel) = &state.selection {
          result.delete_rows = Some(sel.rows.clone());
        }
      }
    }
  }

  // ── PENDING KEYBOARD EVENTS COMMITMENTS ──────────────────────────────────
  let cached_members = member_candidates.to_vec();
  let cached_categories = category_candidates.to_vec();
  let cached_vendors = vendor_candidates.to_vec();
  let cached_descriptions = description_candidates.to_vec();
  let candidates_fn = move |col| match col {
    GridColumn::Member => cached_members.clone(),
    GridColumn::Category => cached_categories.clone(),
    GridColumn::Vendor => cached_vendors.clone(),
    GridColumn::Description => cached_descriptions.clone(),
    _ => Vec::new(),
  };

  let indices_to_use = sorted_indices;

  if let Some((column, _, targets)) = state.apply_pending_keyboard(
    rows,
    indices_to_use,
    *autocomplete_selection,
    candidates_fn,
  ) {
    match column {
      GridColumn::Date => result.pending_field_updates.extend(targets.into_iter().map(|t| (t, "date"))),
      GridColumn::Amount => result.pending_field_updates.extend(targets.into_iter().map(|t| (t, "amount"))),
      GridColumn::Member => result.pending_member_commits.extend(targets),
      GridColumn::Category => result.pending_category_commits.extend(targets),
      GridColumn::Vendor => result.pending_field_updates.extend(targets.into_iter().map(|t| (t, "vendor"))),
      GridColumn::Description => result.pending_field_updates.extend(targets.into_iter().map(|t| (t, "description"))),
    }
  }

  let selection = &mut state.selection;
  let drag = &mut state.drag;
  let edit_cell = &mut state.edit_cell;
  let edit_original = &mut state.edit_original;
  let typeahead = &mut state.typeahead;
  let scroll_offset = &mut state.scroll_offset;

  // Apply any pending typeahead character before rendering so the first frame
  // shows the typed character in the correct cell.
  if let Some((tc, ti)) = edit_cell.as_ref().copied().zip(typeahead.take()) {
    let Some(row) = rows.get_mut(tc.1) else { *typeahead = None; return result; };
    match tc.0 {
      GridColumn::Date        => { row.row_date_mut().clear();           row.row_date_mut().push(ti); }
      GridColumn::Amount      => { row.row_amount_input_mut().clear();   row.row_amount_input_mut().push(ti); }
      GridColumn::Member      => { row.row_member_mut().clear();         row.row_member_mut().push(ti); }
      GridColumn::Category    => { row.row_category_mut().clear();       row.row_category_mut().push(ti); }
      GridColumn::Vendor      => { row.row_vendor_mut().clear();         row.row_vendor_mut().push(ti); }
      GridColumn::Description => { row.row_description_mut().clear();   row.row_description_mut().push(ti); }
    }
  } else {
    *typeahead = None; // discard if no edit cell
  }

  let category_menu_w  = category_picker_menu_width(ui, categories, 200.0);
  let member_menu_w    = MEMBER_PICKER_MIN_WIDTH;
  let member_count     = members.len();
  let member_scroll    = if member_count > 10 { Some(member_picker_max_height(member_count)) } else { None };

  let mut row_drag_bands: Vec<(usize, f32, f32)> = Vec::new();
  let mut scroll_y = *scroll_offset;

  // Helper closures to make cell egui::Id stable based on visual row positions
  let cell_id_by_visual = |col: GridColumn, visual_row: usize| -> egui::Id {
    egui::Id::new((cell_id_prefix, format!("{col:?}"), visual_row))
  };
  let get_cell_id_for_raw_row = |col: GridColumn, raw_idx: usize| -> Option<egui::Id> {
    sorted_indices.iter().position(|&i| i == raw_idx)
      .map(|vis_row| cell_id_by_visual(col, vis_row))
  };
  let cell_id = |col: GridColumn, idx: usize| -> egui::Id {
    get_cell_id_for_raw_row(col, idx).unwrap_or_else(|| cell_id_by_visual(col, idx))
  };
  let cell_has_focus = |ctx: &egui::Context, col: GridColumn, raw_idx: usize| -> bool {
    get_cell_id_for_raw_row(col, raw_idx)
      .is_some_and(|id| ctx.memory(|m| m.has_focus(id)))
  };

  let w = ui.available_width();
  let fractions = [0.12, 0.10, 0.10, 0.18, 0.15, 0.35];
  let labels: [(GridColumn, &str); 6] = [
    (GridColumn::Date, "Date"),
    (GridColumn::Amount, "Amount"),
    (GridColumn::Member, "Member"),
    (GridColumn::Category, "Category"),
    (GridColumn::Vendor, "Vendor"),
    (GridColumn::Description, "Description"),
  ];
  let sort_cols: [ExpenseSortColumn; 6] = [
    ExpenseSortColumn::Date, ExpenseSortColumn::Amount,
    ExpenseSortColumn::Member, ExpenseSortColumn::Category,
    ExpenseSortColumn::Vendor, ExpenseSortColumn::Description,
  ];

  // Frozen header row
  ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
  ui.horizontal(|ui| {
    ui.spacing_mut().item_spacing.x = 0.0;
    for (i, (_, label)) in labels.iter().enumerate() {
      ui.allocate_ui_with_layout(
        egui::vec2(w * fractions[i], GRID_HEADER_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
          if grid_header(ui, label).clicked() {
            result.clicked_sort_column = Some(sort_cols[i]);
          }
        },
      );
    }
  });

  // Scrollable body
  #[allow(deprecated)]
  let scroll_response = egui::ScrollArea::vertical()
    .id_salt(format!("{cell_id_prefix}_scroll"))
    .auto_shrink([false, false])
    .drag_to_scroll(false)
    .scroll_offset(egui::vec2(0.0, scroll_y))
    .show(ui, |ui| {
      ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
      let mut builder = TableBuilder::new(ui)
        .striped(true)
        .vscroll(false)
        .min_scrolled_height(120.0)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
      for &f in &fractions {
        builder = builder.column(egui_extras::Column::initial(w * f).clip(true));
      }

      builder
        .body(|mut body| {
          for (visual_row_index, &idx) in sorted_indices.iter().enumerate() {
            body.row(GRID_ROW_HEIGHT, |mut row_ui| {

              // ── DATE ────────────────────────────────────────────────────
              row_ui.col(|ui| {
                let column = GridColumn::Date;
                let cell_rect = ui.max_rect();
                if show_row_separator {
                  ui.painter().line_segment(
                    [cell_rect.left_bottom(), egui::pos2(cell_rect.right(), cell_rect.bottom())],
                    egui::Stroke::new(1.5, ui.visuals().widgets.noninteractive.bg_stroke.color),
                  );
                }
                row_drag_bands.push((idx, cell_rect.top(), cell_rect.bottom()));
                process_grid_column_cell(
                  ui,
                  column,
                  idx,
                  cell_id_by_visual(column, visual_row_index).with("bg"),
                  selection,
                  drag,
                  edit_cell,
                );
                let _sel = grid_row_selected(selection, column, idx);
                ui.horizontal(|ui| {
                  let editing = grid_cell_editing(edit_cell, column, idx);
                  let (_resp, date_changed) = ui_grid_date_edit(
                    ui,
                    rows[idx].row_date_mut(),
                    edit_original,
                    edit_cell,
                    cell_id_by_visual(column, visual_row_index),
                    editing,
                  );
                  let press_enter = editing && ui.input(|i| i.key_pressed(egui::Key::Enter));
                  if date_changed || press_enter {
                    result.pending_field_updates.push((idx, "date"));
                    if press_enter {
                      *edit_cell = None;
                      *edit_original = None;
                    }
                    // propagate to selected rows
                    let targets = grid_commit_targets(selection, column, idx);
                    let value = rows[idx].row_date().to_string();
                    for &t in &targets {
                      if t != idx {
                        if let Some(r) = rows.get_mut(t) { *r.row_date_mut() = value.clone(); }
                      }
                      result.pending_field_updates.push((t, "date"));
                    }
                  }
                  if editing && cell_has_focus(ui.ctx(), column, idx) {
                    result.active_cell = Some((column, idx));
                  }
                });
              });

              // ── AMOUNT ──────────────────────────────────────────────────
              row_ui.col(|ui| {
                let column = GridColumn::Amount;
                process_grid_column_cell(
                  ui,
                  column,
                  idx,
                  cell_id_by_visual(column, visual_row_index).with("bg"),
                  selection,
                  drag,
                  edit_cell,
                );
                let _sel = grid_row_selected(selection, column, idx);
                ui.horizontal(|ui| {
                  let editing = grid_cell_editing(edit_cell, column, idx);
                  if editing {
                    let amount_resp = ui_grid_text_edit(
                      ui,
                      rows[idx].row_amount_input_mut(),
                      edit_original,
                      edit_cell,
                      cell_id_by_visual(column, visual_row_index),
                      true,
                    );
                    if amount_resp.changed() {
                      // Strip non-numeric chars then collapse extra decimals
                      rows[idx].row_amount_input_mut().retain(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '-');
                      let s = rows[idx].row_amount_input().to_string();
                      let mut seen_dot = false;
                      *rows[idx].row_amount_input_mut() = s.chars().filter(|&c| {
                        if c == '.' { if seen_dot { return false; } seen_dot = true; }
                        true
                      }).collect();
                      result.pending_field_updates.push((idx, "amount"));
                    }
                    if editing && cell_has_focus(ui.ctx(), column, idx) {
                      result.active_cell = Some((column, idx));
                    }
                    if grid_text_field_committed(ui, &amount_resp, egui::Rect::NOTHING) {
                      let orig = edit_original.take().unwrap_or_default();
                      if let Some(cents) = parse_amount_cents(rows[idx].row_amount_input()) {
                        let formatted = money(cents).replace('$', "");
                        *rows[idx].row_amount_input_mut() = formatted.clone();
                        // propagate to selected rows
                        let targets = grid_commit_targets(selection, column, idx);
                        for &t in &targets {
                          if t != idx {
                            if let Some(r) = rows.get_mut(t) {
                              *r.row_amount_input_mut() = formatted.clone();
                            }
                          }
                          result.pending_field_updates.push((t, "amount"));
                        }
                      } else {
                        *rows[idx].row_amount_input_mut() = orig;
                      }
                      *edit_cell = None;
                    }
                  } else {
                    let display = if let Some(cents) = parse_amount_cents(rows[idx].row_amount_input()) {
                      money(cents)
                    } else {
                      rows[idx].row_amount_input().to_string()
                    };
                    let mut temp = display;
                    ui_grid_text_edit(ui, &mut temp, edit_original, edit_cell, cell_id_by_visual(column, visual_row_index), false);
                  }
                });
              });

              // ── MEMBER ──────────────────────────────────────────────────
              row_ui.col(|ui| {
                let column = GridColumn::Member;
                let member_bg = member_color_for(members, rows[idx].row_member());
                if member_bg.a() > 0 && member_bg != Color32::TRANSPARENT {
                  let muted = Color32::from_rgba_unmultiplied(member_bg.r(), member_bg.g(), member_bg.b(), 60);
                  ui.painter().rect_filled(ui.max_rect(), 0.0, muted);
                }
                process_grid_column_cell(
                  ui,
                  column,
                  idx,
                  cell_id_by_visual(column, visual_row_index).with("bg"),
                  selection,
                  drag,
                  edit_cell,
                );
                let _sel = grid_row_selected(selection, column, idx);
                ui.horizontal(|ui| {
                  ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                  let editing = grid_cell_editing(edit_cell, column, idx);
                  if editing && edit_original.is_none() {
                    *edit_original = Some(rows[idx].row_member().to_string());
                  }
                  let member_bg = member_color_for(members, rows[idx].row_member());
                  let cell = member_cell_ui(ui, rows[idx].row_member_mut(), member_bg, cell_id_by_visual(column, visual_row_index), editing);
                  if editing && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                    if let Some(orig) = edit_original.take() { *rows[idx].row_member_mut() = orig; }
                    *edit_cell = None;
                    cell.text.surrender_focus();
                  }
                  if editing && cell_has_focus(ui.ctx(), column, idx) {
                    result.active_cell = Some((column, idx));
                  }
                  if editing {
                    if let Some(picked) = show_cell_autocomplete_popup(ui, &cell.text, rows[idx].row_member(), member_candidates, autocomplete_selection) {
                      let targets = grid_commit_targets(selection, column, idx);
                      for &t in &targets {
                        if let Some(r) = rows.get_mut(t) { *r.row_member_mut() = picked.clone(); }
                      }
                      result.pending_member_commits.extend(targets);
                      *edit_cell = None;
                      *edit_original = None;
                    }
                  }
                  // Chevron picker
                  let mut member_pick = false;
                  let mut picked_member_value: Option<String> = None;
                  let current_member = rows[idx].row_member().trim().to_string();
                  grid_chevron_picker_popup(ui, &cell.chevron, member_menu_w, member_scroll, |ui| {
                    if member_none_row_selectable(ui, current_member.is_empty()) {
                      picked_member_value = Some(String::new()); member_pick = true; ui.close();
                    }
                    ui.separator();
                    for member in members {
                      if member_row_selectable(ui, member, member.name.eq_ignore_ascii_case(&current_member)) {
                        picked_member_value = Some(member.name.clone()); member_pick = true; ui.close();
                      }
                    }
                  });
                  if member_pick {
                    let value = picked_member_value.unwrap_or_default();
                    let targets = grid_commit_targets(selection, column, idx);
                    for &t in &targets {
                      if let Some(r) = rows.get_mut(t) { *r.row_member_mut() = value.clone(); }
                    }
                    result.pending_member_commits.extend(targets);
                    *edit_cell = None;
                    *edit_original = None;
                  } else if editing && grid_text_field_committed(ui, &cell.text, egui::Rect::NOTHING) {
                    let targets = grid_commit_targets(selection, column, idx);
                    let value = rows[idx].row_member().to_string();
                    for &t in &targets {
                      if t != idx {
                        if let Some(r) = rows.get_mut(t) { *r.row_member_mut() = value.clone(); }
                      }
                    }
                    result.pending_member_commits.extend(targets);
                    *edit_cell = None;
                    *edit_original = None;
                  }
                });
              });

              // ── CATEGORY ────────────────────────────────────────────────
              row_ui.col(|ui| {
                let column = GridColumn::Category;
                let cat_bg = category_color_for(categories, rows[idx].row_category());
                if cat_bg.a() > 0 && cat_bg != Color32::TRANSPARENT {
                  let muted = Color32::from_rgba_unmultiplied(cat_bg.r(), cat_bg.g(), cat_bg.b(), 60);
                  ui.painter().rect_filled(ui.max_rect(), 0.0, muted);
                }
                process_grid_column_cell(
                  ui,
                  column,
                  idx,
                  cell_id_by_visual(column, visual_row_index).with("bg"),
                  selection,
                  drag,
                  edit_cell,
                );
                let _sel = grid_row_selected(selection, column, idx);
                ui.horizontal(|ui| {
                  ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                  let editing = grid_cell_editing(edit_cell, column, idx);
                  if editing && edit_original.is_none() {
                    *edit_original = Some(rows[idx].row_category().to_string());
                  }
                  let cat_bg = category_color_for(categories, rows[idx].row_category());
                  let cell = category_cell_ui(ui, rows[idx].row_category_mut(), cat_bg, cell_id_by_visual(column, visual_row_index), editing);
                  if editing && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                    if let Some(orig) = edit_original.take() { *rows[idx].row_category_mut() = orig; }
                    *edit_cell = None;
                    cell.text.surrender_focus();
                  }
                  if editing && cell_has_focus(ui.ctx(), column, idx) {
                    result.active_cell = Some((column, idx));
                  }
                  // Typeahead autocomplete
                  if editing {
                    if let Some(picked) = show_cell_autocomplete_popup(ui, &cell.text, rows[idx].row_category(), category_candidates, autocomplete_selection) {
                      let targets = grid_commit_targets(selection, column, idx);
                      for &t in &targets {
                        if let Some(r) = rows.get_mut(t) { *r.row_category_mut() = picked.clone(); }
                      }
                      result.pending_category_commits.extend(targets);
                      *edit_cell = None;
                      *edit_original = None;
                    }
                  }
                  // Context menu: Edit color
                  let category_name = rows[idx].row_category().trim().to_string();
                  cell.text.context_menu(|ui| {
                    if category_name.is_empty() {
                      ui.label(RichText::new("Pick a category from the list").small().weak());
                      return;
                    }
                    if let Some(cat) = find_category_by_label(categories, &category_name) {
                      if ui.button("Edit color\u{2026}").clicked() {
                        result.open_color_popup = Some((cat.id, category_name.clone(), cat_bg));
                        ui.close();
                      }
                    }
                  });
                  // Chevron picker
                  let mut combobox_pick = false;
                  let mut picked_category_value: Option<String> = None;
                  grid_chevron_picker_popup(ui, &cell.chevron, category_menu_w, Some(280.0), |ui| {
                    if let Some(label) = category_picker_menu_ui(ui, categories, rows[idx].row_category()) {
                      picked_category_value = Some(label); combobox_pick = true;
                    }
                  });
                  if combobox_pick {
                    let mut value = picked_category_value.unwrap_or_default();
                    if let Some(matched) = find_category_by_label(categories, &value) {
                      let parents = category_parent_map(categories);
                      value = matched.full_label(&parents);
                    }
                    let targets = grid_commit_targets(selection, column, idx);
                    for &t in &targets {
                      if let Some(r) = rows.get_mut(t) { *r.row_category_mut() = value.clone(); }
                    }
                    result.pending_category_commits.extend(targets);
                    *edit_cell = None;
                    *edit_original = None;
                  } else if editing && grid_text_field_committed(ui, &cell.text, egui::Rect::NOTHING) {
                    let targets = grid_commit_targets(selection, column, idx);
                    let value = rows[idx].row_category().to_string();
                    for &t in &targets {
                      if t != idx {
                        if let Some(r) = rows.get_mut(t) { *r.row_category_mut() = value.clone(); }
                      }
                    }
                    result.pending_category_commits.extend(targets);
                    *edit_cell = None;
                    *edit_original = None;
                  }
                });
              });

              // ── VENDOR ──────────────────────────────────────────────────
              row_ui.col(|ui| {
                let column = GridColumn::Vendor;
                process_grid_column_cell(
                  ui,
                  column,
                  idx,
                  cell_id_by_visual(column, visual_row_index).with("bg"),
                  selection,
                  drag,
                  edit_cell,
                );
                let _sel = grid_row_selected(selection, column, idx);
                ui.horizontal(|ui| {
                  let editing = grid_cell_editing(edit_cell, column, idx);
                  let response = ui_grid_text_edit(
                    ui, rows[idx].row_vendor_mut(), edit_original, edit_cell,
                    cell_id_by_visual(column, visual_row_index), editing,
                  );
                  if editing && cell_has_focus(ui.ctx(), column, idx) {
                    result.active_cell = Some((column, idx));
                  }
                  if editing {
                    if let Some(picked) = show_cell_autocomplete_popup(ui, &response, rows[idx].row_vendor(), vendor_candidates, autocomplete_selection) {
                      let targets = grid_commit_targets(selection, column, idx);
                      for &t in &targets {
                        if let Some(r) = rows.get_mut(t) { *r.row_vendor_mut() = picked.clone(); }
                      }
                      result.pending_field_updates.extend(targets.iter().map(|&t| (t, "vendor")));
                      *edit_cell = None;
                      *edit_original = None;
                    }
                  }
                  if response.changed() {
                    result.pending_field_updates.push((idx, "vendor"));
                  }
                  if editing && grid_text_field_committed(ui, &response, egui::Rect::NOTHING) {
                    let targets = grid_commit_targets(selection, column, idx);
                    let value = rows[idx].row_vendor().to_string();
                    for &t in &targets {
                      if t != idx {
                        if let Some(r) = rows.get_mut(t) { *r.row_vendor_mut() = value.clone(); }
                      }
                      result.pending_field_updates.push((t, "vendor"));
                    }
                    *edit_cell = None;
                    *edit_original = None;
                  }
                });
              });

              // ── DESCRIPTION ─────────────────────────────────────────────
              row_ui.col(|ui| {
                let column = GridColumn::Description;
                process_grid_column_cell(
                  ui,
                  column,
                  idx,
                  cell_id_by_visual(column, visual_row_index).with("bg"),
                  selection,
                  drag,
                  edit_cell,
                );
                let _sel = grid_row_selected(selection, column, idx);
                ui.horizontal(|ui| {
                  let editing = grid_cell_editing(edit_cell, column, idx);
                  let response = ui_grid_text_edit(
                    ui, rows[idx].row_description_mut(), edit_original, edit_cell,
                    cell_id_by_visual(column, visual_row_index), editing,
                  );
                  if editing && cell_has_focus(ui.ctx(), column, idx) {
                    result.active_cell = Some((column, idx));
                  }
                  if editing {
                    if let Some(picked) = show_cell_autocomplete_popup(ui, &response, rows[idx].row_description(), description_candidates, autocomplete_selection) {
                      let targets = grid_commit_targets(selection, column, idx);
                      for &t in &targets {
                        if let Some(r) = rows.get_mut(t) { *r.row_description_mut() = picked.clone(); }
                      }
                      result.pending_field_updates.extend(targets.iter().map(|&t| (t, "description")));
                      *edit_cell = None;
                      *edit_original = None;
                    }
                  }
                  if response.changed() {
                    result.pending_field_updates.push((idx, "description"));
                  }
                  if editing && grid_text_field_committed(ui, &response, egui::Rect::NOTHING) {
                    let targets = grid_commit_targets(selection, column, idx);
                    let value = rows[idx].row_description().to_string();
                    for &t in &targets {
                      if t != idx {
                        if let Some(r) = rows.get_mut(t) { *r.row_description_mut() = value.clone(); }
                      }
                      result.pending_field_updates.push((t, "description"));
                    }
                    *edit_cell = None;
                    *edit_original = None;
                  }
                });
              });

            }); // body row
          } // for idx
        }); // body
    }); // TableBuilder

  scroll_y = scroll_response.state.offset.y;
  grid_apply_shift_hand_pan(ui.ctx(), &mut scroll_y);
  *scroll_offset = scroll_y;

  // Type-to-start-edit.
  // We inline the focus check here instead of using grid_try_start_edit_from_typing's
  // broad `mem.focused().is_some()` guard — that guard blocks typeahead whenever ANY
  // widget has focus (including the Window close button), breaking the import grid.
  // Instead we only block typeahead if one of OUR grid TextEdit cells has focus.
  if edit_cell.is_none() {
    let grid_cell_has_focus = selection.as_ref().is_some_and(|sel| {
      sel.rows.iter().any(|&r| {
        ui.ctx().memory(|m| m.has_focus(cell_id(sel.column, r)))
      })
    });
    if !grid_cell_has_focus {
      let _ = grid_try_start_edit_from_typing(ui, selection, edit_cell, typeahead);
    }
  }

  // Enter key: if multi-row selection, commit the active cell to all rows
  if let Some((column, idx)) = *edit_cell {
    if ui.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
      let multi = selection.as_ref().is_some_and(|s| s.column == column && s.rows.len() > 1);
      let ac_active = !autocomplete_suggestions_list(
        match column {
          GridColumn::Member      => rows.get(idx).map(|r| r.row_member()).unwrap_or(""),
          GridColumn::Category    => rows.get(idx).map(|r| r.row_category()).unwrap_or(""),
          GridColumn::Vendor      => rows.get(idx).map(|r| r.row_vendor()).unwrap_or(""),
          GridColumn::Description => rows.get(idx).map(|r| r.row_description()).unwrap_or(""),
          _ => "",
        },
        match column {
          GridColumn::Member      => member_candidates,
          GridColumn::Category    => category_candidates,
          GridColumn::Vendor      => vendor_candidates,
          GridColumn::Description => description_candidates,
          _ => &[],
        },
      ).is_empty();
      if multi && !ac_active {
        // Commit current value to all selected rows
        let targets = grid_commit_targets(selection, column, idx);
        match column {
          GridColumn::Date => {
            let value = rows.get(idx).map(|r| r.row_date().to_string()).unwrap_or_default();
            for &t in &targets { if let Some(r) = rows.get_mut(t) { *r.row_date_mut() = value.clone(); } result.pending_field_updates.push((t, "date")); }
          }
          GridColumn::Amount => {
            let value = rows.get(idx).map(|r| r.row_amount_input().to_string()).unwrap_or_default();
            for &t in &targets { if let Some(r) = rows.get_mut(t) { *r.row_amount_input_mut() = value.clone(); } result.pending_field_updates.push((t, "amount")); }
          }
          GridColumn::Member => {
            let value = rows.get(idx).map(|r| r.row_member().to_string()).unwrap_or_default();
            for &t in &targets { if let Some(r) = rows.get_mut(t) { *r.row_member_mut() = value.clone(); } }
            result.pending_member_commits.extend(targets);
          }
          GridColumn::Category => {
            let value = rows.get(idx).map(|r| r.row_category().to_string()).unwrap_or_default();
            for &t in &targets { if let Some(r) = rows.get_mut(t) { *r.row_category_mut() = value.clone(); } }
            result.pending_category_commits.extend(targets);
          }
          GridColumn::Vendor => {
            let value = rows.get(idx).map(|r| r.row_vendor().to_string()).unwrap_or_default();
            for &t in &targets { if let Some(r) = rows.get_mut(t) { *r.row_vendor_mut() = value.clone(); } result.pending_field_updates.push((t, "vendor")); }
          }
          GridColumn::Description => {
            let value = rows.get(idx).map(|r| r.row_description().to_string()).unwrap_or_default();
            for &t in &targets { if let Some(r) = rows.get_mut(t) { *r.row_description_mut() = value.clone(); } result.pending_field_updates.push((t, "description")); }
          }
        }
      }
    }
  }

  // Drag
  grid_snap_drag_to_pointer_y(ui, &row_drag_bands, sorted_indices, drag, selection);
  if drag.is_some() { ui.ctx().request_repaint(); }
  finish_grid_drag(drag, ui);

  // Handle double-click edit activation FIRST so that the focus request below
  // fires in the same frame the edit_cell is set (instead of one frame late).
  let pending_edit = ui.ctx().data_mut(|d| {
    d.remove_temp::<Option<(GridColumn, usize)>>(egui::Id::new("pending_edit_cell"))
  });
  if let Some(Some(cell)) = pending_edit {
    *edit_cell = Some(cell);
    ui.ctx().request_repaint();
  }

  // Focus cell — runs after pending_edit_cell is resolved so the request_focus
  // call covers double-click activations in the same frame.
  if let Some(target) = state.pending_focus_target.take() {
    *selection = Some(GridSelection {
      column: target.0,
      rows: vec![target.1],
    });
    *edit_cell = Some(target);
    if let Some(id) = get_cell_id_for_raw_row(target.0, target.1) {
      request_grid_cell_focus(ui, id);
    }
  } else if let Some((col, idx)) = *edit_cell {
    if let Some(id) = get_cell_id_for_raw_row(col, idx) {
      let chevron_id = id.with("chevron");
      ui.memory_mut(|mem| { mem.surrender_focus(chevron_id); mem.request_focus(id); });
    }
  }

  result
}

// ---------------------------------------------------------------------------
// Helper: collect the target rows for a multi-row commit
// ---------------------------------------------------------------------------
// (grid_commit_targets is already in widgets.rs — we re-export nothing, just use it)
