use eframe::egui::{self, Color32, Id, Shape, pos2, RichText, color_picker::show_color_at};
use egui_extras::Column;

use crate::models::*;
use crate::db::*;

pub const GRID_HEADER_HEIGHT: f32 = 24.0;
pub const GRID_ROW_HEIGHT: f32 = 24.0;
pub const DESCRIPTION_MIN_WIDTH: f32 = 160.0;
pub const GRID_SELECTION_STATUS_HEIGHT: f32 = 18.0;

pub const PICKER_ROW_HEIGHT: f32 = 24.0;
pub const CATEGORY_PICKER_MIN_WIDTH: f32 = 200.0;
pub const CATEGORY_PICKER_MAX_WIDTH: f32 = 320.0;
pub const MEMBER_PICKER_MIN_WIDTH: f32 = 140.0;

pub fn grid_header(ui: &mut egui::Ui, label: &str) {
  let width = ui.available_width().max(1.0);
  let (rect, _) = ui.allocate_exact_size(egui::vec2(width, GRID_HEADER_HEIGHT), egui::Sense::hover());
  ui.painter().rect_filled(rect, 0.0, ui.visuals().window_fill);
  ui.painter().rect_stroke(
    rect,
    0.0,
    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    egui::StrokeKind::Inside,
  );
  ui.painter().text(
    egui::pos2(rect.left() + 4.0, rect.center().y),
    egui::Align2::LEFT_CENTER,
    label,
    egui::FontId::proportional(13.0),
    ui.visuals().strong_text_color(),
  );
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
      sorted_indices[lo..=hi].to_vec()
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
    if sel.column == column && sel.rows.len() > 1 && sel.rows.contains(&active_row) {
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
  selection
    .as_ref()
    .is_some_and(|sel| sel.column == column && sel.rows.contains(&row))
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
  if ui.ctx().memory(|mem| mem.focused().is_some()) {
    return false;
  }
  let Some(ch) = grid_consume_printable_char(ui) else {
    return false;
  };
  let Some((column, row)) = grid_selection_anchor(selection) else {
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
  dragging: bool,
  editing: bool,
) {
  if !ui.is_rect_visible(cell_rect) {
    return;
  }
  if dragging && selected {
    ui.painter().rect_filled(cell_rect, 0.0, Color32::from_rgba_unmultiplied(ui.visuals().selection.bg_fill.r(), ui.visuals().selection.bg_fill.g(), ui.visuals().selection.bg_fill.b(), 40));
  } else if selected {
    ui.painter().rect_filled(cell_rect, 0.0, Color32::from_rgba_unmultiplied(ui.visuals().selection.bg_fill.r(), ui.visuals().selection.bg_fill.g(), ui.visuals().selection.bg_fill.b(), 15));
  }
  if editing {
    ui.painter().rect_stroke(
      cell_rect.expand(1.0),
      0.0,
      egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
      egui::StrokeKind::Outside,
    );
  } else if selected {
    ui.painter().rect_stroke(
      cell_rect,
      0.0,
      egui::Stroke::new(1.0, ui.visuals().selection.bg_fill),
      egui::StrokeKind::Inside,
    );
  }
}

pub fn process_grid_column_cell(
  ui: &mut egui::Ui,
  column: GridColumn,
  row: usize,
  selection: &mut Option<GridSelection>,
  drag: &mut Option<GridSelectDrag>,
  edit_cell: &mut Option<(GridColumn, usize)>,
) {
  let cell_rect = ui.max_rect();
  let selected = grid_row_selected(selection, column, row);
  let dragging = drag.as_ref().is_some_and(|active| active.column == column);
  let editing = grid_cell_editing(edit_cell, column, row);
  paint_grid_cell_highlight(ui, cell_rect, selected, dragging, editing);

  let cell_id = ui.id().with("cell").with(column).with(row);
  let response = ui.interact(cell_rect, cell_id, egui::Sense::click_and_drag());

  if response.clicked() {
    *selection = Some(GridSelection {
      column,
      rows: vec![row],
    });
    *edit_cell = None;
  }

  if response.double_clicked() {
    *selection = Some(GridSelection {
      column,
      rows: vec![row],
    });
    let is_import = ui.ctx().data(|d| d.get_temp::<bool>(egui::Id::new("is_rendering_import_grid")).unwrap_or(false));
    let pending_key = if is_import { "pending_import_edit_cell" } else { "pending_edit_cell" };
    ui.ctx().data_mut(|d| {
      d.insert_temp(egui::Id::new(pending_key), Some((column, row)));
    });
    ui.ctx().request_repaint();
  }

  if response.drag_started() {
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

  if response.drag_stopped() {
    *drag = None;
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

pub fn request_expense_cell_focus(ui: &mut egui::Ui, column: GridColumn, expense_idx: usize) {
  request_grid_cell_focus(ui, expense_cell_id(column, expense_idx));
}

pub fn request_grid_cell_focus(ui: &mut egui::Ui, id: Id) {
  let chevron_id = id.with("chevron");
  ui.memory_mut(|mem| {
    mem.surrender_focus(chevron_id);
    mem.request_focus(id);
  });
}

pub fn expense_cell_id(column: GridColumn, expense_idx: usize) -> Id {
  Id::new(("expense_cell", format!("{column:?}"), expense_idx))
}

pub fn member_picker_max_height(member_count: usize) -> f32 {
  const SEP: f32 = 8.0;
  ((member_count + 1) as f32 * PICKER_ROW_HEIGHT + SEP).min(280.0)
}

pub fn measure_picker_label_width(ui: &egui::Ui, text: &str) -> f32 {
  let font_id = egui::TextStyle::Body.resolve(ui.style());
  ui.painter()
    .layout_no_wrap(text.to_owned(), font_id, ui.visuals().text_color())
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
    .stroke(egui::Stroke::new(1.0, style.visuals.widgets.noninteractive.bg_stroke.color))
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
    .text_color(ui.visuals().text_color())
    .background_color(Color32::TRANSPARENT)
    .show(ui);
  let response = output.response.response.clone();
  if editing && response.has_focus() {
    ui.painter().rect_stroke(
      response.rect.expand(2.0),
      0.0,
      egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
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
  let text = RichText::new(label).color(ui.visuals().text_color());
  ui.add_sized(
    [ui.available_width(), 20.0],
    egui::Button::new(text)
      .fill(if highlighted { ui.visuals().widgets.hovered.bg_fill } else { ui.visuals().window_fill })
      .stroke(egui::Stroke::new(0.0, Color32::TRANSPARENT)),
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
  ui.push_id(cell_id, |ui| {
    let cell_rect = ui.max_rect();
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

    let text_w = if ui.max_rect().width() > 16.0 { ui.max_rect().width() - 16.0 } else { 0.0 };
    let chevron_w = if ui.max_rect().width() > 16.0 { 16.0 } else { 0.0 };

    let text_color = text_on_bg(bg);

    let text_output = egui::TextEdit::singleline(value)
      .id(cell_id)
      .interactive(editing)
      .desired_width(text_w)
      .frame(egui::Frame::NONE)
      .margin(egui::Margin { left: 3, right: 3, top: 0, bottom: 0 })
      .text_color(text_color)
      .background_color(Color32::TRANSPARENT)
      .show(ui);

    let text = text_output.response.response.clone();

    let chevron_rect = egui::Rect::from_min_max(
      egui::pos2(cell_rect.right() - chevron_w, cell_rect.min.y),
      cell_rect.max,
    );
    let chevron_resp = ui.interact(chevron_rect, cell_id.with("chevron"), egui::Sense::click());

    if ui.is_rect_visible(chevron_rect) {
      let accent = text_on_bg(bg);
      let divider = Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 22);
      ui.painter().line_segment(
        [chevron_rect.left_top(), chevron_rect.left_bottom()],
        egui::Stroke::new(0.5, divider),
      );
      if chevron_resp.hovered() {
        ui.painter().rect_filled(chevron_rect, 0.0, Color32::from_black_alpha(30));
      }
      let chevron_color = if chevron_resp.hovered() {
        ui.visuals().selection.bg_fill
      } else {
        accent
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
        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
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
      ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
    }
    ui.painter().text(
      egui::pos2(rect.left() + 8.0, rect.center().y),
      egui::Align2::LEFT_CENTER,
      "(none)",
      egui::FontId::proportional(13.0),
      ui.visuals().text_color(),
    );
  }
  response.clicked()
}

pub fn member_row_selectable(ui: &mut egui::Ui, member: &HouseholdMember, selected: bool) -> bool {
  let row_w = ui.available_width().max(1.0);
  let (rect, response) = ui.allocate_exact_size(egui::vec2(row_w, PICKER_ROW_HEIGHT), egui::Sense::click());
  if ui.is_rect_visible(rect) {
    if response.hovered() || selected {
      ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
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
      ui.visuals().text_color(),
    );
  }
  response.clicked()
}

pub fn category_sub_picker_row(ui: &mut egui::Ui, label: &str, selected: bool, indent_px: f32) -> bool {
  ui.horizontal(|ui| {
    ui.add_space(indent_px);
    ui.set_width(ui.available_width().max(1.0));
    ui.selectable_label(selected, RichText::new(label).color(ui.visuals().text_color()))
      .clicked()
  })
  .inner
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
      ui.painter().rect_filled(rect, 0.0, Color32::from_black_alpha(35));
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
  ui.add_space(10.0);
  egui::Frame::new()
    .fill(ui.visuals().window_fill)
    .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_fill))
    .corner_radius(8.0)
    .inner_margin(egui::Margin::symmetric(14, 10))
    .show(ui, |ui| {
      ui.label(RichText::new(title).strong().color(ui.visuals().selection.bg_fill));
      ui.add_space(8.0);
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
      egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
      egui::StrokeKind::Inside,
    );
  }
  response.on_hover_text("Click to edit color")
}

pub fn sorted_expense_indices(
  expenses: &[Expense],
  sort: ExpenseSort,
  edit_cell: Option<(GridColumn, usize)>,
  edit_original: Option<&str>,
) -> Vec<usize> {
  let mut indices: Vec<usize> = (0..expenses.len()).collect();
  indices.sort_by(|&a, &b| {
    let cmp = match sort.column {
      ExpenseSortColumn::Date => {
          let val_a = if edit_cell == Some((GridColumn::Date, a)) { edit_original.unwrap_or(&expenses[a].date) } else { &expenses[a].date };
          let val_b = if edit_cell == Some((GridColumn::Date, b)) { edit_original.unwrap_or(&expenses[b].date) } else { &expenses[b].date };
          val_a.cmp(val_b)
      }
      ExpenseSortColumn::Amount => expenses[a].amount_cents.cmp(&expenses[b].amount_cents),
      ExpenseSortColumn::Member => {
          let val_a = if edit_cell == Some((GridColumn::Member, a)) { edit_original.unwrap_or(&expenses[a].member) } else { &expenses[a].member };
          let val_b = if edit_cell == Some((GridColumn::Member, b)) { edit_original.unwrap_or(&expenses[b].member) } else { &expenses[b].member };
          val_a.to_lowercase().cmp(&val_b.to_lowercase())
      }
      ExpenseSortColumn::Category => {
          let val_a = if edit_cell == Some((GridColumn::Category, a)) { edit_original.unwrap_or(&expenses[a].category) } else { &expenses[a].category };
          let val_b = if edit_cell == Some((GridColumn::Category, b)) { edit_original.unwrap_or(&expenses[b].category) } else { &expenses[b].category };
          val_a.to_lowercase().cmp(&val_b.to_lowercase())
      }
      ExpenseSortColumn::Vendor => {
          let val_a = if edit_cell == Some((GridColumn::Vendor, a)) { edit_original.unwrap_or(&expenses[a].vendor) } else { &expenses[a].vendor };
          let val_b = if edit_cell == Some((GridColumn::Vendor, b)) { edit_original.unwrap_or(&expenses[b].vendor) } else { &expenses[b].vendor };
          val_a.to_lowercase().cmp(&val_b.to_lowercase())
      }
      ExpenseSortColumn::Description => {
          let val_a = if edit_cell == Some((GridColumn::Description, a)) { edit_original.unwrap_or(&expenses[a].description) } else { &expenses[a].description };
          let val_b = if edit_cell == Some((GridColumn::Description, b)) { edit_original.unwrap_or(&expenses[b].description) } else { &expenses[b].description };
          val_a.to_lowercase().cmp(&val_b.to_lowercase())
      }
    };
    if sort.ascending { cmp } else { cmp.reverse() }
  });
  indices
}

pub fn text_on_bg(bg: Color32) -> Color32 {
  let lum = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
  if lum > 120.0 { Color32::from_rgb(0x19, 0x19, 0x19) } else { Color32::WHITE }
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
        ui.style_mut().visuals.widgets.hovered.bg_fill = ui.visuals().widgets.hovered.bg_fill;
        ui.style_mut().visuals.widgets.active.bg_fill = ui.visuals().widgets.hovered.bg_fill;
        ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
        ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
        ui.style_mut().spacing.button_padding = egui::vec2(4.0, 0.0);
        
        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = ui.visuals().text_color();
        ui.style_mut().visuals.widgets.hovered.fg_stroke.color = ui.visuals().selection.bg_fill;
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


pub fn resizable_column(initial: f32, minimum: f32, maximum: f32) -> Column {
  Column::initial(initial)
    .at_least(minimum)
    .at_most(maximum.max(minimum))
    .resizable(true)
    .clip(true)
}

pub fn description_column() -> Column {
  Column::remainder()
    .at_least(DESCRIPTION_MIN_WIDTH)
    .resizable(false)
    .clip(true)
}

pub fn spreadsheet_columns(
  ui: &mut egui::Ui,
  date: f32,
  amount: f32,
  member: f32,
  category: f32,
  vendor: f32,
) -> [Column; 6] {
  let table_width = ui.available_width().max(800.0);
  let fixed_budget = (table_width - DESCRIPTION_MIN_WIDTH).max(420.0);
  [
    resizable_column(date, 72.0, fixed_budget * 0.14),
    resizable_column(amount, 72.0, fixed_budget * 0.12),
    resizable_column(member, 88.0, fixed_budget * 0.14),
    resizable_column(category, 140.0, fixed_budget * 0.26),
    resizable_column(vendor, 100.0, fixed_budget * 0.24),
    description_column(),
  ]
}

pub fn themed_modal_frame(ctx: &egui::Context) -> egui::Frame {
  egui::Frame::window(&ctx.global_style())
    .fill(ctx.global_style().visuals.panel_fill)
    .stroke(egui::Stroke::new(2.0, ctx.global_style().visuals.selection.bg_fill))
    .corner_radius(10.0)
    .inner_margin(egui::Margin::symmetric(14, 12))
}

pub fn themed_panel_frame(style: &egui::Style) -> egui::Frame {
  egui::Frame::new()
    .fill(style.visuals.window_fill)
    .stroke(egui::Stroke::new(1.0, style.visuals.widgets.noninteractive.bg_stroke.color))
    .corner_radius(8.0)
    .inner_margin(egui::Margin::symmetric(10, 8))
}

pub fn configure_theme(ctx: &egui::Context, is_dark: bool) {
  let mut visuals = if is_dark { egui::Visuals::dark() } else { egui::Visuals::light() };
  
  if is_dark {
    let panel_fill = Color32::from_rgb(0x0e, 0x0f, 0x11);
    let window_fill = Color32::from_rgb(0x1a, 0x1c, 0x20);
    let hovered_bg = Color32::from_rgb(0x2c, 0x2e, 0x33);
    let text_primary = Color32::from_rgb(0xf3, 0xf4, 0xf6);
    let text_muted = Color32::from_rgb(0x9c, 0xa3, 0xaf);
    let border_color = Color32::from_rgb(0x2c, 0x2e, 0x33);
    let accent_cyan = Color32::from_rgb(0x3b, 0x82, 0xf6);

    visuals.panel_fill = panel_fill;
    visuals.window_fill = window_fill;
    visuals.extreme_bg_color = panel_fill;
    visuals.faint_bg_color = window_fill;
    visuals.text_edit_bg_color = Some(panel_fill);
    visuals.widgets.noninteractive.bg_fill = window_fill;
    visuals.widgets.inactive.bg_fill = window_fill;
    visuals.widgets.inactive.weak_bg_fill = hovered_bg;
    visuals.widgets.hovered.bg_fill = hovered_bg;
    visuals.widgets.active.bg_fill = hovered_bg;
    visuals.widgets.open.bg_fill = hovered_bg;
    visuals.widgets.noninteractive.fg_stroke.color = text_muted;
    visuals.widgets.inactive.fg_stroke.color = text_muted;
    visuals.widgets.hovered.fg_stroke.color = text_primary;
    visuals.widgets.active.fg_stroke.color = text_primary;
    visuals.selection.bg_fill = accent_cyan;
    visuals.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(0x19, 0x19, 0x19));
    visuals.text_cursor.stroke = egui::Stroke::new(2.5, text_primary);
    visuals.text_cursor.on_duration = 0.65;
    visuals.text_cursor.off_duration = 0.35;
    visuals.hyperlink_color = accent_cyan;
    visuals.warn_fg_color = text_muted;
    visuals.error_fg_color = text_muted;
    visuals.override_text_color = Some(text_primary);
    visuals.window_stroke = egui::Stroke::new(1.0, border_color);
  } else {
    let panel_fill = Color32::from_rgb(0xf3, 0xf4, 0xf6);
    let window_fill = Color32::from_rgb(0xff, 0xff, 0xff);
    let hovered_bg = Color32::from_rgb(0xe5, 0xe7, 0xeb);
    let text_primary = Color32::from_rgb(0x11, 0x18, 0x27);
    let text_muted = Color32::from_rgb(0x6b, 0x72, 0x80);
    let border_color = Color32::from_rgb(0xd1, 0xd5, 0xdb);
    let accent_cyan = Color32::from_rgb(0x25, 0x63, 0xeb);

    visuals.panel_fill = panel_fill;
    visuals.window_fill = window_fill;
    visuals.extreme_bg_color = panel_fill;
    visuals.faint_bg_color = window_fill;
    visuals.text_edit_bg_color = Some(panel_fill);
    visuals.widgets.noninteractive.bg_fill = window_fill;
    visuals.widgets.inactive.bg_fill = window_fill;
    visuals.widgets.inactive.weak_bg_fill = hovered_bg;
    visuals.widgets.hovered.bg_fill = hovered_bg;
    visuals.widgets.active.bg_fill = hovered_bg;
    visuals.widgets.open.bg_fill = hovered_bg;
    visuals.widgets.noninteractive.fg_stroke.color = text_muted;
    visuals.widgets.inactive.fg_stroke.color = text_muted;
    visuals.widgets.hovered.fg_stroke.color = text_primary;
    visuals.widgets.active.fg_stroke.color = text_primary;
    visuals.selection.bg_fill = accent_cyan;
    visuals.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(0xff, 0xff, 0xff));
    visuals.text_cursor.stroke = egui::Stroke::new(2.5, text_primary);
    visuals.text_cursor.on_duration = 0.65;
    visuals.text_cursor.off_duration = 0.35;
    visuals.hyperlink_color = accent_cyan;
    visuals.warn_fg_color = text_muted;
    visuals.error_fg_color = text_muted;
    visuals.override_text_color = Some(text_primary);
    visuals.window_stroke = egui::Stroke::new(1.0, border_color);
  }
  
  visuals.popup_shadow = egui::Shadow {
      offset: [0, 4], blur: 8, spread: 0,
      color: Color32::from_black_alpha(150),
  };
  visuals.window_shadow = egui::Shadow {
      offset: [0, 8], blur: 16, spread: 0,
      color: Color32::from_black_alpha(200),
  };
  
  ctx.set_visuals(visuals);

  let mut style = (*ctx.global_style()).clone();
  style.spacing.item_spacing = egui::vec2(6.0, 4.0);
  style.spacing.button_padding = egui::vec2(8.0, 4.0);
  style.visuals.widgets.inactive.corner_radius = 4.0.into();
  style.visuals.widgets.hovered.corner_radius = 4.0.into();
  style.visuals.widgets.active.corner_radius = 4.0.into();
  ctx.set_global_style(style);
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
        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
        egui::StrokeKind::Outside,
      );
    } else if response.hovered() {
      painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, ui.visuals().widgets.hovered.fg_stroke.color),
        egui::StrokeKind::Outside,
      );
    } else {
      painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
      );
    }
  }
  response
}

pub fn accent_color_picker_ui(ui: &mut egui::Ui, color: &mut Color32) -> bool {
  let mut changed = false;
  ui.label(RichText::new("Choose an accent color").small().color(ui.visuals().text_color()));
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
    GridColumn::Date => Some(GridColumn::Amount),
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
    GridColumn::Amount => Some(GridColumn::Date),
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
        ui.visuals().selection.bg_fill,
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
      if outer.width() < 80.0 || outer.height() < 80.0 {
        return true;
      }
      if let Some(monitor) = vp.monitor_size {
        if monitor.x > 1.0 && monitor.y > 1.0 {
          let monitor_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, monitor);
          return !monitor_rect.intersects(outer);
        }
      }
    } else {
      return true;
    }
    false
  })
}

pub fn ensure_root_window_visible(ctx: &egui::Context, force_center: bool) {
  ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
  if ctx.input(|i| i.viewport().minimized == Some(true)) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
  }
  if force_center || viewport_needs_reposition(ctx) {
    if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
      ctx.send_viewport_cmd(cmd);
    }
  }
  ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
  ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
    egui::UserAttentionType::Critical,
  ));
}

