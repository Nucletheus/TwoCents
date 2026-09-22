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

    let filtered = filter_expenses(expenses, state, &excluded_category_labels(categories));
    let comparisons = compare_budget_vs_actual(&filtered, budgets, categories);

    if comparisons.is_empty() {
        render_empty_state(
            ui,
            "No budget or spending data",
            Some("Set budgets and add expenses to see comparison"),
        );
        return;
    }

    let has_actual = comparisons.iter().any(|c| c.actual > 0.0);
    if !has_actual {
        render_empty_state(
            ui,
            "No spending in selected period",
            Some("Add expenses to compare against your budgets"),
        );
        return;
    }

    // Capture the palette up-front. `plot.show` mutably borrows `ui`,
    // so we can't reach for color accessors inside the closure.
    let budget_color = crate::ui::components::border_strong(ui);
    let success = crate::ui::components::success_color(ui);
    let error = crate::ui::components::error_color(ui);
    let fg_default = crate::ui::components::fg_default(ui);

    // ponytail: cap the chart+panel row at the available viewport height
    // so the page never grows taller than the window (kills the blank
    // scroll area). The chart keeps its fixed height and the right panel
    // scrolls internally.
    let avail = ui.available_size_before_wrap();
    let row_height = (avail.y - 16.0).max(360.0);
    ui.allocate_ui_with_layout(
        egui::vec2(avail.x, row_height),
        egui::Layout::left_to_right(egui::Align::TOP),
        |ui| {
            // Left side: chart. One BarChart per category so the x-axis
            // shows category names. The chart's auto-legend is hidden —
            // the right panel has a custom legend that uses theme colors.
            ui.vertical(|ui| {
                ui.set_max_height(row_height);
                ui.set_width(avail.x - 320.0);

                let max_budget = comparisons
                    .iter()
                    .map(|c| c.budgeted.max(c.actual))
                    .fold(0.0_f64, f64::max);
                let y_ceiling = if max_budget <= 0.0 { 100.0 } else { max_budget * 1.1 };

                let cat_labels: Vec<String> =
                    comparisons.iter().map(|c| subcategory_label(&c.category)).collect();
                let x_formatter =
                    move |x: egui_plot::GridMark, _r: &std::ops::RangeInclusive<f64>| -> String {
                        let idx = x.value.round() as usize;
                        cat_labels.get(idx).cloned().unwrap_or_default()
                    };

                let mut cat_color: std::collections::HashMap<String, Color32> =
                    std::collections::HashMap::new();
                for comp in comparisons.iter() {
                    cat_color.entry(comp.category.clone()).or_insert(comp.color);
                }
                // Capture the muted color up-front so we don't try to
                // borrow ui immutably inside the plot closure.
                let budget_color = crate::ui::components::border_strong(ui);
                // ponytail: thick zero line color, resolved up-front
                // (can't borrow ui inside the plot closure).
                let zero_color = budget_color;

                let plot = Plot::new("budget_vs_actual_plot")
                    .height(400.0)
                    .allow_zoom(true)
                    .allow_scroll(true)
                    .default_y_bounds(0.0, y_ceiling)
                    .show_grid([false, false])
                    .x_axis_formatter(x_formatter)
                    .label_formatter(|name, value| format!("{}: ${:.2}", name, value.y));

                let n = comparisons.len() as f64;
                let default_bounds =
                    egui_plot::PlotBounds::from_min_max([0.0, 0.0], [n, y_ceiling]);
                plot.show(ui, |plot_ui| {
                    // Shared chart chrome: thick zero line + Reset View.
                    // No negative tint (budget/actual are both >= 0).
                    super::charts_common::apply_zero_line_and_bounds(
                        plot_ui,
                        default_bounds,
                        None,
                        zero_color,
                        state.reset_view,
                    );
                    for (i, comp) in comparisons.iter().enumerate() {
                        let x = i as f64;
                        let actual_color = cat_color
                            .get(&comp.category)
                            .copied()
                            .unwrap_or(budget_color);
                        let budget_outline = crate::ui::components::darker(budget_color, 0.18);
                        let actual_outline = crate::ui::components::darker(actual_color, 0.18);
                        let budget_bar = Bar::new(x - 0.18, comp.budgeted)
                            .width(0.35)
                            .fill(budget_color)
                            .stroke(egui::Stroke::new(1.0_f32, budget_outline))
                            .name("Budget");
                        let actual_bar = Bar::new(x + 0.18, comp.actual)
                            .width(0.35)
                            .fill(actual_color)
                            .stroke(egui::Stroke::new(1.0_f32, actual_outline))
                            .name("Actual");
                        let chart = BarChart::new(&comp.category, vec![budget_bar, actual_bar]);
                        plot_ui.bar_chart(chart);
                    }
                });
            });

            ui.add_space(crate::ui::theme_tokens::SPACE_5);

            // Right side: themed summary card. Bounded to the row height
            // and scrolls internally so it can't push the layout taller
            // than the viewport.
            ui.vertical(|ui| {
                ui.set_max_height(row_height);
                ui.set_width(300.0);
                crate::ui::components::card(ui).show(ui, |ui| {
                    ui.set_max_height(row_height - 24.0);
                    egui::ScrollArea::vertical()
                        .id_salt("budget_summary_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            crate::ui::components::heading_lg(ui, "Budget vs Actual");
                            ui.add_space(crate::ui::theme_tokens::SPACE_1);
                            crate::ui::components::label_muted(
                                ui,
                                "Bars compare monthly budget against actual spend, per category.",
                            );
                            ui.add_space(crate::ui::theme_tokens::SPACE_3);

                            legend_row(ui, budget_color, "Budget");
                            legend_row(ui, success, "Under budget");
                            legend_row(ui, error, "Over budget");

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

                            ui.add_space(crate::ui::theme_tokens::SPACE_3);
                            ui.separator();
                            ui.add_space(crate::ui::theme_tokens::SPACE_2);

                            crate::ui::components::section_header(ui, "Categories");
                            for comp in &comparisons {
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(12.0, 12.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, comp.color);
                                    ui.add_space(crate::ui::theme_tokens::SPACE_1);
                                    let display_name = if comp.category.len() > 20 {
                                        format!("{}…", &comp.category[..17])
                                    } else {
                                        comp.category.clone()
                                    };
                                    ui.label(display_name);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let variance_text = if comp.variance >= 0.0 {
                                                format!("↑${:.2}", comp.variance.abs())
                                            } else {
                                                format!("↓${:.2}", comp.variance.abs())
                                            };
                                            let variance_color =
                                                if comp.variance >= 0.0 { success } else { error };
                                            ui.label(
                                                RichText::new(variance_text).color(variance_color),
                                            );
                                        },
                                    );
                                });
                                ui.add_space(crate::ui::theme_tokens::SPACE_1);
                            }
                        });
                });
            });
        },
    );
}

fn legend_row(ui: &mut egui::Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, color);
        ui.add_space(crate::ui::theme_tokens::SPACE_1);
        ui.label(label);
    });
}

fn stat_row(ui: &mut egui::Ui, label: &str, value: String, value_color: Color32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(value_color));
        });
    });
}

// ponytail: x-axis gets the bare subcategory name (after the " › " separator)
// so dense budgets don't overflow the axis. The full "Parent › Sub" name is
// still listed in the right panel and on hover.
fn subcategory_label(full: &str) -> String {
    match full.rfind(crate::models::CATEGORY_LABEL_SEP) {
        Some(i) => full[i + crate::models::CATEGORY_LABEL_SEP.len()..].to_string(),
        None => full.to_string(),
    }
}
