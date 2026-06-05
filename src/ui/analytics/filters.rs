use eframe::egui::{self, Color32, RichText};
use std::collections::HashSet;

use crate::models::{Category, HouseholdMember};
use super::state::*;

pub fn render_filter_panel(
    ui: &mut egui::Ui,
    state: &mut AnalyticsState,
    categories: &[Category],
    members: &[HouseholdMember],
    available_vendors: &[String],
) -> bool {
    let mut changed = false;
    
    // Date preset buttons
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Date Range:").strong());
        
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
        
        for preset in presets {
            let selected = state.date_preset == preset;
            if ui.selectable_label(selected, preset.label()).clicked() {
                state.date_preset = preset;
                let (start, end) = preset.date_range();
                state.date_start = start;
                state.date_end = end;
                changed = true;
            }
        }
        
        ui.add_space(8.0);
        if ui.small_button("Clear").clicked() {
            state.date_preset = DatePreset::Last3Months;
            let (start, end) = DatePreset::Last3Months.date_range();
            state.date_start = start;
            state.date_end = end;
            changed = true;
        }
    });
    
    // Custom date range (only shown when Custom is selected)
    if state.date_preset == DatePreset::Custom {
        ui.horizontal(|ui| {
            ui.label("Start:");
            if let Some(ref mut start) = state.date_start {
                let mut date_str = start.format("%Y-%m-%d").to_string();
                if ui.text_edit_singleline(&mut date_str).changed() {
                    if let Ok(new_date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        *start = new_date;
                        changed = true;
                    }
                }
            }
            
            ui.label("End:");
            if let Some(ref mut end) = state.date_end {
                let mut date_str = end.format("%Y-%m-%d").to_string();
                if ui.text_edit_singleline(&mut date_str).changed() {
                    if let Ok(new_date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                        *end = new_date;
                        changed = true;
                    }
                }
            }
        });
    }
    
    // Granularity and filters
    ui.horizontal_wrapped(|ui| {
        // Granularity dropdown
        ui.label(RichText::new("Granularity:").strong());
        egui::ComboBox::from_id_salt("granularity_combo")
            .selected_text(state.granularity.label())
            .show_ui(ui, |ui| {
                let granularities = [
                    AnalyticsGranularity::Daily,
                    AnalyticsGranularity::Weekly,
                    AnalyticsGranularity::Monthly,
                    AnalyticsGranularity::Quarterly,
                    AnalyticsGranularity::Yearly,
                ];
                
                for g in granularities {
                    if ui.selectable_label(state.granularity == g, g.label()).clicked() {
                        state.granularity = g;
                        changed = true;
                    }
                }
            });
        
        if ui.small_button("Clear").clicked() {
            state.granularity = AnalyticsGranularity::Monthly;
            state.selected_categories.clear();
            state.selected_members.clear();
            state.selected_vendors.clear();
            changed = true;
        }
        
        ui.separator();
        
        // Category filter with colors
        let category_names: Vec<String> = categories.iter().map(|c| c.name.clone()).collect();
        let category_colors: Vec<Color32> = categories.iter().map(|c| c.color).collect();
        ui.label(RichText::new("Categories:").strong());
        if multi_select_dropdown_with_colors(
            ui,
            "categories_dropdown",
            &category_names,
            Some(&category_colors),
            &mut state.selected_categories,
        ) {
            changed = true;
        }
        
        ui.separator();
        
        // Member filter with colors
        let member_names: Vec<String> = members.iter().map(|m| m.name.clone()).collect();
        let member_colors: Vec<Color32> = members.iter().map(|m| m.color).collect();
        ui.label(RichText::new("Members:").strong());
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
        ui.label(RichText::new("Vendors:").strong());
        if multi_select_dropdown_with_colors(
            ui,
            "vendors_dropdown",
            available_vendors,
            None,
            &mut state.selected_vendors,
        ) {
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
                if !button_response.rect.contains(pos) && !area_response.response.rect.contains(pos) {
                    ui.data_mut(|data| data.insert_temp(popup_id, false));
                }
            }
        }
    }
    
    changed
}
