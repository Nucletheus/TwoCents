use eframe::egui::{self, Color32, Id, RichText};
use std::collections::HashSet;
use crate::models::*;
use crate::db::*;
use crate::ui::widgets::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn member_color(&self, name: &str) -> Color32 {
    let trimmed = name.trim();
    if trimmed.is_empty() {
      return Color32::TRANSPARENT;
    }
    self
      .members
      .iter()
      .find(|member| member.name.eq_ignore_ascii_case(trimmed))
      .map(|member| member.color)
      .unwrap_or(Color32::TRANSPARENT)
  }

  fn set_member_color(&mut self, member_id: i64, color: Color32) {
    match update_member_color_db(&self.conn, self.household_id, member_id, color) {
      Ok(()) => {
        if let Some(member) = self.members.iter_mut().find(|member| member.id == member_id) {
          member.color = color;
        }
        self.log(format!(
          "[household] updated color for '{}'",
          self
            .members
            .iter()
            .find(|member| member.id == member_id)
            .map(|member| member.name.as_str())
            .unwrap_or("member")
        ));
      }
      Err(err) => self.log(format!("[error] member color save failed: {err}")),
    }
  }

  pub fn open_member_color_popup(&mut self, id: i64, name: String, color: Color32) {
    self.member_color_popup = Some(MemberColorPopup { id, name, color });
  }

  pub fn ui_member_color_popup(&mut self, ctx: &egui::Context) {
    let mut popup_state = self.member_color_popup.clone();
    let mut open = popup_state.is_some();
    let mut color_changed: Option<(i64, Color32)> = None;
    if let Some(popup) = &mut popup_state {
      let title = format!("Member color: {}", popup.name);
      egui::Window::new(title)
        .id(Id::new("member_color_popup"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(300.0)
        .frame(themed_modal_frame(ctx))
        .show(ctx, |ui| {
          if accent_color_picker_ui(ui, &mut popup.color) {
            color_changed = Some((popup.id, popup.color));
          }
        });
    }
    if let Some((id, color)) = color_changed {
      self.set_member_color(id, color);
    }
    self.member_color_popup = if open { popup_state } else { None };
  }

  pub fn category_color(&self, label: &str) -> Color32 {
    let trimmed = label.trim();
    if trimmed.is_empty() {
      return Color32::TRANSPARENT;
    }
    find_category_by_label(&self.categories, trimmed)
      .map(|category| category.color)
      .unwrap_or(Color32::TRANSPARENT)
  }

  fn set_category_color(&mut self, category_id: i64, color: Color32) {
    let is_parent = self
      .categories
      .iter()
      .find(|category| category.id == category_id)
      .is_some_and(|category| category.parent_id.is_none());

    match update_category_color_db(&self.conn, self.household_id, category_id, color) {
      Ok(()) => {
        if let Some(category) = self.categories.iter_mut().find(|category| category.id == category_id) {
          category.color = color;
        }
        if is_parent {
          self.recolor_subcategories_for_parent(category_id, color);
        }
        self.chart_dirty = true;
      }
      Err(err) => self.log(format!("[error] category color save failed: {err}")),
    }
  }

  fn recolor_subcategories_for_parent(&mut self, parent_id: i64, parent_color: Color32) {
    let mut sub_ids: Vec<i64> = self
      .categories
      .iter()
      .filter(|category| category.parent_id == Some(parent_id))
      .map(|category| category.id)
      .collect();
    sub_ids.sort_by(|a, b| {
      let name_a = self
        .categories
        .iter()
        .find(|category| category.id == *a)
        .map(|category| category.name.as_str())
        .unwrap_or("");
      let name_b = self
        .categories
        .iter()
        .find(|category| category.id == *b)
        .map(|category| category.name.as_str())
        .unwrap_or("");
      name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });
    let sub_count = sub_ids.len().max(1);
    for (sub_index, sub_id) in sub_ids.iter().enumerate() {
      let sub_color = subcategory_color_from_parent(parent_color, sub_index, sub_count);
      match update_category_color_db(&self.conn, self.household_id, *sub_id, sub_color) {
        Ok(()) => {
          if let Some(sub) = self.categories.iter_mut().find(|category| category.id == *sub_id) {
            sub.color = sub_color;
          }
        }
        Err(err) => self.log(format!("[error] subcategory color save failed: {err}")),
      }
    }
  }

  fn delete_category(&mut self, category_id: i64) {
    match delete_category_from_db(&self.conn, self.household_id, category_id) {
      Ok(()) => {
        self.expenses = load_expenses(&self.conn, self.household_id).unwrap_or_default();
        self.reload_categories();
        self.chart_dirty = true;
        self.log("[categories] deleted category and cleared matching expenses");
      }
      Err(err) => self.log(format!("[error] delete category failed: {err}")),
    }
  }

  fn add_parent_category(&mut self, name: &str) {
    let name = name.trim();
    if name.is_empty() {
      return;
    }
    match add_parent_category_db(&self.conn, self.household_id, name) {
      Ok(()) => {
        self.reload_categories();
        self.log(format!("[categories] added parent '{name}'"));
      }
      Err(err) => self.log(format!("[error] add parent category failed: {err}")),
    }
  }

  fn add_subcategory(&mut self, parent_id: i64, name: &str) {
    let name = name.trim();
    if name.is_empty() {
      return;
    }
    match add_subcategory_db(&self.conn, self.household_id, parent_id, name) {
      Ok(()) => {
        self.reload_categories();
        self.log(format!("[categories] added subcategory '{name}'"));
      }
      Err(err) => self.log(format!("[error] add subcategory failed: {err}")),
    }
  }

  pub fn open_category_color_popup(&mut self, category_id: i64, name: String, color: Color32) {
    self.category_color_popup = Some(CategoryColorPopup { id: category_id, name, color });
  }

  fn ui_category_settings_list(&mut self, ui: &mut egui::Ui) {
    let categories = self.categories.clone();
    let parents = category_parent_map(&categories);
    let parent_ids = sorted_parent_category_ids(&categories);

    let mut delete_target: Option<i64> = None;
    let mut add_sub_to_parent: Option<i64> = None;

    for parent_id in parent_ids {
      let Some(parent) = categories.iter().find(|category| category.id == parent_id) else {
        continue;
      };
      ui.horizontal(|ui| {
        if color_swatch_button(ui, parent.color).clicked() {
          self.open_category_color_popup(parent.id, parent.name.clone(), parent.color);
        }
        ui.label(RichText::new(&parent.name).strong().color(ui.visuals().selection.bg_fill));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
          if ui
            .add(
              egui::Button::new(RichText::new("Delete").small().color(ui.visuals().text_color()))
                .fill(ui.visuals().widgets.hovered.bg_fill)
                .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)),
            )
            .clicked()
          {
            delete_target = Some(parent.id);
          }
          if ui
            .add(
              egui::Button::new(RichText::new("+ Sub").small().color(ui.visuals().text_color()))
                .fill(ui.visuals().widgets.hovered.bg_fill)
                .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)),
            )
            .clicked()
          {
            add_sub_to_parent = Some(parent.id);
          }
        });
      });

      let mut sub_ids: Vec<i64> = categories
        .iter()
        .filter(|category| category.parent_id == Some(parent_id))
        .map(|category| category.id)
        .collect();
      sub_ids.sort_by(|a, b| {
        let name_a = categories.iter().find(|c| c.id == *a).map(|c| c.name.as_str()).unwrap_or("");
        let name_b = categories.iter().find(|c| c.id == *b).map(|c| c.name.as_str()).unwrap_or("");
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
      });

      for sub_id in sub_ids {
        let Some(sub) = categories.iter().find(|category| category.id == sub_id) else {
          continue;
        };
        ui.horizontal(|ui| {
          ui.add_space(20.0);
          if color_swatch_button(ui, sub.color).clicked() {
            self.open_category_color_popup(sub.id, sub.full_label(&parents), sub.color);
          }
          ui.label(RichText::new(&sub.name).color(ui.visuals().text_color()).small());
          ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
              .add(
                egui::Button::new(RichText::new("Delete").small().color(ui.visuals().text_color()))
                  .fill(ui.visuals().widgets.hovered.bg_fill)
                  .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)),
              )
              .clicked()
            {
              delete_target = Some(sub.id);
            }
          });
        });
      }
      ui.separator();
    }

    if let Some(parent_id) = add_sub_to_parent {
      self.new_subcategory_parent_id = Some(parent_id);
    }
    if let Some(category_id) = delete_target {
      self.delete_category(category_id);
    }
  }

  pub fn ui_category_settings_window(&mut self, ctx: &egui::Context) {
    if !self.show_category_settings {
      return;
    }
    let mut open = true;
    let mut add_parent = false;
    let mut add_sub = false;
    egui::Window::new("Settings")
      .id(Id::new("category_settings_window"))
      .open(&mut open)
      .collapsible(false)
      .resizable(true)
      .default_width(400.0)
      .frame(themed_modal_frame(ctx))
      .show(ctx, |ui| {
        ui.label(RichText::new("Settings").strong().color(ui.visuals().strong_text_color()));
        ui.label(
          RichText::new("Create and delete categories here. The expense grid only uses these.")
            .small()
            .color(ui.visuals().text_color()),
        );
        ui.add_space(6.0);
        themed_panel_frame(ui.style()).show(ui, |ui| {
          ui.label(RichText::new("Add parent category").strong().color(ui.visuals().selection.bg_fill));
          ui.horizontal(|ui| {
            let parent_name_response = ui.add(
              egui::TextEdit::singleline(&mut self.new_parent_category_name)
                .id(settings_new_parent_name_id())
                .hint_text("e.g. Home — press Enter")
                .desired_width(220.0),
            );
            add_parent =
              ui.button("Add parent").clicked() || text_field_enter_pressed(ui, &parent_name_response);
          });
          ui.add_space(8.0);
          ui.label(RichText::new("Add subcategory").strong().color(ui.visuals().selection.bg_fill));
          ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("new_subcategory_parent")
              .selected_text(
                self
                  .new_subcategory_parent_id
                  .and_then(|id| self.categories.iter().find(|c| c.id == id))
                  .map(|c| c.name.clone())
                  .unwrap_or_else(|| "Select parent…".to_string()),
              )
              .show_ui(ui, |ui| {
                egui::ScrollArea::vertical()
                  .id_salt("new_subcategory_parent_scroll")
                  .max_height(200.0)
                  .show(ui, |ui| {
                    let parents = self.categories.clone();
                    for parent_id in sorted_parent_category_ids(&parents) {
                      let Some(parent) = parents.iter().find(|category| category.id == parent_id) else {
                        continue;
                      };
                      if ui
                        .selectable_label(
                          self.new_subcategory_parent_id == Some(parent.id),
                          &parent.name,
                        )
                        .clicked()
                      {
                        self.new_subcategory_parent_id = Some(parent.id);
                      }
                    }
                  });
              });
            let sub_name_response = ui.add(
              egui::TextEdit::singleline(&mut self.new_subcategory_name)
                .id(settings_new_sub_name_id())
                .hint_text("e.g. Mortgage — press Enter")
                .desired_width(140.0),
            );
            add_sub = ui.button("Add sub").clicked() || text_field_enter_pressed(ui, &sub_name_response);
          });
          ui.add_space(8.0);
          ui.separator();
          ui.label(RichText::new("Your categories").strong().color(ui.visuals().selection.bg_fill));
          egui::ScrollArea::vertical()
            .id_salt("category_settings_scroll")
            .auto_shrink([false, false])
            .max_height(320.0)
            .show(ui, |ui| {
              self.ui_category_settings_list(ui);
            });
        });
      });
    if add_parent {
      let name = self.new_parent_category_name.trim().to_string();
      if !name.is_empty() {
        self.add_parent_category(&name);
        self.new_parent_category_name.clear();
        ctx.memory_mut(|mem| mem.request_focus(settings_new_parent_name_id()));
      }
    }
    if add_sub {
      if let Some(parent_id) = self.new_subcategory_parent_id {
        let name = self.new_subcategory_name.trim().to_string();
        if !name.is_empty() {
          self.add_subcategory(parent_id, &name);
          self.new_subcategory_name.clear();
          ctx.memory_mut(|mem| mem.request_focus(settings_new_sub_name_id()));
        }
      } else {
        self.log("[categories] pick a parent before adding a subcategory");
        ctx.memory_mut(|mem| mem.request_focus(settings_new_sub_name_id()));
      }
    }
    self.show_category_settings = open;
  }

  pub fn ui_category_color_popup(&mut self, ctx: &egui::Context) {
    let mut popup_state = self.category_color_popup.clone();
    let mut open = popup_state.is_some();
    let mut color_changed: Option<(i64, Color32)> = None;
    if let Some(popup) = &mut popup_state {
      let title = format!("Color: {}", popup.name);
      egui::Window::new(title)
        .id(Id::new("category_color_popup"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(280.0)
        .frame(themed_modal_frame(ctx))
        .show(ctx, |ui| {
          if accent_color_picker_ui(ui, &mut popup.color) {
            color_changed = Some((popup.id, popup.color));
          }
        });
    }
    if let Some((id, color)) = color_changed {
      self.set_category_color(id, color);
    }
    self.category_color_popup = if open { popup_state } else { None };
  }

  pub fn ui_category_picker_menu(&self, ui: &mut egui::Ui, current: &str) -> Option<String> {
    let categories = &self.categories;
    let parents = category_parent_map(categories);
    let mut parent_ids: Vec<i64> = categories
      .iter()
      .filter(|category| category.parent_id.is_none())
      .map(|category| category.id)
      .collect();
    parent_ids.sort_by(|a, b| {
      let name_a = categories.iter().find(|c| c.id == *a).map(|c| c.name.as_str()).unwrap_or("");
      let name_b = categories.iter().find(|c| c.id == *b).map(|c| c.name.as_str()).unwrap_or("");
      name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });

    let mut picked: Option<String> = None;
    let mut parent_has_children = HashSet::new();
    for category in categories {
      if let Some(parent_id) = category.parent_id {
        parent_has_children.insert(parent_id);
      }
    }

    for parent_id in parent_ids {
      let Some(parent) = categories.iter().find(|category| category.id == parent_id) else {
        continue;
      };
      if !parent_has_children.contains(&parent.id) {
        let label = parent.name.clone();
        if category_picker_row(
          ui,
          &label,
          parent.color,
          current.eq_ignore_ascii_case(&label),
          0.0,
        ) {
          picked = Some(label);
          ui.close();
        }
      } else {
        category_picker_header_row(ui, &parent.name, parent.color);
        let mut sub_ids: Vec<i64> = categories
          .iter()
          .filter(|category| category.parent_id == Some(parent_id))
          .map(|category| category.id)
          .collect();
        sub_ids.sort_by(|a, b| {
          let name_a = categories.iter().find(|c| c.id == *a).map(|c| c.name.as_str()).unwrap_or("");
          let name_b = categories.iter().find(|c| c.id == *b).map(|c| c.name.as_str()).unwrap_or("");
          name_a.to_lowercase().cmp(&name_b.to_lowercase())
        });
        for sub_id in sub_ids {
          let Some(sub) = categories.iter().find(|category| category.id == sub_id) else {
            continue;
          };
          let label = sub.full_label(&parents);
          if category_sub_picker_row(ui, &sub.name, current.eq_ignore_ascii_case(&label), 16.0) {
            picked = Some(label);
            ui.close();
          }
        }
        ui.separator();
      }
    }
    picked
  }

  pub fn ui_delete_confirmations(&mut self, ctx: &egui::Context) {
    if self.show_delete_expense_confirm {
      let mut open = self.show_delete_expense_confirm;
      let row_count = self.delete_expense_indices.len();
      
      egui::Window::new("Confirm Deletion")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_width(320.0)
        .frame(
          egui::Frame::window(&ctx.global_style())
            .fill(ctx.global_style().visuals.panel_fill)
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(239, 68, 68)))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(16, 16)),
        )
        .show(ctx, |ui| {
          ui.vertical_centered(|ui| {
            ui.label(
              RichText::new("⚠ Warning")
                .font(egui::FontId::proportional(18.0))
                .strong()
                .color(egui::Color32::from_rgb(239, 68, 68)),
            );
            ui.add_space(8.0);
            ui.label(
              RichText::new(format!(
                "Are you sure you want to permanently delete the selected {} expense row(s)?",
                row_count
              ))
              .font(egui::FontId::proportional(14.0))
              .color(ui.visuals().text_color()),
            );
            ui.add_space(6.0);
            ui.label(
              RichText::new("This action cannot be undone.")
                .font(egui::FontId::proportional(11.0))
                .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(16.0);
            
            ui.horizontal(|ui| {
              ui.columns(2, |cols| {
                cols[0].vertical_centered(|ui| {
                  if ui.button("Cancel").clicked() {
                    self.show_delete_expense_confirm = false;
                    self.delete_expense_indices.clear();
                  }
                });
                cols[1].vertical_centered(|ui| {
                  let confirm_btn = egui::Button::new(
                    RichText::new("Delete")
                      .strong()
                      .color(egui::Color32::WHITE),
                  )
                  .fill(egui::Color32::from_rgb(239, 68, 68));
                  if ui.add(confirm_btn).clicked() {
                    self.show_delete_expense_confirm = false;
                    let targets = std::mem::take(&mut self.delete_expense_indices);
                    self.delete_expenses_by_indices(&targets);
                  }
                });
              });
            });
          });
        });
      self.show_delete_expense_confirm = open;
    }

    if self.show_delete_import_confirm {
      let mut open = self.show_delete_import_confirm;
      let row_count = self.delete_import_indices.len();
      
      egui::Window::new("Remove staged rows?")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_width(320.0)
        .frame(
          egui::Frame::window(&ctx.global_style())
            .fill(ctx.global_style().visuals.panel_fill)
            .stroke(egui::Stroke::new(2.0, ctx.global_style().visuals.selection.bg_fill))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::symmetric(16, 16)),
        )
        .show(ctx, |ui| {
          ui.vertical_centered(|ui| {
            ui.label(
              RichText::new("Remove staged rows")
                .font(egui::FontId::proportional(18.0))
                .strong()
                .color(ui.visuals().selection.bg_fill),
            );
            ui.add_space(8.0);
            ui.label(
              RichText::new(format!(
                "Are you sure you want to remove the selected {} row(s) from the import review list?",
                row_count
              ))
              .font(egui::FontId::proportional(14.0))
              .color(ui.visuals().text_color()),
            );
            ui.add_space(6.0);
            ui.label(
              RichText::new("These rows will not be saved into your database.")
                .font(egui::FontId::proportional(11.0))
                .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(16.0);
            
            ui.horizontal(|ui| {
              ui.columns(2, |cols| {
                cols[0].vertical_centered(|ui| {
                  if ui.button("Cancel").clicked() {
                    self.show_delete_import_confirm = false;
                    self.delete_import_indices.clear();
                  }
                });
                cols[1].vertical_centered(|ui| {
                  let confirm_btn = egui::Button::new(
                    RichText::new("Remove")
                      .strong()
                      .color(egui::Color32::WHITE),
                  )
                  .fill(ui.visuals().selection.bg_fill);
                  if ui.add(confirm_btn).clicked() {
                    self.show_delete_import_confirm = false;
                    let targets = std::mem::take(&mut self.delete_import_indices);
                    self.delete_import_rows_by_indices(&targets);
                  }
                });
              });
            });
          });
        });
      self.show_delete_import_confirm = open;
    }
  }
}

fn settings_new_parent_name_id() -> Id {
  Id::new("settings_new_parent_category_name")
}

fn settings_new_sub_name_id() -> Id {
  Id::new("settings_new_subcategory_name")
}

fn grid_cell_active(response: &egui::Response) -> bool {
  response.has_focus() || response.lost_focus()
}

fn text_field_enter_pressed(ui: &egui::Ui, response: &egui::Response) -> bool {
  if !grid_cell_active(response) {
    return false;
  }
  ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
}
