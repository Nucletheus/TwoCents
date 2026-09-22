use eframe::egui::{self, Color32, RichText, Id};
use crate::models::*;
use crate::db::*;
use crate::ui::widgets::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_households(&mut self, ui: &mut egui::Ui) {
    let mut save_self = false;
    let mut delete_member: Option<i64> = None;
    let mut status_message: Option<String> = None;
    let mut member_color_pick: Option<(i64, String, Color32)> = None;
    let self_member = self.members.iter().find(|member| member.is_self).cloned();
    let other_members: Vec<HouseholdMember> = self
      .members
      .iter()
      .filter(|member| !member.is_self)
      .cloned()
      .collect();

    egui::Frame::new()
      .inner_margin(egui::Margin::symmetric(20, 14))
      .show(ui, |ui| {
        ui.set_max_width(560.0);
        // ponytail: heading + muted description, Notion hierarchy.
        crate::ui::components::heading_lg(ui, "Households");
        crate::ui::components::label_muted(
          ui,
          "Manage your household, members, and member colors.",
        );
        ui.add_space(crate::ui::theme_tokens::SPACE_4);

        household_panel(ui, "Active household", |ui| {
          ui.label(RichText::new(&self.household_name).size(16.0).color(crate::ui::components::fg_default(ui)));
        });

        household_panel(ui, "You", |ui| {
          ui.label(
            RichText::new("Default member for new expenses")
              .small()
              .color(crate::ui::components::fg_muted(ui)),
          );
          ui.add_space(4.0);
          ui.horizontal(|ui| {
            if let Some(member) = &self_member {
              if color_swatch_button(ui, member.color).clicked() {
                member_color_pick = Some((member.id, member.name.clone(), member.color));
              }
            }
            let self_field =
              grid_text_edit_cell(ui, &mut self.editing_self_name, Id::new("editing_self_name"), true);
            if self_field.lost_focus() {
              save_self = true;
            }
          });
        });

        household_panel(ui, "Household members", |ui| {
          egui::ScrollArea::vertical()
            .id_salt(("household_members_list", self.household_id))
            .max_height(140.0)
            .show(ui, |ui| {
              if other_members.is_empty() {
                ui.label(RichText::new("No other members yet.").color(crate::ui::components::fg_muted(ui)));
              }
              for member in other_members {
                ui.horizontal(|ui| {
                  if color_swatch_button(ui, member.color).clicked() {
                    member_color_pick = Some((member.id, member.name.clone(), member.color));
                  }
                  ui.label(RichText::new(&member.name).color(crate::ui::components::fg_default(ui)));
                  ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::popups::styled_button(ui, "Remove", false).clicked() {
                      delete_member = Some(member.id);
                    }
                  });
                });
                ui.separator();
              }
            });
        });

        let mut refocus_add_member = false;
        household_panel(ui, "Add member", |ui| {
          let add_row = ui.horizontal(|ui| {
            let field_width = (ui.available_width() - 104.0).max(120.0);
            let add_field = ui.add(
              egui::TextEdit::singleline(&mut self.new_member_name)
                .id_salt("new_household_member")
                .desired_width(field_width)
                .margin(egui::Margin::symmetric(4, 2))
                .text_color(crate::ui::components::fg_default(ui))
                .background_color(ui.visuals().panel_fill),
            );
            let add_clicked = ui
              .add(
                egui::Button::new(RichText::new("Add").color(crate::ui::theme::contrast_text(crate::ui::components::accent_color(ui))))
                  .fill(crate::ui::components::accent_color(ui))
                  .min_size(egui::vec2(96.0, 30.0)),
              )
              .clicked();
            (add_field, add_clicked)
          });
          let add_field = &add_row.inner.0;
          let enter_submit =
            add_field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
          let add_clicked = add_row.inner.1;
          if add_clicked || enter_submit {
            let pending_name = self.new_member_name.trim().to_string();
            match self.try_add_household_member(&pending_name) {
              Ok(()) => status_message = Some(format!("Added '{pending_name}'.")),
              Err(err) => {
                                status_message = Some(err);
              }
            }
            refocus_add_member = true;
          }
          if refocus_add_member {
            add_field.request_focus();
          }
          ui.label(
            RichText::new("Type a name, press Enter, or click Add.")
              .small()
              .color(crate::ui::components::fg_muted(ui)),
          );
        });

        if let Some(message) = &status_message {
          ui.add_space(8.0);
          ui.label(RichText::new(message).color(crate::ui::components::accent_color(ui)));
        }
      });

    if let Some((id, name, color)) = member_color_pick {
      self.open_member_color_popup(id, name, color);
    }

    if save_self {
      let name = self.editing_self_name.trim().to_string();
      if !name.is_empty() {
        match update_self_member_name(&self.conn, self.household_id, &name) {
          Ok(()) => {
            self.reload();
                      }
          Err(_err) => {}
        }
      }
    }
    if let Some(member_id) = delete_member {
      match delete_household_member(&self.conn, self.household_id, member_id) {
        Ok(()) => {
          self.reload();
                  }
        Err(_err) => {}
      }
    }
  }

  fn try_add_household_member(&mut self, name: &str) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() {
      return Err("Enter a member name first.".to_string());
    }
    add_household_member(&self.conn, self.household_id, name).map_err(|err| err.to_string())?;
    self.new_member_name.clear();
    self.reload();
        Ok(())
  }
}
