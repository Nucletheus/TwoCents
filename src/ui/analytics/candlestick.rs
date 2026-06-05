use eframe::egui::{self, Color32, RichText};
use plotters::prelude::*;
use plotters::style::colors::WHITE;

use crate::models::*;
use super::aggregation::*;
use super::state::*;
use super::empty_state::{render_no_data_state, render_empty_state};

pub fn render_candlestick_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    state: &AnalyticsState,
) {
    if expenses.is_empty() {
        render_empty_state(ui, "No expenses recorded", Some("Add some expenses to see spending velocity"));
        return;
    }
    
    let Some(start) = state.date_start else {
        render_empty_state(ui, "No start date set", Some("Select a date range in the filters above"));
        return;
    };
    let Some(end) = state.date_end else {
        render_empty_state(ui, "No end date set", Some("Select a date range in the filters above"));
        return;
    };
    
    let filtered = filter_expenses(expenses, state);
    let candlesticks = aggregate_candlestick(&filtered, state.granularity, start, end);
    
    if candlesticks.is_empty() {
        render_no_data_state(ui);
        return;
    }
    
    ui.horizontal(|ui| {
        // Left side: Candlestick chart
        ui.vertical(|ui| {
            let chart_size = egui::vec2(600.0, 400.0);
            let (_rect, _response) = ui.allocate_exact_size(chart_size, egui::Sense::hover());
            
            // Render candlestick chart using plotters
            let mut buffer = vec![0u8; (chart_size.x as usize) * (chart_size.y as usize) * 4];
            
            {
                let root = BitMapBackend::with_buffer(&mut buffer, (chart_size.x as u32, chart_size.y as u32))
                    .into_drawing_area();
                
                let bg_color = if ui.visuals().dark_mode {
                    RGBColor(30, 30, 30)
                } else {
                    WHITE
                };
                root.fill(&bg_color).unwrap();
                
                // Calculate Y axis range
                let max_val = candlesticks.iter()
                    .map(|c| c.high)
                    .fold(0.0_f64, f64::max);
                let min_val = candlesticks.iter()
                    .map(|c| c.low)
                    .fold(f64::MAX, f64::min);
                let min_val = if min_val == f64::MAX { 0.0 } else { min_val };
                
                let y_range = if max_val == min_val {
                    0.0..(max_val + 100.0)
                } else {
                    (min_val * 0.9)..(max_val * 1.1)
                };
                
                let mut chart = ChartBuilder::on(&root)
                    .margin(20)
                    .x_label_area_size(40)
                    .y_label_area_size(60)
                    .build_cartesian_2d(0.0..candlesticks.len() as f64, y_range)
                    .unwrap();
                
                chart.configure_mesh()
                    .x_labels(candlesticks.len().min(10))
                    .y_labels(5)
                    .x_label_formatter(&|idx| {
                        let i = *idx as usize;
                        if i < candlesticks.len() {
                            candlesticks[i].label.clone()
                        } else {
                            String::new()
                        }
                    })
                    .draw()
                    .unwrap();
                
                // Draw candlesticks
                for (i, candle) in candlesticks.iter().enumerate() {
                    let is_bullish = candle.close >= candle.open;
                    
                    let body_color = if is_bullish {
                        RGBColor(76, 175, 80) // Green
                    } else {
                        RGBColor(244, 67, 54) // Red
                    };
                    
                    let wick_color = body_color.clone();
                    
                    // Draw wick (high-low line)
                    chart.draw_series(std::iter::once(PathElement::new(
                        vec![(i as f64, candle.low), (i as f64, candle.high)],
                        ShapeStyle {
                            color: wick_color.to_rgba(),
                            filled: false,
                            stroke_width: 1,
                        },
                    ))).unwrap();
                    
                    // Draw body (open-close rectangle)
                    let body_top = candle.open.max(candle.close);
                    let body_bottom = candle.open.min(candle.close);
                    
                    chart.draw_series(std::iter::once(Rectangle::new(
                        [(i as f64 - 0.3, body_bottom), (i as f64 + 0.3, body_top)],
                        body_color.filled(),
                    ))).unwrap();
                }
            }
            
            // Convert to egui texture and display
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [chart_size.x as usize, chart_size.y as usize],
                &buffer,
            );
            
            let texture = ui.ctx().load_texture(
                "candlestick_chart",
                color_image,
                egui::TextureOptions::default(),
            );
            
            ui.image(egui::load::SizedTexture::new(
                texture.id(),
                chart_size,
            ));
        });
        
        ui.add_space(20.0);
        
        // Right side: Legend and summary
        ui.vertical(|ui| {
            ui.heading(RichText::new("Spending Velocity").strong());
            ui.add_space(8.0);
            
            ui.label("Each candle represents:");
            ui.indent("candle_explanation", |ui| {
                ui.label("Open: First transaction");
                ui.label("High: Peak spending");
                ui.label("Low: Minimum spending");
                ui.label("Close: Last transaction");
            });
            
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Legend
            ui.label(RichText::new("Legend").strong());
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(76, 175, 80));
                ui.label("Bullish (close ≥ open)");
            });
            
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(244, 67, 54));
                ui.label("Bearish (close < open)");
            });
            
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Summary statistics
            ui.label(RichText::new("Summary").strong());
            ui.add_space(4.0);
            
            let total_periods = candlesticks.len();
            let bullish_count = candlesticks.iter().filter(|c| c.close >= c.open).count();
            let bearish_count = total_periods - bullish_count;
            
            let total_spending: f64 = candlesticks.iter().map(|c| c.close).sum();
            let avg_spending = if total_periods > 0 {
                total_spending / total_periods as f64
            } else {
                0.0
            };
            
            ui.horizontal(|ui| {
                ui.label("Periods:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{}", total_periods));
                });
            });
            
            ui.horizontal(|ui| {
                ui.label("Bullish:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{}", bullish_count)).color(Color32::from_rgb(76, 175, 80)));
                });
            });
            
            ui.horizontal(|ui| {
                ui.label("Bearish:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{}", bearish_count)).color(Color32::from_rgb(244, 67, 54)));
                });
            });
            
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                ui.label("Total Spending:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("${:.2}", total_spending));
                });
            });
            
            ui.horizontal(|ui| {
                ui.label("Avg per Period:");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("${:.2}", avg_spending));
                });
            });
        });
    });
}
