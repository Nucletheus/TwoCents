use eframe::egui::{self, Color32, Id, Shape, pos2, RichText, color_picker::show_color_at};

use crate::models::*;
use crate::db::*;

pub const GRID_HEADER_HEIGHT: f32 = 24.0;
pub const GRID_ROW_HEIGHT: f32 = 24.0;
pub const GRID_SELECTION_STATUS_HEIGHT: f32 = 18.0;

pub const PICKER_ROW_HEIGHT: f32 = 24.0;
pub const CATEGORY_PICKER_MIN_WIDTH: f32 = 200.0;
pub const CATEGORY_PICKER_MAX_WIDTH: f32 = 320.0;
pub const MEMBER_PICKER_MIN_WIDTH: f32 = 140.0;

pub fn grid_header(ui: &mut egui::Ui, label: &str) -> egui::Response {
  let width = ui.available_width().max(1.0);
  let (rect, response) = ui.allocate_exact_size(egui::vec2(width, GRID_HEADER_HEIGHT), egui::Sense::click());

  // ponytail: header sits INSIDE grid_table_frame (1px stroke, 6px radius,
  // 2px inner margin). The frame already owns the top, left, and right
  // edges — drawing our own top border put a 1px line directly under the
  // frame's 1px stroke, producing a visible 2px doubled line. Drop the
  // top border; the frame provides it. Keep the rounded top corners so
  // the bg_subtle fill matches the frame's radius. Add a 1px bottom
  // border so the header reads as a sub-band separated from the data
  // rows below.
  let fill_color = crate::ui::theme::bg_secondary();
  let border_color = crate::ui::theme::border();
  let radius = crate::ui::theme_tokens::RADIUS_MD as u8;
  ui.painter().rect_filled(
    rect,
    egui::CornerRadius { nw: radius, ne: radius, sw: 0, se: 0 },
    fill_color,
  );
  // Bottom edge only — 1px line separating the header from the data rows.
  ui.painter().line_segment(
    [egui::pos2(rect.left(), rect.bottom()), egui::pos2(rect.right(), rect.bottom())],
    egui::Stroke::new(1.0_f32, border_color),
  );

  let text_color = crate::ui::theme::fg_primary();

  ui.painter().text(
    egui::pos2(rect.left() + crate::ui::theme_tokens::SPACE_2, rect.center().y),
    egui::Align2::LEFT_CENTER,
    label,
    egui::FontId::proportional(13.0),
    text_color,
  );
  response
}


pub fn grid_snap_drag_to_pointer_y(
  ui: &egui::Ui,
  row_bands: &[(usize, f32, f32)],
  sorted_indices: &[usize],
  drag: &mut Option<GridSelectDrag>,
  selection: &mut Option<GridSelection>,
) {
  let Some(active_drag) = drag.as_mut() else {
    return;
  };
  if !ui.input(|input| input.pointer.any_down()) {
    *drag = None;
    return;
  }
  let Some(pointer_y) = ui.input(|input| input.pointer.hover_pos()).map(|pos| pos.y) else {
    return;
  };
  if row_bands.is_empty() {
    return;
  }
  let table_top = row_bands[0].1;
  let table_bottom = row_bands[row_bands.len() - 1].2;
  if pointer_y < table_top - GRID_ROW_HEIGHT || pointer_y > table_bottom + GRID_ROW_HEIGHT {
    return;
  }

  let mut target_row = active_drag.end;
  let mut best_distance = f32::INFINITY;
  for &(row, top, bottom) in row_bands {
    if pointer_y >= top && pointer_y <= bottom {
      target_row = row;
      break;
    }
    let row_center = (top + bottom) * 0.5;
    let distance = (pointer_y - row_center).abs();
    if distance < best_distance {
      best_distance = distance;
      target_row = row;
    }
  }

  active_drag.end = target_row;
  *selection = Some(GridSelection {
    column: active_drag.column,
    rows: selection_range(sorted_indices, active_drag.anchor, target_row),
  });
}

pub fn selection_range(sorted_indices: &[usize], anchor: usize, end: usize) -> Vec<usize> {
  let anchor_pos = sorted_indices.iter().position(|&idx| idx == anchor);
  let end_pos = sorted_indices.iter().position(|&idx| idx == end);
  match (anchor_pos, end_pos) {
    (Some(anchor_pos), Some(end_pos)) => {
      let (lo, hi) = if anchor_pos <= end_pos {
        (anchor_pos, end_pos)
      } else {
        (end_pos, anchor_pos)
      };
      // ponytail: rows must be ascending raw indices — every membership
      // check below binary_searches them, and the visual-order slice is
      // scrambled once the grid is sorted by any non-identity column.
      let mut rows = sorted_indices[lo..=hi].to_vec();
      rows.sort_unstable();
      rows
    }
    _ => vec![anchor],
  }
}

pub fn grid_commit_targets(
  selection: &Option<GridSelection>,
  column: GridColumn,
  active_row: usize,
) -> Vec<usize> {
  if let Some(sel) = selection {
    if sel.column == column && sel.rows.len() > 1 && sel.rows.binary_search(&active_row).is_ok() {
      return sel.rows.clone();
    }
  }
  vec![active_row]
}


pub fn grid_text_field_committed(ui: &egui::Ui, response: &egui::Response, blur_block_rect: egui::Rect) -> bool {
  if !response.lost_focus() {
    return false;
  }
  if blur_block_rect != egui::Rect::NOTHING {
    if ui
      .ctx()
      .pointer_hover_pos()
      .is_some_and(|pos| blur_block_rect.contains(pos))
    {
      return false;
    }
  }
  true
}

pub fn grid_row_selected(selection: &Option<GridSelection>, column: GridColumn, row: usize) -> bool {
  // ponytail: binary_search — valid because selection_range() sorts rows
  // ascending; contains() was O(k) per cell per frame during drag-select.
  selection
    .as_ref()
    .is_some_and(|sel| sel.column == column && sel.rows.binary_search(&row).is_ok())
}

pub fn grid_row_has_any_selection(selection: &Option<GridSelection>, row: usize) -> bool {
  selection
    .as_ref()
    .is_some_and(|sel| sel.rows.binary_search(&row).is_ok())
}

pub fn grid_selection_anchor(selection: &Option<GridSelection>) -> Option<(GridColumn, usize)> {
  selection.as_ref().and_then(|sel| sel.rows.first().copied().map(|row| (sel.column, row)))
}

pub fn grid_cell_editing(edit_cell: &Option<(GridColumn, usize)>, column: GridColumn, row: usize) -> bool {
  edit_cell.as_ref().is_some_and(|&(col, idx)| col == column && idx == row)
}

pub fn grid_consume_printable_char(ui: &egui::Ui) -> Option<char> {
  let mut ch = None;
  ui.input_mut(|input| {
    for event in &input.events {
      if let egui::Event::Text(text) = event {
        if let Some(c) = text.chars().next() {
          if !c.is_control() {
            ch = Some(c);
          }
        }
      }
    }
    if ch.is_some() {
      input.events.retain(|event| !matches!(event, egui::Event::Text(_)));
    }
  });
  ch
}

pub fn grid_try_start_edit_from_typing(
  ui: &egui::Ui,
  selection: &Option<GridSelection>,
  edit_cell: &mut Option<(GridColumn, usize)>,
  typeahead: &mut Option<char>,
) -> bool {
  if edit_cell.is_some() {
    return false;
  }
  // Check selection BEFORE consuming the character. If we consumed first and then
  // found no selection, the character would be silently dropped — a particular
  // problem when a second grid (e.g. the expense grid) renders before this one and
  // has no selection: it would eat every keystroke meant for the active grid.
  let Some((column, row)) = grid_selection_anchor(selection) else {
    return false;
  };
  let Some(ch) = grid_consume_printable_char(ui) else {
    return false;
  };
  *typeahead = Some(ch);
  *edit_cell = Some((column, row));
  true
}

pub fn finish_grid_drag(drag: &mut Option<GridSelectDrag>, ui: &egui::Ui) {
  if drag.is_some() && !ui.input(|input| input.pointer.any_down()) {
    ui.ctx().stop_dragging();
    *drag = None;
  }
}

pub fn paint_grid_cell_highlight(
  ui: &mut egui::Ui,
  cell_rect: egui::Rect,
  selected: bool,
  row_has_selection: bool,
  dragging: bool,
  editing: bool,
) {
  if !ui.is_rect_visible(cell_rect) {
    return;
  }
  // ponytail: route cell-paint colors through the components palette so
  // every cell reacts to the same theme changes as the rest of the UI.
  let sel = crate::ui::theme::accent();
  if dragging && selected {
    ui.painter().rect_filled(cell_rect, 0.0, Color32::from_rgba_unmultiplied(sel.r(), sel.g(), sel.b(), 40));
  } else if selected {
    ui.painter().rect_filled(cell_rect, 0.0, Color32::from_rgba_unmultiplied(sel.r(), sel.g(), sel.b(), 15));
  } else if row_has_selection {
    // Subtle "grayed out" highlight for non-selected columns in a selected row
    let ghost = crate::ui::theme::bg_hover().linear_multiply(0.5);
    ui.painter().rect_filled(cell_rect, 0.0, ghost);
  }
  if editing {
    ui.painter().rect_stroke(
      cell_rect.expand(1.0),
      0.0,
      egui::Stroke::new(2.0_f32, sel),
      egui::StrokeKind::Outside,
    );
  } else if selected {
    ui.painter().rect_stroke(
      cell_rect,
      0.0,
      egui::Stroke::new(1.0_f32, crate::ui::theme::border()),
      egui::StrokeKind::Inside,
    );
  }
}

pub fn process_grid_column_cell(
  ui: &mut egui::Ui,
  column: GridColumn,
  row: usize,
  cell_id: egui::Id,
  selection: &mut Option<GridSelection>,
  drag: &mut Option<GridSelectDrag>,
  edit_cell: &mut Option<(GridColumn, usize)>,
) {
  let cell_rect = ui.max_rect();
  let selected = grid_row_selected(selection, column, row);
  let row_has_selection = grid_row_has_any_selection(selection, row);
  let dragging = drag.as_ref().is_some_and(|active| active.column == column);
  let editing = grid_cell_editing(edit_cell, column, row);
  paint_grid_cell_highlight(ui, cell_rect, selected, row_has_selection, dragging, editing);

  let response = ui.interact(cell_rect, cell_id, egui::Sense::click_and_drag());

  if response.clicked() {
    let already_selected = selection.as_ref().map_or(false, |s| s.column == column && s.rows.binary_search(&row).is_ok());
    if !already_selected {
      *selection = Some(GridSelection {
        column,
        rows: vec![row],
      });
    }
    *edit_cell = None;
    // Surrender focus from any other widget (e.g. Window buttons, bulk_category
    // field) so keyboard input routes to the grid immediately.
    surrender_focused_widget(ui.ctx());
  }

  if response.double_clicked() {
    let already_selected = selection.as_ref().map_or(false, |s| s.column == column && s.rows.binary_search(&row).is_ok());
    if !already_selected {
      *selection = Some(GridSelection {
        column,
        rows: vec![row],
      });
    }
    ui.ctx().data_mut(|d| {
      d.insert_temp(egui::Id::new("pending_edit_cell"), Some((column, row)));
    });
    ui.ctx().request_repaint();
  }

  // ponytail: start a drag only when none is active — a second cell firing
  // drag_started mid-gesture re-anchored the drag (anchor/end reset,
  // selection collapse, "skipping"). ender removed: egui can reassign the
  // dragged id mid-gesture, firing spurious drag_stopped that killed the
  // drag; finish_grid_drag already ends it on true button release.
  if response.drag_started() && drag.is_none() {
    surrender_focused_widget(ui.ctx());
    *edit_cell = None;
    *drag = Some(GridSelectDrag {
      column,
      anchor: row,
      end: row,
    });
    *selection = Some(GridSelection {
      column,
      rows: vec![row],
    });
  }
}

pub fn surrender_focused_widget(ctx: &egui::Context) {
  if let Some(id) = ctx.memory(|mem| mem.focused()) {
    ctx.memory_mut(|mem| mem.surrender_focus(id));
  }
}

pub fn grid_cell_active(response: &egui::Response) -> bool {
  response.has_focus() || response.lost_focus()
}

pub fn autocomplete_accept_key_pressed(ui: &egui::Ui, response: &egui::Response) -> bool {
  if !grid_cell_active(response) {
    return false;
  }
  ui.input(|input| {
    input.events.iter().any(|event| {
      matches!(
        event,
        egui::Event::Key {
          key: egui::Key::Tab | egui::Key::Enter,
          pressed: true,
          modifiers,
          ..
        } if !modifiers.any()
      )
    })
  })
}

pub fn grid_shift_pan_id() -> Id {
  Id::new("grid_shift_pan")
}

pub fn grid_apply_shift_hand_pan(ctx: &egui::Context, scroll_offset_y: &mut f32) {
  let shift = ctx.input(|input| input.modifiers.shift);
  if shift {
    let grabbing = ctx.input(|input| input.pointer.primary_down());
    ctx.set_cursor_icon(if grabbing {
      egui::CursorIcon::Grabbing
    } else {
      egui::CursorIcon::Grab
    });
  }
  if !shift || !ctx.input(|input| input.pointer.primary_down()) {
    return;
  }
  let delta = ctx.input(|input| input.pointer.delta());
  if delta.y != 0.0 {
    *scroll_offset_y = (*scroll_offset_y - delta.y).max(0.0);
    ctx.set_dragged_id(grid_shift_pan_id());
  }
}

pub fn request_grid_cell_focus(ui: &mut egui::Ui, id: Id) {
  let chevron_id = id.with("chevron");
  ui.memory_mut(|mem| {
    mem.surrender_focus(chevron_id);
    mem.request_focus(id);
  });
}

pub fn member_picker_max_height(member_count: usize) -> f32 {
  const SEP: f32 = 8.0;
  ((member_count + 1) as f32 * PICKER_ROW_HEIGHT + SEP).min(280.0)
}

pub fn measure_picker_label_width(ui: &egui::Ui, text: &str) -> f32 {
  let font_id = egui::TextStyle::Body.resolve(ui.style());
  ui.painter()
    .layout_no_wrap(text.to_owned(), font_id, crate::ui::theme::fg_primary())
    .size()
    .x
}

pub fn category_picker_menu_width(ui: &egui::Ui, categories: &[Category], cell_width: f32) -> f32 {
  let mut max_w = cell_width.max(CATEGORY_PICKER_MIN_WIDTH);
  for category in categories {
    let pad = if category.parent_id.is_some() { 36.0 } else { 24.0 };
    max_w = max_w.max(measure_picker_label_width(ui, &category.name) + pad);
  }
  max_w.min(CATEGORY_PICKER_MAX_WIDTH)
}

pub fn picker_popup_frame(style: &egui::Style) -> egui::Frame {
  egui::Frame::new()
    .fill(style.visuals.window_fill)
    .stroke(egui::Stroke::new(1.0_f32, style.visuals.widgets.noninteractive.bg_stroke.color))
    .corner_radius(4.0)
    .inner_margin(egui::Margin::same(4))
    .shadow(egui::Shadow::NONE)
}

pub fn grid_chevron_picker_popup(
  ui: &mut egui::Ui,
  chevron: &egui::Response,
  menu_width: f32,
  scroll_max_height: Option<f32>,
  build_menu: impl FnOnce(&mut egui::Ui),
) {
  let menu_width = menu_width.max(120.0);
  let mut anchor = chevron.clone();
  anchor.rect.min.y = ui.max_rect().min.y;
  anchor.rect.max.y = ui.max_rect().min.y + GRID_ROW_HEIGHT;
  anchor.id = anchor.id.with("popup_anchor");
  let _ = egui::Popup::from_toggle_button_response(&anchor)
    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
    .frame(picker_popup_frame(ui.style()))
    .width(menu_width)
    .layout(egui::Layout::top_down(egui::Align::LEFT))
    .show(|ui| {
      ui.set_min_width(menu_width);
      ui.set_max_width(menu_width);
      let show_body = |ui: &mut egui::Ui| {
        ui.set_width(menu_width);
        build_menu(ui);
      };
      if let Some(max_height) = scroll_max_height {
        egui::ScrollArea::vertical()
          .id_salt(chevron.id.with("picker_scroll"))
          .auto_shrink([true, true])
          .max_height(max_height)
          .show(ui, show_body);
      } else {
        show_body(ui);
      }
    });
}


pub fn grid_text_edit_cell(
  ui: &mut egui::Ui,
  value: &mut String,
  cell_id: Id,
  editing: bool,
) -> egui::Response {
  ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
  let output = egui::TextEdit::singleline(value)
    .id(cell_id)
    .interactive(editing)
    .desired_width(f32::INFINITY)
    .frame(egui::Frame::NONE)
    .margin(egui::Margin::same(0))
    .text_color(crate::ui::theme::fg_primary())
    .background_color(Color32::TRANSPARENT)
    .show(ui);
  let response = output.response.response.clone();
  if editing && response.has_focus() {
    ui.painter().rect_stroke(
      response.rect.expand(2.0),
      0.0,
      egui::Stroke::new(2.0_f32, crate::ui::theme::accent()),
      egui::StrokeKind::Outside,
    );
  }
  response
}

pub fn autocomplete_suggestions_list(value: &str, candidates: &[String]) -> Vec<String> {
  let prefix = value.trim().to_lowercase();
  if prefix.is_empty() {
    return Vec::new();
  }
  candidates
    .iter()
    .filter(|candidate| {
      let candidate_lower = candidate.to_lowercase();
      candidate_lower.starts_with(&prefix) && candidate_lower != prefix
    })
    .take(6)
    .cloned()
    .collect()
}

pub fn show_cell_autocomplete_popup(
  ui: &mut egui::Ui,
  response: &egui::Response,
  value: &str,
  candidates: &[String],
  selected_index: &mut usize,
) -> Option<String> {
  if !grid_cell_active(response) {
    return None;
  }

  let suggestions = autocomplete_suggestions_list(value, candidates);
  if suggestions.is_empty() {
    *selected_index = 0;
    return None;
  }

  if response.has_focus() {
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
      *selected_index = (*selected_index + 1).min(suggestions.len() - 1);
    }
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
      *selected_index = (*selected_index).saturating_sub(1);
    }
  }
  if *selected_index >= suggestions.len() {
    *selected_index = 0;
  }

  let mut picked = None;
  
  if autocomplete_accept_key_pressed(ui, response) {
    picked = Some(suggestions[*selected_index].clone());
  }

  let mut anchor = response.clone();
  anchor.rect.min.y = ui.max_rect().min.y;
  anchor.rect.max.y = ui.max_rect().min.y + GRID_ROW_HEIGHT;
  anchor.id = anchor.id.with("popup_anchor");
  let _ = egui::Popup::from_response(&anchor)
    .gap(2.0)
    .frame(picker_popup_frame(ui.style()))
    .show(|ui| {
      ui.set_min_width(anchor.rect.width().max(120.0));
      for (idx, suggestion) in suggestions.iter().enumerate() {
        let highlighted = idx == *selected_index;
        let btn = autocomplete_suggestion_button(ui, suggestion, highlighted);
        if btn.hovered() {
          *selected_index = idx;
        }
        if btn.clicked() {
          picked = Some(suggestion.clone());
        }
      }
    });

  picked
}

/// ponytail: combo-style autocomplete — same interaction as the grid cell
/// popup, but empty text lists EVERY candidate, typing switches to
/// contains-search, and `force_open` (a ▾ toggle) shows the list without
/// focus. Selection index lives in egui temp state, so callers need no
/// extra field.
pub fn show_autocomplete_popup(
  ui: &mut egui::Ui,
  response: &egui::Response,
  value: &str,
  candidates: &[String],
  force_open: bool,
  anchor: &egui::Response,
) -> Option<String> {
  if !grid_cell_active(response) && !force_open {
    return None;
  }
  let prefix = value.trim().to_lowercase();
  // ponytail: ▾ (force_open) always lists every candidate — a combo's arrow
  // ignores the current text; typed search stays contains-based.
  let suggestions: Vec<String> = if force_open {
    candidates.iter().filter(|candidate| !candidate.trim().is_empty()).cloned().collect()
  } else {
    candidates.iter()
      .filter(|candidate| {
        let candidate_lower = candidate.to_lowercase();
        !candidate_lower.is_empty()
          && (prefix.is_empty() || (candidate_lower.contains(&prefix) && candidate_lower != prefix))
      })
      .cloned()
      .collect()
  };
  if suggestions.is_empty() {
    return None;
  }

  let selection_id = response.id.with("autocomplete_selection");
  let mut selected_index = ui.ctx().data_mut(|d| d.get_temp::<usize>(selection_id).unwrap_or(0));
  if selected_index >= suggestions.len() {
    selected_index = 0;
  }
  if response.has_focus() {
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
      selected_index = (selected_index + 1).min(suggestions.len() - 1);
    }
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
      selected_index = selected_index.saturating_sub(1);
    }
  }

  let mut picked = None;
  if autocomplete_accept_key_pressed(ui, response) {
    picked = Some(suggestions[selected_index].clone());
  }

  let _ = egui::Popup::from_response(anchor)
    .gap(2.0)
    .frame(picker_popup_frame(ui.style()))
    .show(|ui| {
      ui.set_min_width(anchor.rect.width().max(140.0));
      for (idx, suggestion) in suggestions.iter().enumerate() {
        let highlighted = idx == selected_index;
        let btn = autocomplete_suggestion_button(ui, suggestion, highlighted);
        if btn.hovered() {
          selected_index = idx;
        }
        if btn.clicked() {
          picked = Some(suggestion.clone());
        }
      }
    });
  ui.ctx().data_mut(|d| d.insert_temp(selection_id, selected_index));

  picked
}

pub fn unique_nonempty_values<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
  let mut unique = Vec::new();
  for value in values {
    let trimmed = value.trim();
    if !trimmed.is_empty() && !unique.iter().any(|existing: &String| existing.eq_ignore_ascii_case(trimmed)) {
      unique.push(trimmed.to_owned());
    }
  }
  unique
}

pub fn autocomplete_suggestion_button(ui: &mut egui::Ui, label: &str, highlighted: bool) -> egui::Response {
  let text = RichText::new(label).color(crate::ui::theme::fg_primary());
  ui.add_sized(
    [ui.available_width(), 20.0],
    egui::Button::new(text)
      .fill(if highlighted { crate::ui::theme::bg_hover() } else { crate::ui::theme::bg_primary() })
      .stroke(egui::Stroke::new(0.0_f32, Color32::TRANSPARENT)),
  )
}


pub struct CategoryCellUi {
  pub text: egui::Response,
  pub chevron: egui::Response,
}

pub fn paint_chevron_down(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
  let cx = rect.center().x;
  let cy = rect.center().y;
  let w = 5.0;
  let h = 3.5;
  painter.add(Shape::convex_polygon(
    vec![
      pos2(cx - w, cy - h * 0.5),
      pos2(cx + w, cy - h * 0.5),
      pos2(cx, cy + h),
    ],
    color,
    egui::Stroke::NONE,
  ));
}

pub fn category_cell_ui(
  ui: &mut egui::Ui,
  value: &mut String,
  bg: Color32,
  cell_id: Id,
  editing: bool,
) -> CategoryCellUi {
  let mut display_val = if !editing && !value.is_empty() {
    value.rsplit(": ").next().unwrap_or(value).to_string()
  } else {
    value.clone()
  };
  ui.push_id(cell_id, |ui| {
    let cell_rect = ui.max_rect();
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

    let chevron_w = if cell_rect.width() > 16.0 { 16.0 } else { 0.0 };

    let text_output = egui::TextEdit::singleline(&mut display_val)
      .id(cell_id)
      .interactive(editing)
      .desired_width((cell_rect.width() - chevron_w).max(0.0))
      .frame(egui::Frame::NONE)
      .margin(egui::Margin { left: 3, right: 3, top: 0, bottom: 0 })
      .text_color(crate::ui::theme::fg_primary())
      .background_color(Color32::TRANSPARENT)
      .show(ui);

    if editing {
      *value = display_val;
    }

    let text = text_output.response.response.clone();

    let chevron_rect = egui::Rect::from_min_max(
      egui::pos2(cell_rect.right() - chevron_w, cell_rect.min.y),
      cell_rect.max,
    );
    let chevron_resp = ui.interact(chevron_rect, cell_id.with("chevron"), egui::Sense::click());

    if ui.is_rect_visible(chevron_rect) {
      let divider = Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), 22);
      ui.painter().line_segment(
        [chevron_rect.left_top(), chevron_rect.left_bottom()],
        egui::Stroke::new(0.5_f32, divider),
      );
      if chevron_resp.hovered() {
        let hover = crate::ui::theme::accent().linear_multiply(0.12);
        ui.painter().rect_filled(chevron_rect, 0.0, hover);
      }
      let chevron_color = if chevron_resp.hovered() {
        crate::ui::theme::accent()
      } else {
        bg
      };
      let chevron_paint =
        egui::Rect::from_center_size(chevron_rect.center(), egui::vec2(14.0, 10.0));
      paint_chevron_down(ui.painter(), chevron_paint, chevron_color);
    }
    let chevron = chevron_resp.on_hover_text("Pick from list (click)");

    if editing && text.has_focus() {
      ui.painter().rect_stroke(
        text.rect.expand(1.0),
        0.0,
        egui::Stroke::new(2.0_f32, crate::ui::theme::accent()),
        egui::StrokeKind::Outside,
      );
    }

    CategoryCellUi { text, chevron }
  }).inner
}

pub fn member_cell_ui(
  ui: &mut egui::Ui,
  value: &mut String,
  bg: Color32,
  cell_id: Id,
  editing: bool,
) -> CategoryCellUi {
  category_cell_ui(ui, value, bg, cell_id, editing)
}

pub fn member_none_row_selectable(ui: &mut egui::Ui, selected: bool) -> bool {
  let row_w = ui.available_width().max(1.0);
  let (rect, response) = ui.allocate_exact_size(egui::vec2(row_w, PICKER_ROW_HEIGHT), egui::Sense::click());
  if ui.is_rect_visible(rect) {
    if response.hovered() || selected {
      ui.painter().rect_filled(rect, 2.0, crate::ui::theme::bg_hover());
    }
    ui.painter().text(
      egui::pos2(rect.left() + 8.0, rect.center().y),
      egui::Align2::LEFT_CENTER,
      "(none)",
      egui::FontId::proportional(13.0),
      crate::ui::theme::fg_primary(),
    );
  }
  response.clicked()
}

pub fn member_row_selectable(ui: &mut egui::Ui, member: &HouseholdMember, selected: bool) -> bool {
  let row_w = ui.available_width().max(1.0);
  let (rect, response) = ui.allocate_exact_size(egui::vec2(row_w, PICKER_ROW_HEIGHT), egui::Sense::click());
  if ui.is_rect_visible(rect) {
    if response.hovered() || selected {
      ui.painter().rect_filled(rect, 2.0, crate::ui::theme::bg_hover());
    }
    let swatch = egui::Rect::from_min_size(
      rect.min + egui::vec2(6.0, (PICKER_ROW_HEIGHT - 12.0) * 0.5),
      egui::vec2(12.0, 12.0),
    );
    show_color_at(ui.painter(), member.color, swatch.shrink(1.0));
    ui.painter().text(
      egui::pos2(swatch.right() + 6.0, rect.center().y),
      egui::Align2::LEFT_CENTER,
      &member.name,
      egui::FontId::proportional(13.0),
      crate::ui::theme::fg_primary(),
    );
  }
  response.clicked()
}

fn contrast_on_accent(ui: &egui::Ui, selected: bool) -> Option<Color32> {
  if selected {
    Some(crate::ui::theme::contrast_text(crate::ui::theme::accent()))
  } else { None }
}

pub fn category_sub_picker_row(ui: &mut egui::Ui, label: &str, selected: bool, indent_px: f32) -> bool {
  ui.horizontal(|ui| {
    ui.add_space(indent_px);
    ui.set_width(ui.available_width().max(1.0));
    let text = if let Some(c) = contrast_on_accent(ui, selected) {
      RichText::new(label).color(c)
    } else {
      RichText::new(label)
    };
    ui.selectable_label(selected, text).clicked()
  }).inner
}

pub fn category_picker_header_row(ui: &mut egui::Ui, label: &str, bg: Color32) {
  let row_w = ui.available_width().max(1.0);
  let (rect, _) = ui.allocate_exact_size(egui::vec2(row_w, PICKER_ROW_HEIGHT), egui::Sense::hover());
  if ui.is_rect_visible(rect) {
    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter().text(
      egui::pos2(rect.left() + 8.0, rect.center().y),
      egui::Align2::LEFT_CENTER,
      label,
      egui::FontId::proportional(13.0),
      text_on_bg(bg),
    );
  }
}

pub fn category_picker_row(ui: &mut egui::Ui, label: &str, bg: Color32, selected: bool, indent_px: f32) -> bool {
  let row_w = ui.available_width().max(1.0);
  let (rect, response) = ui.allocate_exact_size(egui::vec2(row_w, PICKER_ROW_HEIGHT), egui::Sense::click());
  if ui.is_rect_visible(rect) {
    ui.painter().rect_filled(rect, 0.0, bg);
    if response.hovered() || selected {
      let overlay = crate::ui::theme::accent().linear_multiply(0.14);
      ui.painter().rect_filled(rect, 0.0, overlay);
    }
    ui.painter().text(
      egui::pos2(rect.left() + indent_px + 8.0, rect.center().y),
      egui::Align2::LEFT_CENTER,
      label,
      egui::FontId::proportional(13.0),
      text_on_bg(bg),
    );
  }
  response.clicked()
}

pub fn household_panel<R>(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
  ui.add_space(crate::ui::theme_tokens::SPACE_3);
  crate::ui::components::card_subtle(ui)
    .inner_margin(egui::Margin::symmetric(crate::ui::theme_tokens::SPACE_3 as i8, crate::ui::theme_tokens::SPACE_2 as i8))
    .show(ui, |ui| {
      // ponytail: section_header reads as a quiet marker. The old strong+fg_default
      // label was visually identical to the page heading, which fought the hierarchy.
      crate::ui::components::section_header(ui, title);
      ui.add_space(crate::ui::theme_tokens::SPACE_2);
      add_contents(ui)
    })
    .inner
}

pub fn color_swatch_button(ui: &mut egui::Ui, color: Color32) -> egui::Response {
  let size = egui::vec2(22.0, 16.0);
  let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
  if ui.is_rect_visible(rect) {
    show_color_at(ui.painter(), color, rect.shrink(1.0));
    ui.painter().rect_stroke(
      rect,
      2.0,
      egui::Stroke::new(1.0_f32, crate::ui::theme::border()),
      egui::StrokeKind::Inside,
    );
  }
  response.on_hover_text("Click to edit color")
}

pub fn sorted_grid_indices<R: GridRow>(
  rows: &[R],
  sort: ExpenseSort,
) -> Vec<usize> {
  let mut indices: Vec<usize> = (0..rows.len()).collect();
  indices.sort_by(|&a, &b| {
    let cmp = match sort.column {
      ExpenseSortColumn::Date => rows[a].row_date().cmp(rows[b].row_date()),
      ExpenseSortColumn::Amount => rows[a].row_amount_cents().cmp(&rows[b].row_amount_cents()),
      ExpenseSortColumn::Member => rows[a].row_member().to_lowercase().cmp(&rows[b].row_member().to_lowercase()),
      ExpenseSortColumn::Category => rows[a].row_category().to_lowercase().cmp(&rows[b].row_category().to_lowercase()),
      ExpenseSortColumn::Vendor => rows[a].row_vendor().to_lowercase().cmp(&rows[b].row_vendor().to_lowercase()),
      ExpenseSortColumn::Description => rows[a].row_description().to_lowercase().cmp(&rows[b].row_description().to_lowercase()),
      ExpenseSortColumn::Account => rows[a].row_account().to_lowercase().cmp(&rows[b].row_account().to_lowercase()),
    };
    if sort.ascending { cmp } else { cmp.reverse() }
  });
  indices
}

pub fn text_on_bg(bg: Color32) -> Color32 {
  // ponytail: delegates to the single contrast helper — the old local
  // threshold drifted from theme::contrast_text's, producing mismatched
  // text-on-swatch colors between pickers and the rest of the app.
  crate::ui::theme::contrast_text(bg)
}

pub fn ui_grid_text_edit(
  ui: &mut egui::Ui,
  value: &mut String,
  edit_original: &mut Option<String>,
  edit_cell: &mut Option<(GridColumn, usize)>,
  cell_id: Id,
  editing: bool,
) -> egui::Response {
  if editing && edit_original.is_none() {
    *edit_original = Some(value.clone());
  }
  let response = grid_text_edit_cell(ui, value, cell_id, editing);
  if editing && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
    if let Some(orig) = edit_original.take() {
      *value = orig;
    }
    *edit_cell = None;
    response.surrender_focus();
  }
  response
}

pub fn ui_grid_date_edit(
  ui: &mut egui::Ui,
  value: &mut String,
  edit_original: &mut Option<String>,
  edit_cell: &mut Option<(GridColumn, usize)>,
  cell_id: Id,
  editing: bool,
) -> (egui::Response, bool) {
  if editing {
    let just_started = edit_original.is_none();
    if edit_original.is_none() {
      *edit_original = Some(value.clone());
    }
    
    let trimmed = value.trim();
    let parsed_date_result = std::str::FromStr::from_str(trimmed);
    let mut parsed_date = parsed_date_result.clone()
        .unwrap_or_else(|_| jiff::civil::Date::constant(2025, 1, 1));
    let initial_parsed = parsed_date.clone();
        
    let response = ui.push_id(cell_id.with("datepicker"), |ui| {
        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        ui.style_mut().visuals.widgets.hovered.bg_fill = crate::ui::theme::bg_hover();
        ui.style_mut().visuals.widgets.active.bg_fill = crate::ui::theme::bg_hover();
        ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
        ui.style_mut().spacing.button_padding = egui::vec2(4.0, 0.0);
        
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = crate::ui::theme::fg_primary();
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = crate::ui::theme::fg_primary();
        ui.style_mut().spacing.interact_size.y = 18.0;
        ui.add(egui_extras::DatePickerButton::new(&mut parsed_date).highlight_weekends(false))
    }).inner;
    
    let mut date_changed = false;
    if !just_started && response.changed() && parsed_date != initial_parsed {
        *value = parsed_date.to_string();
        date_changed = true;
    }
    
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
      if let Some(orig) = edit_original.take() {
        *value = orig;
      }
      *edit_cell = None;
      response.surrender_focus();
    }
    (response, date_changed)
  } else {
    let response = grid_text_edit_cell(ui, value, cell_id, false);
    (response, false)
  }
}



/// Modal window frame: 1px border, 8px radius, 24px padding, drop shadow.
/// ponytail: thin shim over `components::modal` so existing call sites
/// keep their `&ctx` signature.
pub fn themed_modal_frame(ctx: &egui::Context) -> egui::Frame {
  let _ = ctx;
  crate::ui::components::modal()
}

pub fn themed_panel_frame(_style: &egui::Style) -> egui::Frame {
  // ponytail: shim over `components::card_subtle` for backwards compat.
  // The Style arg is ignored; the active palette supplies the colors.
  crate::ui::components::card_subtle_dummy()
}



pub fn accent_palette_swatch_button(
  ui: &mut egui::Ui,
  swatch: Color32,
  selected: bool,
) -> egui::Response {
  let size = egui::vec2(20.0, 20.0);
  let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
  if ui.is_rect_visible(rect) {
    let painter = ui.painter();
    let inner_rect = rect.shrink(if selected { 3.0 } else { 1.0 });
    show_color_at(painter, swatch, inner_rect);
    
    if selected {
      painter.rect_stroke(
        rect,
        3.0,
        egui::Stroke::new(2.0_f32, crate::ui::theme::accent()),
        egui::StrokeKind::Outside,
      );
    } else if response.hovered() {
      painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, crate::ui::theme::border_strong()),
        egui::StrokeKind::Outside,
      );
    } else {
      painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, crate::ui::theme::border()),
        egui::StrokeKind::Inside,
      );
    }
  }
  response
}

pub fn accent_color_picker_ui(ui: &mut egui::Ui, color: &mut Color32) -> bool {
  let mut changed = false;
  ui.label(RichText::new("Choose an accent color").small().color(crate::ui::theme::fg_primary()));
  ui.add_space(4.0);

  const COLS: usize = 6;
  const SWATCH_W: f32 = 20.0;
  const GAP: f32 = 4.0;
  let grid_width = COLS as f32 * SWATCH_W + (COLS.saturating_sub(1)) as f32 * GAP;

  let swatches = accent_palette_swatches();
  egui::ScrollArea::vertical()
    .id_salt("accent_palette_scroll")
    .auto_shrink([false, true])
    .max_height(72.0)
    .show(ui, |ui| {
      ui.set_width(grid_width);
      ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
      for row in swatches.chunks(COLS) {
        ui.horizontal(|ui| {
          ui.spacing_mut().item_spacing = egui::vec2(GAP, 0.0);
          for &swatch in row {
            let selected = colors_match(*color, swatch);
            if accent_palette_swatch_button(ui, swatch, selected).clicked() {
              *color = swatch;
              changed = true;
            }
          }
        });
      }
    });
  changed
}

pub fn grid_column_next(column: GridColumn) -> Option<GridColumn> {
  match column {
    GridColumn::Date => Some(GridColumn::Account),
    GridColumn::Account => Some(GridColumn::Amount),
    GridColumn::Amount => Some(GridColumn::Member),
    GridColumn::Member => Some(GridColumn::Category),
    GridColumn::Category => Some(GridColumn::Vendor),
    GridColumn::Vendor => Some(GridColumn::Description),
    GridColumn::Description => None,
  }
}

pub fn grid_column_prev(column: GridColumn) -> Option<GridColumn> {
  match column {
    GridColumn::Date => None,
    GridColumn::Account => Some(GridColumn::Date),
    GridColumn::Amount => Some(GridColumn::Account),
    GridColumn::Member => Some(GridColumn::Amount),
    GridColumn::Category => Some(GridColumn::Member),
    GridColumn::Vendor => Some(GridColumn::Category),
    GridColumn::Description => Some(GridColumn::Vendor),
  }
}

pub fn grid_nav_target(
  sorted_indices: &[usize],
  expense_idx: usize,
  column: GridColumn,
  nav: GridNav,
) -> Option<(GridColumn, usize)> {
  let row_pos = sorted_indices.iter().position(|&i| i == expense_idx)?;
  match nav {
    GridNav::NextRow => sorted_indices
      .get(row_pos + 1)
      .map(|&idx| (column, idx)),
    GridNav::PrevRow => row_pos
      .checked_sub(1)
      .and_then(|pos| sorted_indices.get(pos).map(|&idx| (column, idx))),
    GridNav::NextCol => grid_column_next(column).map(|col| (col, expense_idx)),
    GridNav::PrevCol => grid_column_prev(column).map(|col| (col, expense_idx)),
  }
}

pub fn expense_grid_selection_status(ui: &mut egui::Ui, selection: &Option<GridSelection>) {
  let width = ui.available_width().max(1.0);
  let (rect, _) = ui.allocate_exact_size(
    egui::vec2(width, GRID_SELECTION_STATUS_HEIGHT),
    egui::Sense::hover(),
  );
  if !ui.is_rect_visible(rect) {
    return;
  }
  if let Some(sel) = selection {
    if !sel.rows.is_empty() {
      let label = if sel.rows.len() > 1 {
        format!(
          "{} rows selected in {:?} column — type to edit, Enter applies to all, Escape clears",
          sel.rows.len(),
          sel.column
        )
      } else {
        format!("1 row selected in {:?} column", sel.column)
      };
      ui.painter().text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        crate::ui::theme::fg_primary(),
      );
    }
  }
}

pub fn viewport_needs_reposition(ctx: &egui::Context) -> bool {
  ctx.input(|i| {
    let vp = i.viewport();
    if vp.minimized == Some(true) {
      return true;
    }
    if vp.visible() == Some(false) {
      return true;
    }
    if let Some(outer) = vp.outer_rect {
      // ponytail: eframe's persisted size restores verbatim and bypasses
      // min_inner_size (only clamped to 64pt in egui-winit) — a tiny size
      // saved by an old build kept relaunching squished. Anything under the
      // app's min (720×560) counts as broken and gets re-sized.
      if outer.width() < 720.0 || outer.height() < 560.0 {
        return true;
      }
      if let Some(monitor) = vp.monitor_size {
        if monitor.x > 1.0 && monitor.y > 1.0 {
          let monitor_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, monitor);
          return !monitor_rect.intersects(outer);
        }
      }
    }
    // Rect not reported yet — do nothing; the retry loop re-checks next frame.
    false
  })
}

pub fn ensure_root_window_visible(ctx: &egui::Context) {
  ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
  if ctx.input(|i| i.viewport().minimized == Some(true)) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
  }
  // Rescue only when the window is actually misplaced or undersized —
  // never re-center, so the persisted position (and monitor) survives
  // relaunches. Re-sizing to the default also fixes a stale tiny
  // persisted size; eframe autosaves the corrected value.
  if viewport_needs_reposition(ctx) {
    let undersized = ctx.input(|i| {
      i.viewport()
        .outer_rect
        .is_some_and(|r| r.width() < 720.0 || r.height() < 560.0)
    });
    if undersized {
      ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1180.0, 780.0)));
    } else if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
      ctx.send_viewport_cmd(cmd);
    }
  }
  ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
  ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
    egui::UserAttentionType::Critical,
  ));
}

pub fn commit_grid_cell<R: GridRow>(
  rows: &mut [R],
  selection: &Option<GridSelection>,
  column: GridColumn,
  idx: usize,
) -> Vec<usize> {
  let targets = grid_commit_targets(selection, column, idx);
  let Some(source) = rows.get(idx) else { return Vec::new(); };
  match column {
    GridColumn::Date => {
      let value = source.row_date().to_string();
      for &t in &targets {
        if let Some(r) = rows.get_mut(t) {
          *r.row_date_mut() = value.clone();
        }
      }
    }
    GridColumn::Amount => {
      let value = source.row_amount_input().to_string();
      let cents = source.row_amount_cents();
      for &t in &targets {
        if let Some(r) = rows.get_mut(t) {
          *r.row_amount_input_mut() = value.clone();
          r.set_row_amount_cents(cents);
        }
      }
    }
    GridColumn::Member => {
      let value = source.row_member().to_string();
      for &t in &targets {
        if let Some(r) = rows.get_mut(t) {
          *r.row_member_mut() = value.clone();
        }
      }
    }
    GridColumn::Category => {
      let value = source.row_category().to_string();
      for &t in &targets {
        if let Some(r) = rows.get_mut(t) {
          *r.row_category_mut() = value.clone();
        }
      }
    }
    GridColumn::Vendor => {
      let value = source.row_vendor().to_string();
      for &t in &targets {
        if let Some(r) = rows.get_mut(t) {
          *r.row_vendor_mut() = value.clone();
        }
      }
    }
    GridColumn::Description => {
      let value = source.row_description().to_string();
      for &t in &targets {
        if let Some(r) = rows.get_mut(t) {
          *r.row_description_mut() = value.clone();
        }
      }
    }
    GridColumn::Account => {
      let value = source.row_account().to_string();
      for &t in &targets {
        if let Some(r) = rows.get_mut(t) {
          *r.row_account_mut() = value.clone();
        }
      }
    }
  }
  targets
}

impl GridState {
  pub fn clear_selection(&mut self) {
    self.selection = None;
    self.drag = None;
    self.edit_cell = None;
    self.typeahead = None;
    self.active_cell = None;
  }

  pub fn handle_raw_input<R: GridRow>(
    &mut self,
    ctx: &egui::Context,
    raw_input: &mut egui::RawInput,
    rows: &[R],
    candidates_fn: impl Fn(GridColumn) -> Vec<String>,
  ) {
    let Some((column, idx)) = self.active_cell else {
      return;
    };
    let mut keyboard_action = None;
    raw_input.events.retain(|event| {
      match event {
        egui::Event::Key {
          key: egui::Key::Enter,
          pressed: true,
          modifiers,
          ..
        } if !modifiers.any() => {
          let has_suggestions = if let Some(row) = rows.get(idx) {
            let val = match column {
              GridColumn::Member      => row.row_member(),
              GridColumn::Category    => row.row_category(),
              GridColumn::Vendor      => row.row_vendor(),
              GridColumn::Description => row.row_description(),
              _ => "",
            };
            !autocomplete_suggestions_list(val, &candidates_fn(column)).is_empty()
          } else {
            false
          };
          
          if has_suggestions {
            keyboard_action = Some(GridKeyboardAction::Tab { shift: false });
            false
          } else if self.selection.as_ref().is_some_and(|selection| {
            selection.column == column && selection.rows.len() > 1
          }) {
            true
          } else {
            keyboard_action = Some(GridKeyboardAction::Enter);
            false
          }
        }
        egui::Event::Key {
          key: egui::Key::Tab,
          pressed: true,
          modifiers,
          ..
        } => {
          keyboard_action = Some(GridKeyboardAction::Tab {
            shift: modifiers.shift,
          });
          false
        }
        _ => true,
      }
    });

    if let Some(action) = keyboard_action {
      self.pending_keyboard = Some(GridPendingKeyboard {
        column,
        expense_idx: idx,
        action,
      });
      ctx.request_repaint();
    }
  }

  pub fn apply_pending_keyboard<R: GridRow>(
    &mut self,
    rows: &mut [R],
    sorted_indices: &[usize],
    autocomplete_selection: usize,
    candidates_fn: impl Fn(GridColumn) -> Vec<String>,
  ) -> Option<(GridColumn, usize, Vec<usize>)> {
    let pending = self.pending_keyboard.take()?;
    
    // 1. If Tab, fill autocomplete
    if matches!(pending.action, GridKeyboardAction::Tab { .. }) {
      let candidates = candidates_fn(pending.column);
      if let Some(row) = rows.get_mut(pending.expense_idx) {
        let value_mut = match pending.column {
          GridColumn::Member      => Some(row.row_member_mut()),
          GridColumn::Category    => Some(row.row_category_mut()),
          GridColumn::Vendor      => Some(row.row_vendor_mut()),
          GridColumn::Description => Some(row.row_description_mut()),
          _ => None,
        };
        if let Some(val) = value_mut {
          let suggestions = autocomplete_suggestions_list(val, &candidates);
          if let Some(suggestion) = suggestions.get(autocomplete_selection) {
            *val = suggestion.clone();
          }
        }
      }
    }
    
    // 2. Commit the cell in memory
    let committed_targets = commit_grid_cell(rows, &self.selection, pending.column, pending.expense_idx);
    
    // 3. Set the new focus target
    self.pending_focus_target = grid_nav_target(sorted_indices, pending.expense_idx, pending.column, pending.action.to_nav());
    self.edit_original = None;
    
    Some((pending.column, pending.expense_idx, committed_targets))
  }
}

#[cfg(test)]
mod tests {
  use super::selection_range;

  // The invariant every grid_row_selected/grid_commit_targets binary_search
  // relies on: selection rows ascend, even when the visual (sorted) order
  // of the grid is a scrambled permutation of raw indices.
  #[test]
  fn selection_range_is_sorted_for_scrambled_sort_order() {
    let sorted = vec![9, 2, 7, 0, 5, 1, 8, 3, 6, 4];
    let rows = selection_range(&sorted, 7, 1);
    assert!(rows.windows(2).all(|w| w[0] < w[1]), "rows not ascending: {rows:?}");
    // visual slice: 7 at pos 2, 1 at pos 5 → [7,0,5,1] → sorted {0,1,5,7}
    assert_eq!(rows, vec![0, 1, 5, 7]);
    for &r in &rows {
      assert!(rows.binary_search(&r).is_ok(), "binary_search missed {r}: {rows:?}");
    }
    assert!(rows.binary_search(&3).is_err());
  }

  #[test]
  fn selection_range_falls_back_to_anchor_when_outside_sorted() {
    assert_eq!(selection_range(&[2, 0, 1], 99, 0), vec![99]);
  }
}


