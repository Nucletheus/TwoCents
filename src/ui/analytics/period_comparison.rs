use eframe::egui::{self, Color32, RichText};
use egui_plot::{Bar, BarChart, Legend, Plot};
use chrono::NaiveDate;

use crate::models::*;
use super::aggregation::*;
use super::state::*;
use super::empty_state::render_empty_state;

pub fn render_period_comparison_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    categories: &[Category],
    state: &mut AnalyticsState,
) {
    if expenses.is_empty() {
        render_empty_state(ui, "No expenses recorded", Some("Add some expenses to compare periods"));
        return;
    }
    
    ui.horizontal(|ui| {
        ui.label("Period A:");
        egui::ComboBox::from_id_salt("period_a_preset")
            .selected_text(state.comparison_period_a_preset.label())
            .show_ui(ui, |ui| {
                let presets = [
                    DatePreset::ThisMonth,
                    DatePreset::LastMonth,
                    DatePreset::Last3Months,
                    DatePreset::Last6Months,
                    DatePreset::YTD,
                    DatePreset::LastYear,
                    DatePreset::AllTime,
                ];
                for preset in presets {
                    ui.selectable_value(&mut state.comparison_period_a_preset, preset, preset.label());
                }
            });
        
        ui.add_space(16.0);
        
        ui.label("Period B:");
        egui::ComboBox::from_id_salt("period_b_preset")
            .selected_text(state.comparison_period_b_preset.label())
            .show_ui(ui, |ui| {
                let presets = [
                    DatePreset::ThisMonth,
                    DatePreset::LastMonth,
                    DatePreset::Last3Months,
                    DatePreset::Last6Months,
                    DatePreset::YTD,
                    DatePreset::LastYear,
                    DatePreset::AllTime,
                ];
                for preset in presets {
                    ui.selectable_value(&mut state.comparison_period_b_preset, preset, preset.label());
                }
            });
        
        ui.add_space(16.0);
        
        ui.label("Mode:");
        egui::ComboBox::from_id_salt("comparison_mode")
            .selected_text(state.comparison_mode.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.comparison_mode, ComparisonMode::SideBySide, "Side-by-Side");
                ui.selectable_value(&mut state.comparison_mode, ComparisonMode::Overlay, "Overlay");
                ui.selectable_value(&mut state.comparison_mode, ComparisonMode::Delta, "Delta");
            });
    });
    
    ui.add_space(8.0);
    
    let (start_a, end_a) = state.comparison_period_a_preset.date_range();
    let (start_b, end_b) = state.comparison_period_b_preset.date_range();
    
    let start_a = start_a.unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let end_a = end_a.unwrap_or_else(|| chrono::Local::now().date_naive());
    let start_b = start_b.unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let end_b = end_b.unwrap_or_else(|| chrono::Local::now().date_naive());
    
    let filtered_a: Vec<Expense> = expenses.iter()
        .filter(|e| {
            if let Some(date) = parse_expense_date(&e.date) {
                date >= start_a && date <= end_a
            } else {
                false
            }
        })
        .cloned()
        .collect();
    
    let filtered_b: Vec<Expense> = expenses.iter()
        .filter(|e| {
            if let Some(date) = parse_expense_date(&e.date) {
                date >= start_b && date <= end_b
            } else {
                false
            }
        })
        .cloned()
        .collect();
    
    let data = aggregate_period_comparison(
        &filtered_a,
        &filtered_b,
        state.comparison_period_a_preset.label(),
        state.comparison_period_b_preset.label(),
        categories,
    );
    
    // Check if both periods have no data
    if data.period_a_total == 0.0 && data.period_b_total == 0.0 {
        render_empty_state(
            ui,
            "No spending in either period",
            Some("Try different date ranges or add expenses"),
        );
        return;
    }
    
    match state.comparison_mode {
        ComparisonMode::SideBySide => render_side_by_side(ui, &data),
        ComparisonMode::Overlay => render_overlay(ui, &data),
        ComparisonMode::Delta => render_delta(ui, &data),
    }
}

fn render_side_by_side(ui: &mut egui::Ui, data: &PeriodComparisonData) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading(RichText::new(&data.period_a_label).strong());
            ui.label(format!("Total: ${:.2}", data.period_a_total));
            ui.add_space(8.0);
            
            let plot = Plot::new("period_a_plot")
                .height(300.0)
                .show_x(false)
                .legend(Legend::default());
            
            plot.show(ui, |plot_ui| {
                let bars: Vec<Bar> = data.category_comparisons.iter().enumerate().map(|(i, cat)| {
                    Bar::new(i as f64, cat.period_a_amount)
                        .name(&cat.category)
                        .fill(cat.color)
                        .width(0.8)
                }).collect();
                
                plot_ui.bar_chart(BarChart::new("Period A", bars));
            });
        });
        
        ui.add_space(16.0);
        
        ui.vertical(|ui| {
            ui.heading(RichText::new(&data.period_b_label).strong());
            ui.label(format!("Total: ${:.2}", data.period_b_total));
            ui.add_space(8.0);
            
            let plot = Plot::new("period_b_plot")
                .height(300.0)
                .show_x(false)
                .legend(Legend::default());
            
            plot.show(ui, |plot_ui| {
                let bars: Vec<Bar> = data.category_comparisons.iter().enumerate().map(|(i, cat)| {
                    Bar::new(i as f64, cat.period_b_amount)
                        .name(&cat.category)
                        .fill(cat.color)
                        .width(0.8)
                }).collect();
                
                plot_ui.bar_chart(BarChart::new("Period B", bars));
            });
        });
    });
    
    ui.add_space(16.0);
    render_summary(ui, data);
}

fn render_overlay(ui: &mut egui::Ui, data: &PeriodComparisonData) {
    let plot = Plot::new("overlay_plot")
        .height(400.0)
        .show_x(false)
        .legend(Legend::default());
    
    plot.show(ui, |plot_ui| {
        let bars_a: Vec<Bar> = data.category_comparisons.iter().enumerate().map(|(i, cat)| {
            Bar::new(i as f64 - 0.2, cat.period_a_amount)
                .name(&cat.category)
                .fill(Color32::from_rgb(100, 150, 200))
                .width(0.4)
        }).collect();
        
        let bars_b: Vec<Bar> = data.category_comparisons.iter().enumerate().map(|(i, cat)| {
            Bar::new(i as f64 + 0.2, cat.period_b_amount)
                .name(&cat.category)
                .fill(Color32::from_rgb(200, 100, 100))
                .width(0.4)
        }).collect();
        
        plot_ui.bar_chart(BarChart::new(&data.period_a_label, bars_a));
        plot_ui.bar_chart(BarChart::new(&data.period_b_label, bars_b));
    });
    
    ui.add_space(16.0);
    render_summary(ui, data);
}

fn render_delta(ui: &mut egui::Ui, data: &PeriodComparisonData) {
    ui.heading(RichText::new("Difference (Period B - Period A)").strong());
    ui.add_space(8.0);
    
    let plot = Plot::new("delta_plot")
        .height(400.0)
        .show_x(false)
        .legend(Legend::default());
    
    plot.show(ui, |plot_ui| {
        let bars: Vec<Bar> = data.category_comparisons.iter().enumerate().map(|(i, cat)| {
            let color = if cat.difference >= 0.0 {
                Color32::from_rgb(200, 100, 100)
            } else {
                Color32::from_rgb(100, 200, 100)
            };
            
            Bar::new(i as f64, cat.difference)
                .name(&cat.category)
                .fill(color)
                .width(0.8)
        }).collect();
        
        plot_ui.bar_chart(BarChart::new("Difference", bars));
    });
    
    ui.add_space(16.0);
    render_summary(ui, data);
}

fn render_summary(ui: &mut egui::Ui, data: &PeriodComparisonData) {
    ui.separator();
    ui.add_space(8.0);
    
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Summary").strong());
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label(format!("{}:", data.period_a_label));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("${:.2}", data.period_a_total));
                });
            });
            
            ui.horizontal(|ui| {
                ui.label(format!("{}:", data.period_b_label));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("${:.2}", data.period_b_total));
                });
            });
            
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                ui.label("Difference:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let color = if data.difference >= 0.0 {
                        Color32::from_rgb(200, 100, 100)
                    } else {
                        Color32::from_rgb(100, 200, 100)
                    };
                    let text = if data.difference >= 0.0 {
                        format!("+${:.2} (+{:.1}%)", data.difference, data.difference_pct)
                    } else {
                        format!("-${:.2} ({:.1}%)", data.difference.abs(), data.difference_pct.abs())
                    };
                    ui.label(RichText::new(text).color(color));
                });
            });
        });
        
        ui.add_space(32.0);
        
        ui.vertical(|ui| {
            ui.label(RichText::new("Category Breakdown").strong());
            ui.add_space(4.0);
            
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for cat in &data.category_comparisons {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, cat.color);
                            
                            ui.add_space(4.0);
                            
                            let display_name = if cat.category.len() > 20 {
                                format!("{}...", &cat.category[..17])
                            } else {
                                cat.category.clone()
                            };
                            ui.label(display_name);
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let color = if cat.difference >= 0.0 {
                                    Color32::from_rgb(200, 100, 100)
                                } else {
                                    Color32::from_rgb(100, 200, 100)
                                };
                                let text = if cat.difference >= 0.0 {
                                    format!("+${:.2}", cat.difference)
                                } else {
                                    format!("-${:.2}", cat.difference.abs())
                                };
                                ui.label(RichText::new(text).color(color));
                            });
                        });
                        ui.add_space(2.0);
                    }
                });
        });
    });
}
