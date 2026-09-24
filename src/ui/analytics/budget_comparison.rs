use eframe::egui::{self, Color32, RichText};
use egui_plot::{Bar, BarChart, Line, Plot};
use std::hash::{Hash, Hasher};

use super::aggregation::*;
use super::charts_common::{comparison_periods, period_text};
use super::empty_state::{render_empty_state, render_no_budgets_state};
use super::state::*;
use crate::models::*;

/// Everything the Budget vs Actual panel needs for ONE viewed period,
/// pre-computed in mod.rs where the app's snapshot cache is reachable.
pub struct BudgetPeriodBudgets {
    pub budgets: std::collections::HashMap<String, i64>,
    pub start: chrono::NaiveDate,
    pub end: chrono::NaiveDate,
    pub period_label: &'static str,
    pub period_name: String,
}

pub fn render_budget_vs_actual_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    priced_budgets: &BudgetPeriodBudgets,
    categories: &[Category],
    state: &mut AnalyticsState,
) -> bool {
    if priced_budgets.budgets.is_empty() {
        render_no_budgets_state(ui);
        return false;
    }

    // route through the shared filter like the other two charts —
    // raw expenses mixed income rows and excluded categories (transfers)
    // into the actuals. Date window = THIS budget period, not the global
    // filter range (same scope trick Period Comparison uses).
    let excluded = excluded_category_labels(categories);
    let mut scope = state.clone();
    scope.date_start = Some(priced_budgets.start);
    scope.date_end = Some(priced_budgets.end);
    let pool = filter_expenses(expenses, &scope, &excluded);

    // the budget side must honor the category selection too —
    // unioning unfiltered budgets with filtered actuals left every budget
    // bar in place when you clicked a category (looked like a no-op).
    let filtered_budgets: std::collections::HashMap<String, i64> = priced_budgets
        .budgets
        .iter()
        .filter(|(cat, _)| passes_category_filter(state, cat))
        .map(|(cat, amt)| (cat.clone(), *amt))
        .collect();
    let mut comparisons = compare_budget_vs_actual(&pool, &filtered_budgets, categories);

    // Shared Category/Cost sort — same selection as the other two tabs.
    // Rows are ordered BEFORE the reseed sig below, so toggling sort
    // re-fits the plot to the new order.
    super::picker::order_rows_by_legend_sort(
        categories,
        &mut comparisons,
        state.legend_sort,
        |c| &c.category,
        |c| c.actual,
    );

    // data/filters signature → Plot::reset() when it changes.
    // egui_plot freezes auto-bounds on first pan/zoom, so a stale view
    // never re-fits new data; reset re-seeds auto-fit, manual zoom
    // persists until the next change (or the Reset View flag).
    let mut sh = std::collections::hash_map::DefaultHasher::new();
    priced_budgets.period_name.hash(&mut sh);
    for c in &comparisons {
        c.category.hash(&mut sh);
        c.budgeted.to_bits().hash(&mut sh);
        c.actual.to_bits().hash(&mut sh);
    }
    let reseed =
        super::charts_common::take_reseed(&mut state.bva_sig, sh.finish(), state.reset_view);

    // picker rows come from a pool with the category selection
    // cleared (same trick as the Breakdown legend) — building them from the
    // filtered set collapsed the list down to what was already selected, so
    // you could never click the next row, and an empty filtered result
    // returned an empty state before the picker rendered at all.
    let mut picker_scope = scope.clone();
    picker_scope.selected_categories.clear();
    let picker_pool = filter_expenses(expenses, &picker_scope, &excluded);
    let mut row_comparisons =
        compare_budget_vs_actual(&picker_pool, &priced_budgets.budgets, categories);
    super::picker::order_rows_by_legend_sort(
        categories,
        &mut row_comparisons,
        state.legend_sort,
        |c| &c.category,
        |c| c.actual,
    );

    // Granularity tabs (Week / Month / Quarter / Year) + a Period combo
    // walking 24 same-granularity periods back — same controls as Period
    // Comparison. Budgets/actuals re-price next frame from the new index
    // (mod.rs prices per frame; input events always repaint).
    let periods = comparison_periods(state.budget_view_period);
    state.budget_period_index = state.budget_period_index.min(periods.len() - 1);
    let sel = &periods[state.budget_period_index];
    ui.horizontal_wrapped(|ui| {
        crate::ui::components::label_strong(ui, "Granularity:");
        for option in [
            BudgetViewPeriod::Week,
            BudgetViewPeriod::Month,
            BudgetViewPeriod::Quarter,
            BudgetViewPeriod::Year,
        ] {
            if crate::ui::components::tab_label_button(
                ui,
                state.budget_view_period == option,
                option.label(),
            )
            .clicked()
            {
                state.budget_view_period = option;
            }
        }
        ui.add_space(crate::ui::theme_tokens::SPACE_3);
        crate::ui::components::label_strong(ui, "Period:");
        egui::ComboBox::from_id_salt("bva_period_sel")
            .selected_text(period_text(&sel.label, sel.start, sel.end))
            .show_ui(ui, |ui| {
                for (i, p) in periods.iter().enumerate() {
                    if ui
                        .selectable_label(
                            state.budget_period_index == i,
                            period_text(&p.label, p.start, p.end),
                        )
                        .clicked()
                    {
                        state.budget_period_index = i;
                    }
                }
            });
    });
    crate::ui::components::label_muted(
        ui,
        &format!(
            "Comparing {} {} budget vs actual spend ({} – {})",
            priced_budgets.period_label,
            priced_budgets.period_name,
            priced_budgets.start.format("%b %d, %Y"),
            priced_budgets.end.format("%b %d, %Y")
        ),
    );
    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    // Capture the palette up-front. `plot.show` mutably borrows `ui`,
    // so we can't reach for color accessors inside the closure.
    let zero_color = crate::ui::theme::border_strong();
    let grid_color = crate::ui::theme::fg_secondary().gamma_multiply(0.5);
    let success = crate::ui::theme::success();
    let error = crate::ui::theme::error();
    let fg_default = crate::ui::theme::fg_primary();

    // chart + right column take the whole remaining viewport —
    // the picker lives in the right column (under the summary card) now,
    // so nothing is reserved at the bottom.
    let avail = ui.available_size_before_wrap();
    let row_height = (avail.y - 16.0).max(320.0);
    let mut changed = false;
    ui.allocate_ui_with_layout(
        egui::vec2(avail.x, row_height),
        egui::Layout::left_to_right(egui::Align::TOP),
        |ui| {
            // Left side: horizontal paired bars — categories down the
            // y-axis (sorted biggest-at-top), budget bar above actual bar,
            // both extending right on a log $ axis (ln1p transform).
            ui.vertical(|ui| {
                ui.set_max_height(row_height);
                ui.set_width(avail.x - 320.0);

                // the category filter can empty the chart side
                // (no budget/spend matches) while the picker rows above
                // still list every category — show a message here instead
                // of returning early, so the picker stays reachable.
                if comparisons.is_empty() {
                    render_empty_state(
                        ui,
                        "No budget or spending data",
                        Some("Set budgets and add expenses to see comparison"),
                    );
                    return;
                }

                let n = comparisons.len();
                let max_change = comparisons
                    .iter()
                    .map(|comp| {
                        (super::charts_common::ln1p(comp.actual)
                            - super::charts_common::ln1p(comp.budgeted))
                        .abs()
                    })
                    .fold(0.0_f64, f64::max);
                // y = n-1-i puts sorted-desc index 0 at the TOP of the axis;
                // labels indexed the same way for the y formatter.
                let labels_by_y: Vec<String> = comparisons
                    .iter()
                    .rev()
                    .map(|c| super::charts_common::subcategory_label(&c.category))
                    .collect();
                let y_formatter = move |m: egui_plot::GridMark, _r: &std::ops::RangeInclusive<f64>| {
                    let k = m.value.round();
                    if (m.value - k).abs() < 0.01 && k >= 0.0 && (k as usize) < labels_by_y.len() {
                        labels_by_y[k as usize].clone()
                    } else {
                        String::new()
                    }
                };

                let plot = Plot::new("budget_vs_actual_plot")
                    .height(row_height)
                    .allow_zoom(true)
                    .allow_scroll(true)
                    .show_grid([true, false])
                    .grid_color(grid_color)
                    .x_grid_spacer(super::charts_common::money_grid_spacer)
                    .y_grid_spacer(super::charts_common::category_grid_spacer)
                    .custom_x_axes(vec![super::charts_common::money_axis_hints()])
                    .y_axis_formatter(y_formatter);
                let plot = if reseed { plot.reset() } else { plot };

                plot.show(ui, |plot_ui| {
                    // Thick zero line (value axis is x for horizontal bars).
                    plot_ui.vline(egui_plot::VLine::new("zero", 0.0).width(1.5_f32).color(zero_color));
                    for (i, comp) in comparisons.iter().enumerate() {
                        let y = (n - 1 - i) as f64;
                        // PC's exact scheme: both bars in the category's
                        // color (budget dimmed 55%, actual full); the
                        // connector between the tips carries direction —
                        // green under budget, red over.
                        let base = comp.color;
                        let dim = base.gamma_multiply(0.55);
                        let budget_x = super::charts_common::ln1p(comp.budgeted);
                        let actual_x = super::charts_common::ln1p(comp.actual);
                        let conn_color = if comp.variance >= 0.0 { success } else { error };
                        let connector_from = [budget_x, y + 0.18];
                        let connector_to = [actual_x, y - 0.18];
                        let connector_widths = super::charts_common::comparison_connector_widths(
                            (actual_x - budget_x).abs(),
                            max_change,
                        );
                        for (segment, width) in connector_widths.iter().enumerate() {
                            let segment_start = segment as f64 / connector_widths.len() as f64;
                            let segment_end = (segment + 1) as f64 / connector_widths.len() as f64;
                            let start = [
                                connector_from[0]
                                    + (connector_to[0] - connector_from[0]) * segment_start,
                                connector_from[1]
                                    + (connector_to[1] - connector_from[1]) * segment_start,
                            ];
                            let end = [
                                connector_from[0]
                                    + (connector_to[0] - connector_from[0]) * segment_end,
                                connector_from[1]
                                    + (connector_to[1] - connector_from[1]) * segment_end,
                            ];
                            plot_ui.line(
                                Line::new(format!("conn_{i}_{segment}"), vec![start, end])
                                    .color(conn_color)
                                    .width(*width)
                                    .allow_hover(false),
                            );
                        }
                        let budget_bar = Bar::new(y + 0.18, budget_x)
                            .width(0.32)
                            .fill(dim)
                            .stroke(egui::Stroke::new(1.0_f32, crate::ui::components::darker(dim, 0.18)))
                            .name("Budget");
                        let actual_bar = Bar::new(y - 0.18, actual_x)
                            .width(0.32)
                            .fill(base)
                            .stroke(egui::Stroke::new(1.0_f32, crate::ui::components::darker(base, 0.18)))
                            .name("Actual");
                        // bar hover bypasses the plot-level
                        // label_formatter entirely (egui_plot 0.37 routes
                        // bars through add_rulers_and_text) — its default
                        // text showed the raw ln-space coordinate
                        // ("Budget 3.43"). element_formatter is the only
                        // hook for bars; dollar values + signed variance.
                        let category = comp.category.clone();
                        let (budgeted, actual) = (comp.budgeted, comp.actual);
                        let variance_text = if comp.variance >= 0.0 {
                            format!("+${:.2} under budget", comp.variance)
                        } else {
                            format!("-${:.2} over budget", comp.variance.abs())
                        };
                        plot_ui.bar_chart(
                            BarChart::new(&comp.category, vec![budget_bar, actual_bar])
                                .horizontal()
                                .element_formatter(Box::new(move |_bar, _chart| {
                                    format!(
                                        "{}\nBudget: ${:.2}\nActual: ${:.2}\nVariance: {}",
                                        category, budgeted, actual, variance_text
                                    )
                                })),
                        );
                    }
                });
            });

            ui.add_space(crate::ui::theme_tokens::SPACE_5);

            // Right side: summary card on top (natural height, capped so
            // the picker below always gets room), category picker filling
            // the rest of the column — matches the Breakdown side list.
            ui.vertical(|ui| {
                ui.set_max_height(row_height);
                ui.set_width(300.0);
                let card_cap = (row_height - 140.0).max(160.0);
                crate::ui::components::card(ui).show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("budget_summary_scroll")
                        .max_height(card_cap)
                        .show(ui, |ui| {
                            crate::ui::components::heading_lg(ui, "Budget vs Actual");
                            ui.add_space(crate::ui::theme_tokens::SPACE_1);
                            crate::ui::components::label_muted(
                                ui,
                                "Budget vs actual spend per category (log scale).",
                            );
                            crate::ui::components::label_muted(
                                ui,
                                "Bars: Budget (top) vs Actual (bottom); green = under budget, red = over",
                            );
                            crate::ui::components::label_muted(
                                ui,
                                "Connector width = proportional change; widens toward Actual.",
                            );
                            ui.add_space(crate::ui::theme_tokens::SPACE_3);

                            ui.separator();
                            ui.add_space(crate::ui::theme_tokens::SPACE_2);

                            crate::ui::components::section_header(ui, "Summary");
                            let total_budgeted: f64 =
                                comparisons.iter().map(|c| c.budgeted).sum();
                            let total_actual: f64 = comparisons.iter().map(|c| c.actual).sum();
                            let total_variance = total_budgeted - total_actual;

                            let variance_text = if total_variance >= 0.0 {
                                format!("+${:.2}", total_variance)
                            } else {
                                format!("-${:.2}", total_variance.abs())
                            };
                            let variance_color = if total_variance >= 0.0 { success } else { error };
                            stat_row(ui, "Total Budgeted", format!("${:.2}", total_budgeted), fg_default);
                            stat_row(ui, "Total Actual", format!("${:.2}", total_actual), fg_default);
                            stat_row(ui, "Variance", variance_text, variance_color);
                        });
                });

                ui.add_space(crate::ui::theme_tokens::SPACE_3);
                // Same shared Category/Cost sort row as the other tabs —
                // one selection orders every chart's rows + picker.
                if super::picker::render_legend_sort(ui, state) {
                    changed = true;
                }
                // Picker rows = unfiltered-by-category comparisons (full
                // list); values are per-category variance.
                let picker_h = ui.available_size_before_wrap().y.max(120.0);
                let picker_rows: Vec<super::picker::PickerRow> = row_comparisons
                    .iter()
                    .map(|comp| {
                        let (value, value_color) = if comp.variance >= 0.0 {
                            (format!("↑${:.2}", comp.variance.abs()), success)
                        } else {
                            (format!("↓${:.2}", comp.variance.abs()), error)
                        };
                        super::picker::PickerRow {
                            label: comp.category.clone(),
                            color: comp.color,
                            value: Some(value),
                            value_color: Some(value_color),
                        }
                    })
                    .collect();
                if super::picker::render_category_picker(ui, state, &picker_rows, picker_h) {
                    changed = true;
                }
            });
        },
    );
    changed
}

fn stat_row(ui: &mut egui::Ui, label: &str, value: String, value_color: Color32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(value_color));
        });
    });
}
