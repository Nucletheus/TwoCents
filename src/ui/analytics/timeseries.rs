use eframe::egui::{self, Color32};
use egui_plot::{Line, Plot, PlotPoints, Text};
use chrono::Datelike;

use crate::models::*;
use super::state::*;
use super::aggregation::*;
use super::empty_state::{render_no_data_state, render_empty_state};

pub fn render_time_series_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    state: &AnalyticsState,
) {
    if expenses.is_empty() {
        render_empty_state(ui, "No expenses recorded", Some("Add some expenses to see analytics"));
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
    let time_series = aggregate_time_series(&filtered, state.granularity, start, end);
    
    if time_series.is_empty() {
        render_no_data_state(ui);
        return;
    }
    
    // Check if all values are zero
    let has_data = time_series.iter().any(|p| p.amount > 0.0);
    if !has_data {
        render_empty_state(
            ui,
            "No spending in selected period",
            Some("Try a different date range or adjust your filters"),
        );
        return;
    }
    
    let dark_mode = ui.visuals().dark_mode;
    let text_color = if dark_mode {
        Color32::from_rgb(243, 244, 246)
    } else {
        Color32::from_rgb(17, 24, 39)
    };
    let accent_color = if dark_mode {
        Color32::from_rgb(97, 175, 239)
    } else {
        Color32::from_rgb(37, 99, 235)
    };
    let cumulative_color = Color32::from_rgb(139, 164, 66);
    
    // Convert time series to plot points
    let period_points: PlotPoints = time_series.iter()
        .enumerate()
        .map(|(i, point)| [i as f64, point.amount])
        .collect();
    
    let cumulative_points: PlotPoints = time_series.iter()
        .enumerate()
        .map(|(i, point)| [i as f64, point.cumulative])
        .collect();
    
    // Clone for use in closures
    let time_series_for_formatter = time_series.clone();
    let time_series_for_labels = time_series.clone();
    
    // Create the plot
    let plot = Plot::new("time_series_plot")
        .height(400.0)
        .allow_zoom(true)
        .allow_scroll(true)
        .allow_boxed_zoom(true)
        .show_grid([true, true])
        .label_formatter(move |name, value| {
            let idx = value.x as usize;
            if idx < time_series_for_formatter.len() {
                let point = &time_series_for_formatter[idx];
                let date_str = format_date_label(point.date, state.granularity);
                format!(
                    "{}\nAmount: ${:.2}\nCumulative: ${:.2}",
                    date_str, point.amount, point.cumulative
                )
            } else {
                format!("{}: {:.2}", name, value.y)
            }
        });
    
    plot.show(ui, |plot_ui| {
        // Period spending line
        let period_line = Line::new("Spending", period_points)
            .color(accent_color)
            .stroke(egui::Stroke::new(2.0, accent_color));
        plot_ui.line(period_line);
        
        // Cumulative spending line
        let cumulative_line = Line::new("Cumulative", cumulative_points)
            .color(cumulative_color)
            .stroke(egui::Stroke::new(1.5, cumulative_color));
        plot_ui.line(cumulative_line);
        
        // Add x-axis labels
        let label_interval = if time_series_for_labels.len() > 20 {
            time_series_for_labels.len() / 10
        } else {
            1
        };
        
        for (i, point) in time_series_for_labels.iter().enumerate() {
            if i % label_interval == 0 {
                let label = format_date_label(point.date, state.granularity);
                let text = Text::new(
                    format!("label_{}", i),
                    egui_plot::PlotPoint::new(i as f64, 0.0),
                    label
                ).color(text_color);
                plot_ui.text(text);
            }
        }
    });
}

fn format_date_label(date: chrono::NaiveDate, granularity: AnalyticsGranularity) -> String {
    match granularity {
        AnalyticsGranularity::Daily => date.format("%b %d").to_string(),
        AnalyticsGranularity::Weekly => date.format("%b %d").to_string(),
        AnalyticsGranularity::Monthly => date.format("%b %Y").to_string(),
        AnalyticsGranularity::Quarterly => {
            let quarter = (date.month() - 1) / 3 + 1;
            format!("Q{} {}", quarter, date.year())
        }
        AnalyticsGranularity::Yearly => date.format("%Y").to_string(),
    }
}
