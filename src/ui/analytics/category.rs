use eframe::egui::{self, Color32, Pos2, RichText, Stroke};

use crate::models::*;
use super::state::*;
use super::aggregation::*;
use super::empty_state::render_no_data_state;

pub fn render_category_breakdown_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    categories: &[Category],
    state: &mut AnalyticsState,
) -> bool {
    let mut state_changed = false;
    let filtered = filter_expenses(expenses, state);
    let category_totals = aggregate_by_category(&filtered, categories);
    
    if category_totals.is_empty() {
        render_no_data_state(ui);
        return false;
    }
    
    let total: f64 = category_totals.iter().map(|c| c.amount).sum();
    
    ui.horizontal(|ui| {
        // Left side: Donut chart
        ui.vertical(|ui| {
            let chart_size = 400.0;
            let (rect, _response) = ui.allocate_exact_size(
                egui::vec2(chart_size, chart_size),
                egui::Sense::hover(),
            );
            
            // Draw donut chart
            let center = rect.center();
            let outer_radius = chart_size * 0.4;
            let inner_radius = chart_size * 0.25;
            
            let mut start_angle = -std::f32::consts::FRAC_PI_2; // Start from top
            
            for cat in &category_totals {
                let slice_angle = (cat.amount / total) as f32 * std::f32::consts::TAU;
                let end_angle = start_angle + slice_angle;
                
                // Draw slice
                draw_donut_slice(
                    ui,
                    center,
                    inner_radius,
                    outer_radius,
                    start_angle,
                    end_angle,
                    cat.color,
                );
                
                start_angle = end_angle;
            }
            
            // Draw center circle (donut hole)
            ui.painter().circle_filled(center, inner_radius, ui.visuals().panel_fill);
            
            // Draw total in center
            let total_text = format!("${:.2}", total);
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                total_text,
                egui::FontId::proportional(18.0),
                ui.visuals().text_color(),
            );
        });
        
        ui.add_space(20.0);
        
        // Right side: Legend
        ui.vertical(|ui| {
            ui.heading(RichText::new("Categories").strong());
            ui.add_space(8.0);
            
            ui.label(RichText::new("Click a category to filter").size(11.0).color(ui.visuals().weak_text_color()));
            ui.add_space(4.0);
            
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for cat in &category_totals {
                        let is_filtered = state.selected_categories.contains(&cat.category);
                        let is_include_mode = state.category_filter_mode == FilterMode::Include;
                        
                        // Determine visual state
                        let is_active = if is_include_mode {
                            // In include mode, active means it's in the filter set
                            is_filtered
                        } else {
                            // In exclude mode, active means it's NOT in the filter set
                            !is_filtered
                        };
                        
                        let _response = ui.horizontal(|ui| {
                            // Color swatch with click indicator
                            let (swatch_rect, swatch_response) = ui.allocate_exact_size(
                                egui::vec2(16.0, 16.0),
                                egui::Sense::click(),
                            );
                            
                            let swatch_color = if is_active {
                                cat.color
                            } else {
                                cat.color.gamma_multiply(0.3)
                            };
                            ui.painter().rect_filled(swatch_rect, 2.0, swatch_color);
                            
                            if swatch_response.clicked() {
                                state_changed = true;
                                toggle_category_filter(state, &cat.category);
                            }
                            
                            ui.add_space(4.0);
                            
                            // Category name (clickable)
                            let name_response = ui.add(
                                egui::Label::new(&cat.category)
                                    .sense(egui::Sense::click())
                            );
                            
                            if name_response.clicked() {
                                state_changed = true;
                                toggle_category_filter(state, &cat.category);
                            }
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Percentage
                                ui.label(format!("{:.1}%", cat.percentage));
                                ui.add_space(8.0);
                                // Amount
                                ui.label(format!("${:.2}", cat.amount));
                            });
                        });
                        
                        // Highlight row if filtered
                        if is_active {
                            // Add a subtle background to indicate active filter
                        }
                        
                        ui.add_space(4.0);
                    }
                });
            
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            
            // Total
            ui.horizontal(|ui| {
                ui.label(RichText::new("Total:").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("${:.2}", total)).strong());
                });
            });
            
            // Show filter status
            if !state.selected_categories.is_empty() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mode_text = match state.category_filter_mode {
                        FilterMode::Include => "Showing only",
                        FilterMode::Exclude => "Excluding",
                    };
                    ui.label(RichText::new(format!("{} {} categories", mode_text, state.selected_categories.len()))
                        .size(11.0)
                        .color(ui.visuals().weak_text_color()));
                    
                    if ui.small_button("Clear").clicked() {
                        state.selected_categories.clear();
                        state_changed = true;
                    }
                });
            }
        });
    });
    
    state_changed
}

fn toggle_category_filter(state: &mut AnalyticsState, category: &str) {
    if state.selected_categories.contains(category) {
        state.selected_categories.remove(category);
    } else {
        state.selected_categories.insert(category.to_string());
        // If we're in include mode and just added a category, keep it in include mode
        // If we're in exclude mode and just added a category, keep it in exclude mode
    }
}

fn draw_donut_slice(
    ui: &mut egui::Ui,
    center: Pos2,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: Color32,
) {
    let painter = ui.painter();
    let num_segments = ((end_angle - start_angle).abs() * 20.0).max(3.0) as usize;
    
    let mut points = Vec::new();
    
    // Outer arc
    for i in 0..=num_segments {
        let angle = start_angle + (end_angle - start_angle) * (i as f32 / num_segments as f32);
        let x = center.x + outer_radius * angle.cos();
        let y = center.y + outer_radius * angle.sin();
        points.push(Pos2::new(x, y));
    }
    
    // Inner arc (reverse direction)
    for i in (0..=num_segments).rev() {
        let angle = start_angle + (end_angle - start_angle) * (i as f32 / num_segments as f32);
        let x = center.x + inner_radius * angle.cos();
        let y = center.y + inner_radius * angle.sin();
        points.push(Pos2::new(x, y));
    }
    
    // Close the shape
    points.push(points[0]);
    
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}
