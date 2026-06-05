pub mod state;
pub mod aggregation;
pub mod filters;
pub mod timeseries;
pub mod category;
pub mod budget_comparison;
pub mod candlestick;
pub mod period_comparison;
pub mod empty_state;

use eframe::egui;
use crate::db::save_analytics_state;
use crate::TwoCentsApp;
use state::*;
use filters::render_filter_panel;
use timeseries::render_time_series_chart;
use category::render_category_breakdown_chart;
use budget_comparison::render_budget_vs_actual_chart;
use candlestick::render_candlestick_chart;
use period_comparison::render_period_comparison_chart;

impl TwoCentsApp {
    pub fn ui_analytics(&mut self, ui: &mut egui::Ui) {
        ui.heading("Analytics");
        ui.add_space(8.0);
        
        let mut state_changed = false;
        
        // Chart type tabs
        ui.horizontal(|ui| {
            let charts = [
                AnalyticsChart::TimeSeries,
                AnalyticsChart::CategoryBreakdown,
                AnalyticsChart::BudgetVsActual,
                AnalyticsChart::Candlestick,
                AnalyticsChart::PeriodComparison,
            ];
            
            for chart in charts {
                let selected = self.analytics_state.active_chart == chart;
                if ui.selectable_label(selected, chart.label()).clicked() {
                    self.analytics_state.active_chart = chart;
                    state_changed = true;
                }
            }
        });
        
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
        
        // Filter panel
        let available_vendors: Vec<String> = self.expenses
            .iter()
            .map(|e| e.vendor.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        
        let filters_changed = render_filter_panel(
            ui,
            &mut self.analytics_state,
            &self.categories,
            &self.members,
            &available_vendors,
        );
        
        if filters_changed {
            state_changed = true;
        }
        
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        
        // Render active chart
        match self.analytics_state.active_chart {
            AnalyticsChart::TimeSeries => {
                render_time_series_chart(ui, &self.expenses, &self.analytics_state);
            }
            AnalyticsChart::CategoryBreakdown => {
                if render_category_breakdown_chart(ui, &self.expenses, &self.categories, &mut self.analytics_state) {
                    state_changed = true;
                }
            }
            AnalyticsChart::BudgetVsActual => {
                render_budget_vs_actual_chart(ui, &self.expenses, &self.budgets, &self.categories, &self.analytics_state);
            }
            AnalyticsChart::Candlestick => {
                render_candlestick_chart(ui, &self.expenses, &self.analytics_state);
            }
            AnalyticsChart::PeriodComparison => {
                render_period_comparison_chart(ui, &self.expenses, &self.categories, &mut self.analytics_state);
            }
        }
        
        // Persist state if anything changed
        if state_changed {
            let row = self.analytics_state.to_row();
            let _ = save_analytics_state(&self.conn, &row);
        }
    }
}
