pub mod state;
pub mod aggregation;
pub mod charts_common;
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
        // ponytail: page heading + muted description. Matches the rest of
        // the app's heading hierarchy.
        crate::ui::components::heading_lg(ui, "Analytics");
        crate::ui::components::label_muted(
            ui,
            "Visual breakdowns of your spending across the date range you set above.",
        );
        ui.add_space(8.0);

        let mut state_changed = false;

        // Chart type tabs — underline-style (matches the main tab strip)
        // rather than the default selectable_label pill.
        let chart_tabs: [(AnalyticsChart, &str); 5] = [
            (AnalyticsChart::TimeSeries, "Time Series"),
            (AnalyticsChart::CategoryBreakdown, "Category Breakdown"),
            (AnalyticsChart::BudgetVsActual, "Budget vs Actual"),
            (AnalyticsChart::Candlestick, "Candlestick"),
            (AnalyticsChart::PeriodComparison, "Period Comparison"),
        ];
        ui.horizontal_wrapped(|ui| {
            for (chart, label) in chart_tabs {
                if crate::ui::components::tab_label_button(ui, self.analytics_state.active_chart == chart, label).clicked() {
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

        // ponytail: hide the granularity dropdown on the Category
        // Breakdown tab because it has no effect there. The
        // category chart sums across the date range — there's no
        // time axis for granularity to control. The user can still
        // see and change it on the Time Series tab.
        let show_granularity = self.analytics_state.active_chart == AnalyticsChart::TimeSeries;
        let filters_changed = render_filter_panel(
            ui,
            &mut self.analytics_state,
            &self.categories,
            &self.members,
            &available_vendors,
            show_granularity,
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
                render_time_series_chart(
                    ui,
                    &self.expenses,
                    &self.categories,
                    &self.analytics_state,
                );
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
                render_candlestick_chart(ui, &self.expenses, &self.categories, &self.analytics_state);
            }
            AnalyticsChart::PeriodComparison => {
                render_period_comparison_chart(ui, &self.expenses, &self.categories, &mut self.analytics_state);
            }
        }

        // ponytail: Reset View is one-shot. The active chart reads it
        // inside its plot closure this frame; clear it afterwards so it
        // doesn't re-fire on every subsequent frame.
        if self.analytics_state.reset_view {
            self.analytics_state.reset_view = false;
            state_changed = true;
        }

        // Persist state if anything changed
        if state_changed {
            let row = self.analytics_state.to_row();
            let _ = save_analytics_state(&self.conn, &row);
        }
    }
}
