use eframe::egui::{self, Color32};
use std::collections::HashSet;

use super::state::*;
use crate::models::HouseholdMember;
use chrono::Datelike;

fn to_picker_date(date: chrono::NaiveDate) -> jiff::civil::Date {
    jiff::civil::Date::new(date.year() as i16, date.month() as i8, date.day() as i8)
        .unwrap_or_else(|_| jiff::civil::Date::constant(1970, 1, 1))
}

fn from_picker_date(date: jiff::civil::Date) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(date.year() as i32, date.month() as u32, date.day() as u32)
        .unwrap_or_else(|| chrono::Local::now().date_naive())
}

/// The analytics preset strip, reused verbatim by Settlements. Takes the
/// three date fields directly so any tab can own its own range. Returns true
/// when the caller should persist the state.
pub fn date_range_row(
    ui: &mut egui::Ui,
    preset: &mut DatePreset,
    start: &mut Option<chrono::NaiveDate>,
    end: &mut Option<chrono::NaiveDate>,
) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        crate::ui::components::label_strong(ui, "Date Range:");

        let presets = [
            DatePreset::ThisMonth,
            DatePreset::LastMonth,
            DatePreset::Last3Months,
            DatePreset::Last6Months,
            DatePreset::YTD,
            DatePreset::LastYear,
            DatePreset::AllTime,
            DatePreset::Custom,
        ];

        for option in presets {
            let selected = *preset == option;
            if crate::ui::components::tab_label_button(ui, selected, option.label()).clicked() {
                *preset = option;
                let (new_start, new_end) = option.date_range();
                *start = new_start;
                *end = new_end;
                changed = true;
            }
        }
    });

    // Custom date range (only shown when Custom is selected)
    if *preset == DatePreset::Custom {
        // Custom reached with no stored dates (e.g. via
        // AllTime → Custom) used to leave Start/End unusable — the
        // fields didn't render. Seed a sane default window instead.
        if start.is_none() && end.is_none() {
            let today = chrono::Local::now().date_naive();
            *start = Some(today - chrono::Duration::days(30));
            *end = Some(today);
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.add_space(crate::ui::theme_tokens::SPACE_2);
            ui.label("Start:");
            if let Some(start_value) = start {
                let mut picked = to_picker_date(*start_value);
                if crate::ui::widgets::date_picker_button(
                    ui,
                    &mut picked,
                    egui::Id::new("range_start"),
                )
                .changed()
                {
                    *start_value = from_picker_date(picked);
                    changed = true;
                }
            }

            ui.add_space(crate::ui::theme_tokens::SPACE_2);
            ui.label("End:");
            if let Some(end_value) = end {
                let mut picked = to_picker_date(*end_value);
                if crate::ui::widgets::date_picker_button(
                    ui,
                    &mut picked,
                    egui::Id::new("range_end"),
                )
                .changed()
                {
                    *end_value = from_picker_date(picked);
                    changed = true;
                }
            }
        });
    }
    changed
}

pub fn render_filter_panel(
    ui: &mut egui::Ui,
    state: &mut AnalyticsState,
    members: &[HouseholdMember],
    available_vendors: &[String],
    show_date_range: bool,
) -> bool {
    let mut changed = false;

    // row 1 — date preset buttons (themed). User picks a
    // preset and the start/end dates are computed. The "Custom" option
    // reveals a free-form date range below. Hidden on the Period
    // Comparison tab, whose dates come from its own A/B combos.
    if show_date_range {
        changed = date_range_row(
            ui,
            &mut state.date_preset,
            &mut state.date_start,
            &mut state.date_end,
        );
    }

    // Row 2: filter dropdowns. The Clear button is the only "Clear" in
    // the analytics page. It resets all filter state (categories, members,
    // vendors) to defaults and the date range to "Last 3 Months".
    ui.horizontal_wrapped(|ui| {
        // no category dropdown here — it matched `c.name` against
        // a set of full "Parent › Sub" labels (phantom "N selected", and
        // checked entries the filter never matched). Category selection is
        // the shared picker on each chart tab.

        // Member filter with colors
        let member_names: Vec<String> = members.iter().map(|m| m.name.clone()).collect();
        let member_colors: Vec<Color32> = members.iter().map(|m| m.color).collect();
        ui.label("Members:");
        if multi_select_dropdown_with_colors(
            ui,
            "members_dropdown",
            &member_names,
            Some(&member_colors),
            &mut state.selected_members,
        ) {
            changed = true;
        }

        ui.separator();

        // Vendor filter (no colors)
        ui.label("Vendors:");
        if multi_select_dropdown_with_colors(
            ui,
            "vendors_dropdown",
            available_vendors,
            None,
            &mut state.selected_vendors,
        ) {
            changed = true;
        }

        if crate::ui::popups::styled_button(ui, "Clear filters", false).clicked() {
            state.date_preset = DatePreset::Last3Months;
            let (start, end) = DatePreset::Last3Months.date_range();
            state.date_start = start;
            state.date_end = end;
            state.selected_categories.clear();
            state.selected_members.clear();
            state.selected_vendors.clear();
            changed = true;
        }

        ui.add_space(crate::ui::theme_tokens::SPACE_3);

        // "Reset View" restores every chart's zoom/pan to its
        // default bounds on the next draw (handled inside each chart's
        // plot closure via state.reset_view). Kept separate from "Clear
        // filters" so the user can re-frame a chart without losing their
        // filter selections.
        if crate::ui::popups::styled_button(ui, "Reset View", false).clicked() {
            state.reset_view = true;
            changed = true;
        }
    });

    changed
}

fn multi_select_dropdown_with_colors(
    ui: &mut egui::Ui,
    id: &str,
    options: &[String],
    colors: Option<&[Color32]>,
    selected: &mut HashSet<String>,
) -> bool {
    let mut changed = false;

    let label = if selected.is_empty() {
        "All".to_string()
    } else {
        format!("{} selected", selected.len())
    };

    let popup_id = egui::Id::new(id);

    // Check if popup is open
    let is_open = ui.data(|data| data.get_temp::<bool>(popup_id).unwrap_or(false));

    // Create the button
    let button_response = ui.button(label);

    // Toggle popup on button click
    if button_response.clicked() {
        ui.data_mut(|data| data.insert_temp(popup_id, !is_open));
    }

    // Show popup if open
    if is_open {
        let area_response = egui::Area::new(popup_id.with("area"))
            .order(egui::Order::Foreground)
            .fixed_pos(button_response.rect.left_bottom() + egui::vec2(0.0, 2.0))
            .show(ui.ctx(), |ui| {
                let frame = egui::Frame::popup(ui.style());
                frame.show(ui, |ui| {
                    ui.set_max_width(250.0);
                    ui.set_max_height(300.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, option) in options.iter().enumerate() {
                            let mut is_selected = selected.contains(option);

                            ui.horizontal(|ui| {
                                // Show color swatch if colors are provided
                                if let Some(color_list) = colors {
                                    if i < color_list.len() {
                                        let (swatch_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(12.0, 12.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().rect_filled(swatch_rect, 2.0, color_list[i]);
                                    }
                                }

                                // Checkbox
                                if ui.checkbox(&mut is_selected, option).changed() {
                                    if is_selected {
                                        selected.insert(option.clone());
                                    } else {
                                        selected.remove(option);
                                    }
                                    changed = true;
                                }
                            });
                        }
                    });
                });
            });

        // Close popup if clicked outside
        if ui.input(|input| input.pointer.any_click()) {
            let pointer_pos = ui.input(|input| input.pointer.interact_pos());
            if let Some(pos) = pointer_pos {
                if !button_response.rect.contains(pos) && !area_response.response.rect.contains(pos)
                {
                    ui.data_mut(|data| data.insert_temp(popup_id, false));
                }
            }
        }
    }

    changed
}
