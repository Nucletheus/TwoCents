pub mod state;
pub mod aggregation;
pub mod charts_common;
pub mod picker;
pub mod filters;
pub mod category;
pub mod budget_comparison;
pub mod period_comparison;
pub mod empty_state;

use eframe::egui;
use crate::db::save_analytics_state;
use crate::models::BudgetGranularity;
use crate::TwoCentsApp;
use state::*;
use filters::render_filter_panel;
use category::render_category_breakdown_chart;
use budget_comparison::{render_budget_vs_actual_chart, BudgetPeriodBudgets};
use period_comparison::render_period_comparison_chart;

impl TwoCentsApp {
    /// Budget vs Actual re-prices each category's envelope for
    /// the SELECTED analytics period (the shared `self.budgets` map only
    /// covers the Budgets tab's currently-selected period). Lives here
    /// because the snapshot cache + cap lookup (`compute_budget_for`)
    /// are app state. `index` selects one of the 24 shared
    /// `comparison_periods` entries (0 = current).
    fn budget_period_budgets(&self, view: BudgetViewPeriod, index: usize) -> BudgetPeriodBudgets {
        let periods = charts_common::comparison_periods(view);
        let entry = &periods[index.min(periods.len() - 1)];
        let gran = match view {
            BudgetViewPeriod::Week => BudgetGranularity::Weekly,
            BudgetViewPeriod::Month => BudgetGranularity::Monthly,
            BudgetViewPeriod::Quarter => BudgetGranularity::Quarterly,
            BudgetViewPeriod::Year => BudgetGranularity::Yearly,
        };
        let (start, end) = (entry.start, entry.end);
        // year-honest snapshots — one load per frame for the
        // viewed year (the Budgets-tab cache only covers its own year).
        let snaps = if entry.year == self.budget_year {
            std::borrow::Cow::Borrowed(&self.cached_budget_snapshots[..])
        } else {
            std::borrow::Cow::Owned(
                crate::db::load_budget_snapshots_for_year(&self.conn, self.household_id, entry.year)
                    .unwrap_or_default(),
            )
        };
        let mut budgets: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for cat in crate::models::category_assignable_labels(&self.categories) {
            let amount = self.compute_budget_for(&cat, gran, entry.year, Some(entry.period), &snaps);
            if amount > 0 {
                budgets.insert(cat, amount);
            }
        }
        let period_label = match view {
            BudgetViewPeriod::Week => "week",
            BudgetViewPeriod::Month => "month",
            BudgetViewPeriod::Quarter => "quarter",
            BudgetViewPeriod::Year => "year",
        };
        BudgetPeriodBudgets { budgets, start, end, period_label, period_name: entry.label.clone() }
    }

    pub fn ui_analytics(&mut self, ui: &mut egui::Ui) {
        // drop selected categories whose labels no longer exist
        // (renames/deletes) — persisted stale entries silently filtered
        // everything else out while showing up as a phantom "N selected".
        {
            let parents = crate::models::category_parent_map(&self.categories);
            let full_labels: std::collections::HashSet<String> = self
                .categories
                .iter()
                .map(|c| c.full_label(&parents))
                .collect();
            self.analytics_state
                .selected_categories
                .retain(|c| full_labels.contains(c));
        }

        // page heading + muted description. Matches the rest of
        // the app's heading hierarchy.
        crate::ui::components::heading_lg(ui, "Analytics");
        crate::ui::components::label_muted(
            ui,
            "Visual breakdowns of your spending. Category Breakdown uses the date range below; Budget vs Actual uses its own period; Period Comparison uses its A/B ranges.",
        );
        ui.add_space(8.0);

        let mut state_changed = false;

        // Chart type tabs — underline-style (matches the main tab strip)
        // rather than the default selectable_label pill.
        let chart_tabs: [(AnalyticsChart, &str); 3] = [
            (AnalyticsChart::CategoryBreakdown, "Category Breakdown"),
            (AnalyticsChart::BudgetVsActual, "Budget vs Actual"),
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

        // Filter panel. The date-preset row is hidden on the two
        // charts that own their own date story (Budget vs Actual → its
        // period toggle; Period Comparison → its A/B combos) — one date
        // story per tab. Category/member/vendor filters still apply on
        // all three. Category selection is the shared right-column picker
        // on each chart tab (no dropdown here).
        let show_date_range = self.analytics_state.active_chart == AnalyticsChart::CategoryBreakdown;
        // HashSet iteration order is randomized per instance —
        // rebuilding it every frame made the vendor dropdown rows reshuffle
        // frame-to-frame ("scrolling all over the place"). Sort for a stable
        // order; drop blank vendors (expenses with no vendor set).
        let mut available_vendors: Vec<String> = self.expenses
            .iter()
            .map(|e| e.vendor.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .filter(|v| !v.is_empty())
            .collect();
        available_vendors.sort();

        let filters_changed = render_filter_panel(
            ui,
            &mut self.analytics_state,
            &self.members,
            &available_vendors,
            show_date_range,
        );

        if filters_changed {
            state_changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        // Render active chart
        match self.analytics_state.active_chart {
            AnalyticsChart::CategoryBreakdown => {
                if render_category_breakdown_chart(ui, &self.expenses, &self.categories, &mut self.analytics_state) {
                    state_changed = true;
                }
            }
            AnalyticsChart::BudgetVsActual => {
                let priced = self.budget_period_budgets(
                    self.analytics_state.budget_view_period,
                    self.analytics_state.budget_period_index,
                );
                if render_budget_vs_actual_chart(ui, &self.expenses, &priced, &self.categories, &mut self.analytics_state) {
                    state_changed = true;
                }
            }
            AnalyticsChart::PeriodComparison => {
                if render_period_comparison_chart(ui, &self.expenses, &self.categories, &mut self.analytics_state) {
                    state_changed = true;
                }
            }
        }

        // Reset View is one-shot. The active chart reads it
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
