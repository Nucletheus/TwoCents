use eframe::egui::{self, Color32, RichText};
use egui_plot::{Bar, BarChart, Plot};

use crate::models::*;
use super::state::*;
use super::aggregation::*;
use super::empty_state::{render_no_budgets_state, render_empty_state};

pub fn render_budget_vs_actual_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    budgets: &std::collections::HashMap<String, i64>,
    categories: &[Category],
    state: &AnalyticsState,
) {
    if budgets.is_empty() {
        render_no_budgets_state(ui);
        return;
    }
    
    let filtered = filter_expenses(expenses, state);
    let comparisons = compare_budget_vs_actual(&filtered, budgets, categories);
    
    if comparisons.is_empty() {
        render_empty_state(
            ui,
            "No budget or spending data",
            Some("Set budgets and add expenses to see comparison"),
        );
        return;
    }
    
    // Check if there's any actual data
    let has_actual = comparisons.iter().any(|c| c.actual > 0.0);
    if !has_actual {
        render_empty_state(
            ui,
            "No spending in selected period",
            Some("Add expenses to compare against your budgets"),
        );
        return;
    }
    
    ui.horizontal(|ui| {
        // Left side: Grouped bar chart
        ui.vertical(|ui| {
            let plot = Plot::new("budget_vs_actual_plot")
                .height(400.0)
                .allow_zoom(false)
                .allow_scroll(true)
                .show_grid([false, true])
                .label_formatter(|name, value| {
                    format!("{}: ${:.2}", name, value.y)
                });
            
            plot.show(ui, |plot_ui| {
                // Create grouped bars - budget and actual for each category
                let mut budget_bars = Vec::new();
                let mut actual_bars = Vec::new();
                
                for (i, comp) in comparisons.iter().enumerate() {
                    let x = i as f64;
                    
                    // Budget bar
                    let budget_bar = Bar::new(x - 0.2, comp.budgeted)
                        .width(0.35)
                        .fill(Color32::from_rgb(100, 150, 200))
                        .name("Budget");
                    budget_bars.push(budget_bar);
                    
                    // Actual bar - color based on variance
                    let actual_color = if comp.variance >= 0.0 {
                        Color32::from_rgb(100, 180, 100) // Green - under budget
                    } else {
                        Color32::from_rgb(200, 100, 100) // Red - over budget
                    };
                    
                    let actual_bar = Bar::new(x + 0.2, comp.actual)
                        .width(0.35)
                        .fill(actual_color)
                        .name("Actual");
                    actual_bars.push(actual_bar);
                }
                
                let budget_chart = BarChart::new("Budget", budget_bars);
                let actual_chart = BarChart::new("Actual", actual_bars);
                
                plot_ui.bar_chart(budget_chart);
                plot_ui.bar_chart(actual_chart);
            });
            
            // X-axis labels
            ui.horizontal(|ui| {
                for (i, comp) in comparisons.iter().enumerate() {
                    ui.label(format!("{}\n{}", i, comp.category.chars().take(15).collect::<String>()));
                }
            });
        });
        
        ui.add_space(20.0);
        
        // Right side: Legend and summary
        ui.vertical(|ui| {
            ui.heading(RichText::new("Budget vs Actual").strong());
            ui.add_space(8.0);
            
            // Legend
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(100, 150, 200));
                ui.label("Budget");
            });
            
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(100, 180, 100));
                ui.label("Actual (Under)");
            });
            
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(200, 100, 100));
                ui.label("Actual (Over)");
            });
            
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Summary statistics
            let total_budgeted: f64 = comparisons.iter().map(|c| c.budgeted).sum();
            let total_actual: f64 = comparisons.iter().map(|c| c.actual).sum();
            let total_variance = total_budgeted - total_actual;
            
            ui.label(RichText::new("Summary").strong());
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label("Total Budgeted:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("${:.2}", total_budgeted));
                });
            });
            
            ui.horizontal(|ui| {
                ui.label("Total Actual:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("${:.2}", total_actual));
                });
            });
            
            ui.horizontal(|ui| {
                ui.label("Variance:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let variance_color = if total_variance >= 0.0 {
                        Color32::from_rgb(100, 180, 100)
                    } else {
                        Color32::from_rgb(200, 100, 100)
                    };
                    ui.label(RichText::new(format!("${:.2}", total_variance)).color(variance_color));
                });
            });
            
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Category details
            ui.label(RichText::new("Categories").strong());
            ui.add_space(4.0);
            
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for comp in &comparisons {
                        ui.horizontal(|ui| {
                            // Color swatch
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, comp.color);
                            
                            ui.add_space(4.0);
                            
                            // Category name (truncated)
                            let display_name = if comp.category.len() > 20 {
                                format!("{}...", &comp.category[..17])
                            } else {
                                comp.category.clone()
                            };
                            ui.label(display_name);
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Variance with arrow
                                let variance_text = if comp.variance >= 0.0 {
                                    format!("↑${:.2}", comp.variance.abs())
                                } else {
                                    format!("↓${:.2}", comp.variance.abs())
                                };
                                let variance_color = if comp.variance >= 0.0 {
                                    Color32::from_rgb(100, 180, 100)
                                } else {
                                    Color32::from_rgb(200, 100, 100)
                                };
                                ui.label(RichText::new(variance_text).color(variance_color));
                            });
                        });
                        ui.add_space(2.0);
                    }
                });
        });
    });
}
