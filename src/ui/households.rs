use crate::db::*;
use crate::models::*;
use crate::ui::widgets::*;
use crate::TwoCentsApp;
use eframe::egui::{self, Color32, Id, RichText};

const HOUSEHOLD_TWO_COLUMN_MIN_WIDTH: f32 = 900.0;
const HOUSEHOLD_BODY_CARD_MIN_HEIGHT: f32 = 460.0;
const HOUSEHOLD_SUPPORT_CARD_WIDTH: f32 = 300.0;

fn household_uses_two_columns(width: f32) -> bool {
    width >= HOUSEHOLD_TWO_COLUMN_MIN_WIDTH
}

impl TwoCentsApp {
    pub fn ui_households(&mut self, ui: &mut egui::Ui) {
        let mut save_self = false;
        let mut save_household = false;
        let mut delete_member: Option<i64> = None;
        let mut start_member_rename: Option<(i64, String)> = None;
        let mut commit_member_rename: Option<(i64, String)> = None;
        let mut cancel_member_rename = false;
        let mut status_message: Option<String> = None;
        let mut member_color_pick: Option<(i64, String, Color32)> = None;
        let household_id = self.household_id;
        let members = self.members.clone();

        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(0, 14))
            .show(ui, |ui| {
                let two_columns = household_uses_two_columns(ui.available_width());

                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        crate::ui::components::heading_lg(ui, "Household");
                        crate::ui::components::label_muted(
                            ui,
                            "Manage your household, members, and categories.",
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(HOUSEHOLD_SUPPORT_CARD_WIDTH, 0.0),
                            egui::Layout::top_down(egui::Align::Max),
                            |ui| {
                                household_panel(ui, "Support", |ui| {
                                    crate::ui::components::label_muted(
                                        ui,
                                        "TwoCents is a free personal project.",
                                    );
                                    ui.hyperlink_to(
                                        "Buy Me a Coffee",
                                        "https://www.buymeacoffee.com/Nucletheus",
                                    );
                                });
                            },
                        );
                    });
                });
                ui.add_space(crate::ui::theme_tokens::SPACE_4);

                let column_count = if two_columns { 2 } else { 1 };
                ui.columns(column_count, |columns| {
                    household_panel(&mut columns[0], "Household details", |ui| {
                        let card_width = ui.available_width();
                        ui.set_width(card_width);
                        if two_columns {
                            ui.set_min_height(HOUSEHOLD_BODY_CARD_MIN_HEIGHT);
                        }

                        crate::ui::components::section_header(ui, "Active household");
                        ui.add_space(crate::ui::theme_tokens::SPACE_2);
                        let name_field = grid_text_edit_cell(
                            ui,
                            &mut self.editing_household_name,
                            Id::new("editing_household_name"),
                            true,
                        );
                        if name_field.lost_focus() {
                            save_household = true;
                        }

                        ui.add_space(crate::ui::theme_tokens::SPACE_3);
                        ui.separator();
                        ui.add_space(crate::ui::theme_tokens::SPACE_2);
                        crate::ui::components::section_header(ui, "Household members");
                        ui.add_space(crate::ui::theme_tokens::SPACE_2);
                        egui::ScrollArea::vertical()
                            .id_salt(("household_members_list", self.household_id))
                            .max_height(140.0)
                            .show(ui, |ui| {
                                if members.is_empty() {
                                    ui.label(
                                        RichText::new("No members yet.")
                                            .color(crate::ui::theme::fg_secondary()),
                                    );
                                }
                                for member in members {
                                    ui.horizontal(|ui| {
                                        if color_swatch_button(ui, member.color).clicked() {
                                            member_color_pick = Some((
                                                member.id,
                                                member.name.clone(),
                                                member.color,
                                            ));
                                        }
                                        if member.is_self {
                                            let name_width = 180.0;
                                            let self_field = ui
                                                .allocate_ui_with_layout(
                                                    egui::vec2(
                                                        name_width,
                                                        ui.spacing().interact_size.y,
                                                    ),
                                                    egui::Layout::left_to_right(egui::Align::Center),
                                                    |ui| {
                                                        grid_text_edit_cell(
                                                            ui,
                                                            &mut self.editing_self_name,
                                                            Id::new("editing_self_name"),
                                                            true,
                                                        )
                                                    },
                                                )
                                                .inner;
                                            if self_field.lost_focus() {
                                                save_self = true;
                                            }
                                            ui.add_space(crate::ui::theme_tokens::SPACE_2);
                                            ui.label(
                                                RichText::new("Default")
                                                    .small()
                                                    .color(crate::ui::theme::fg_secondary()),
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new(&member.name)
                                                    .color(crate::ui::theme::fg_primary()),
                                            );
                                            if self.editing_member_id == Some(member.id) {
                                                let response = ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.editing_member_name,
                                                    )
                                                    .id(Id::new((
                                                        "editing_member_name",
                                                        member.id,
                                                    )))
                                                    .desired_width(130.0),
                                                );
                                                if response.lost_focus()
                                                    || (response.has_focus()
                                                        && ui.input(|input| {
                                                            input.key_pressed(egui::Key::Enter)
                                                        }))
                                                {
                                                    commit_member_rename = Some((
                                                        member.id,
                                                        self.editing_member_name.clone(),
                                                    ));
                                                }
                                                if ui.small_button("Cancel").clicked()
                                                    || ((response.has_focus()
                                                        || response.lost_focus())
                                                        && ui.input(|input| {
                                                            input.key_pressed(egui::Key::Escape)
                                                        }))
                                                {
                                                    cancel_member_rename = true;
                                                }
                                            } else {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if crate::ui::popups::styled_button(
                                                            ui,
                                                            "Remove",
                                                            false,
                                                        )
                                                        .clicked()
                                                        {
                                                            delete_member = Some(member.id);
                                                        }
                                                        if crate::ui::popups::styled_button(
                                                            ui,
                                                            "Rename",
                                                            false,
                                                        )
                                                        .clicked()
                                                        {
                                                            start_member_rename = Some((
                                                                member.id,
                                                                member.name.clone(),
                                                            ));
                                                        }
                                                    },
                                                );
                                            }
                                        }
                                    });
                                }
                            });

                        ui.add_space(crate::ui::theme_tokens::SPACE_3);
                        ui.separator();
                        ui.add_space(crate::ui::theme_tokens::SPACE_1);
                        let (add_field, add_clicked) = ui
                            .with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                                let add_field = ui.add(
                                    egui::TextEdit::singleline(&mut self.new_member_name)
                                        .id_salt("new_household_member")
                                        .desired_width(220.0)
                                        .margin(egui::Margin::symmetric(4, 2))
                                        .text_color(crate::ui::theme::fg_primary())
                                        .background_color(crate::ui::theme::bg_primary()),
                                );
                                ui.add_space(crate::ui::theme_tokens::SPACE_1);
                                let add_clicked =
                                    crate::ui::popups::styled_button(ui, "Add", true).clicked();
                                ui.add_space(crate::ui::theme_tokens::SPACE_1);
                                (add_field, add_clicked)
                            })
                            .inner;
                        let enter_submit = add_field.lost_focus()
                            && ui.input(|input| input.key_pressed(egui::Key::Enter));
                        if add_clicked || enter_submit {
                            let pending_name = self.new_member_name.trim().to_string();
                            match self.try_add_household_member(&pending_name) {
                                Ok(()) => status_message = Some(format!("Added '{pending_name}'.")),
                                Err(err) => status_message = Some(err),
                            }
                            add_field.request_focus();
                        }
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                            egui::Layout::right_to_left(egui::Align::Min),
                            |ui| {
                                ui.label(
                                    RichText::new("Type a name, press Enter, or click Add.")
                                        .small()
                                        .color(crate::ui::theme::fg_secondary()),
                                );
                            },
                        );

                        if let Some(message) = &status_message {
                            ui.add_space(crate::ui::theme_tokens::SPACE_2);
                            ui.label(RichText::new(message).color(crate::ui::theme::accent()));
                        }
                    });

                    let categories_ui = if two_columns {
                        columns.get_mut(1)
                    } else {
                        columns.get_mut(0)
                    };
                    if let Some(categories_ui) = categories_ui {
                        household_panel(categories_ui, "Categories", |ui| {
                            let card_width = ui.available_width();
                            ui.set_width(card_width);
                            if two_columns {
                                ui.set_min_height(HOUSEHOLD_BODY_CARD_MIN_HEIGHT);
                            }
                            crate::ui::components::label_muted(
                                ui,
                                "Categories marked \"Excluded\" are left out of budgets, analytics, and settlements.",
                            );
                            ui.add_space(crate::ui::theme_tokens::SPACE_2);
                            self.ui_category_settings_section(ui);
                        });
                    }
                });
            });

        if let Some((id, name, color)) = member_color_pick {
            self.open_member_color_popup(id, name, color);
        }

        if save_household {
            let name = self.editing_household_name.trim().to_string();
            if !name.is_empty() {
                let result = self.run_household_mutation(
                    |tx| update_household_name(tx, household_id, &name),
                    |before, after| UndoAction::HouseholdRename { before, after },
                );
                match result {
                    Ok(true) => {
                        self.household_name = name.clone();
                        self.editing_household_name = name;
                        self.set_transient_status("Household renamed.");
                    }
                    Ok(false) => {}
                    Err(error) => self.set_transient_status(error),
                }
            }
        }
        if save_self {
            let name = self.editing_self_name.trim().to_string();
            if !name.is_empty() {
                let result = self.run_household_mutation(
                    |tx| update_self_member_name(tx, household_id, &name),
                    |before, after| UndoAction::MemberRename { before, after },
                );
                match result {
                    Ok(true) => self.set_transient_status("Member renamed."),
                    Ok(false) => {}
                    Err(error) => self.set_transient_status(error),
                }
            }
        }
        if let Some((member_id, name)) = start_member_rename {
            self.editing_member_id = Some(member_id);
            self.editing_member_name = name;
            ui.ctx().memory_mut(|memory| {
                memory.request_focus(Id::new(("editing_member_name", member_id)))
            });
        }
        if let Some((member_id, name)) = commit_member_rename {
            if cancel_member_rename {
                self.editing_member_id = None;
                self.editing_member_name.clear();
            } else {
                match self.rename_member_with_undo(member_id, &name) {
                    Ok(()) => {
                        self.editing_member_id = None;
                        self.editing_member_name.clear();
                    }
                    Err(error) => self.set_transient_status(error),
                }
            }
        } else if cancel_member_rename {
            self.editing_member_id = None;
            self.editing_member_name.clear();
        }
        if let Some(member_id) = delete_member {
            let member = self
                .members
                .iter()
                .find(|member| member.id == member_id)
                .map(|member| (member.id, member.name.clone()));
            if let Some((id, name)) = member {
                self.open_destructive_confirmation(DestructiveConfirm::Member { id, name });
            }
        }
    }

    pub(crate) fn rename_member_with_undo(
        &mut self,
        member_id: i64,
        name: &str,
    ) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Enter a member name first.".to_string());
        }
        let household_id = self.household_id;
        let result = self.run_household_mutation(
            |tx| rename_household_member(tx, household_id, member_id, &name),
            |before, after| UndoAction::MemberRename { before, after },
        )?;
        if result {
            self.set_transient_status("Member renamed.");
        }
        Ok(())
    }

    pub(crate) fn delete_member_with_undo(&mut self, member_id: i64) -> Result<(), String> {
        let household_id = self.household_id;
        let result = self.run_household_mutation(
            |tx| delete_household_member(tx, household_id, member_id),
            |before, after| UndoAction::MemberDelete { before, after },
        )?;
        if result {
            self.set_transient_status("Member removed.");
        }
        Ok(())
    }

    fn try_add_household_member(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a member name first.".to_string());
        }
        let household_id = self.household_id;
        let result = self.run_household_mutation(
            |tx| add_household_member(tx, household_id, name),
            |before, after| UndoAction::MemberAdd { before, after },
        )?;
        if !result {
            return Err("Member was not added.".to_string());
        }
        self.new_member_name.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{household_uses_two_columns, HOUSEHOLD_TWO_COLUMN_MIN_WIDTH};

    #[test]
    fn household_columns_switch_at_configured_width() {
        assert!(!household_uses_two_columns(
            HOUSEHOLD_TWO_COLUMN_MIN_WIDTH - 1.0
        ));
        assert!(household_uses_two_columns(HOUSEHOLD_TWO_COLUMN_MIN_WIDTH));
    }
}
