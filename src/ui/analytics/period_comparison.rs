use chrono::NaiveDate;
use eframe::egui::{self, RichText};
use egui_plot::{Bar, BarChart, Line, Plot};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use super::aggregation::*;
use super::charts_common::{comparison_periods, period_text};
use super::empty_state::render_empty_state;
use super::picker::{render_category_picker, render_legend_sort, PickerRow};
use super::state::*;
use crate::models::*;

/// Direction color: green = movement in the category's good direction —
/// spending down, income up. Near-equal rows are neutral.
fn direction_color(
    categories: &[Category],
    label: &str,
    a: f64,
    b: f64,
    success: eframe::egui::Color32,
    error: eframe::egui::Color32,
    neutral: eframe::egui::Color32,
) -> eframe::egui::Color32 {
    if (b - a).abs() < 0.005 {
        return neutral;
    }
    let good = if category_sign(categories, label) == 1 {
        b > a
    } else {
        b < a
    };
    if good {
        success
    } else {
        error
    }
}

pub fn render_period_comparison_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    categories: &[Category],
    state: &mut AnalyticsState,
) -> bool {
    let mut changed = false;

    if expenses.is_empty() {
        render_empty_state(
            ui,
            "No transactions recorded",
            Some("Add some income or expenses to compare periods"),
        );
        return false;
    }

    // Granularity tabs. Indices are clamped below — every granularity
    // builds the same list length, so switching never invalidates them.
    ui.horizontal_wrapped(|ui| {
        crate::ui::components::label_strong(ui, "Granularity:");
        for gran in [
            BudgetViewPeriod::Week,
            BudgetViewPeriod::Month,
            BudgetViewPeriod::Quarter,
            BudgetViewPeriod::Year,
        ] {
            if crate::ui::components::tab_label_button(
                ui,
                state.comparison_granularity == gran,
                gran.label(),
            )
            .clicked()
            {
                state.comparison_granularity = gran;
                changed = true;
            }
        }
    });

    // Built AFTER the granularity row so a switch this frame shows fresh
    // labels immediately.
    let periods = comparison_periods(state.comparison_granularity);
    state.comparison_period_a = state.comparison_period_a.min(periods.len() - 1);
    state.comparison_period_b = state.comparison_period_b.min(periods.len() - 1);
    let pa = periods[state.comparison_period_a].clone();
    let pb = periods[state.comparison_period_b].clone();

    ui.horizontal_wrapped(|ui| {
        crate::ui::components::label_strong(ui, "Period A:");
        egui::ComboBox::from_id_salt("period_a_sel")
            .selected_text(period_text(&pa.label, pa.start, pa.end))
            .show_ui(ui, |ui| {
                for (i, p) in periods.iter().enumerate() {
                    if ui
                        .selectable_label(
                            state.comparison_period_a == i,
                            period_text(&p.label, p.start, p.end),
                        )
                        .clicked()
                    {
                        state.comparison_period_a = i;
                        changed = true;
                    }
                }
            });

        ui.add_space(crate::ui::theme_tokens::SPACE_3);
        crate::ui::components::label_strong(ui, "Period B:");
        egui::ComboBox::from_id_salt("period_b_sel")
            .selected_text(period_text(&pb.label, pb.start, pb.end))
            .show_ui(ui, |ui| {
                for (i, p) in periods.iter().enumerate() {
                    if ui
                        .selectable_label(
                            state.comparison_period_b == i,
                            period_text(&p.label, p.start, p.end),
                        )
                        .clicked()
                    {
                        state.comparison_period_b = i;
                        changed = true;
                    }
                }
            });
    });

    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    // label -> color lookup so each bar is filled with the
    // user-assigned category color, matching the picker swatches below.
    let cat_parents = category_parent_map(categories);
    let mut cat_color: HashMap<String, eframe::egui::Color32> = HashMap::new();
    for category in categories.iter() {
        let label = category.full_label(&cat_parents);
        cat_color.insert(label, category.color);
    }
    // shared chart chrome colors, resolved up-front (can't
    // borrow ui inside plot closures).
    let fallback_color = crate::ui::theme::fg_secondary();
    let zero_color = crate::ui::theme::border_strong();
    let success = crate::ui::theme::success();
    let error = crate::ui::theme::error();
    let fg_default = crate::ui::theme::fg_primary();

    // PC opts into income (positive) rows — the one chart that
    // shows them; every other analytics tab stays spending-only. Date
    // window comes from each period's range; excluded categories and the
    // category/member/vendor selections still apply.
    let excluded = excluded_category_labels(categories);
    let mut scope = state.clone();
    scope.date_start = None;
    scope.date_end = None;
    scope.date_preset = DatePreset::AllTime;
    scope.include_income = true;
    let pool = filter_expenses(expenses, &scope, &excluded);

    // picker rows come from a pool with the category selection
    // cleared (same trick as the Breakdown legend) — rows built from the
    // filtered set collapsed to whatever was already selected, so the
    // whole list vanished the moment you clicked one. Member/vendor/date
    // filters still apply; only the category toggle is excluded.
    let mut picker_scope = scope.clone();
    picker_scope.selected_categories.clear();
    let picker_pool = filter_expenses(expenses, &picker_scope, &excluded);

    let in_range = |e: &Expense, s: NaiveDate, en: NaiveDate| {
        parse_expense_date(&e.date).is_some_and(|d| d >= s && d <= en)
    };
    let split = |pool: &[Expense]| -> (Vec<Expense>, Vec<Expense>) {
        (
            pool.iter()
                .filter(|e| in_range(e, pa.start, pa.end))
                .cloned()
                .collect(),
            pool.iter()
                .filter(|e| in_range(e, pb.start, pb.end))
                .cloned()
                .collect(),
        )
    };
    let (filtered_a, filtered_b) = split(&pool);
    let (picker_a, picker_b) = split(&picker_pool);

    let labels = (format!("A · {}", pa.label), format!("B · {}", pb.label));
    let mut data =
        aggregate_period_comparison(&filtered_a, &filtered_b, &labels.0, &labels.1, categories);
    let mut picker_data =
        aggregate_period_comparison(&picker_a, &picker_b, &labels.0, &labels.1, categories);

    // empty state keys off the UNFILTERED rows — an empty
    // *filtered* result still renders the shell + picker below so the
    // user can unclick the selection (returning early here is what made
    // the whole list disappear on click).
    if picker_data.category_comparisons.is_empty() {
        render_empty_state(
            ui,
            "No transactions in either period",
            Some("Try different periods or add transactions"),
        );
        return false;
    }

    // Shared Category/Cost sort — same selection as the other two tabs;
    // chart rows + picker rows both ride it, applied BEFORE the reseed
    // sig so toggling sort re-fits the plot.
    super::picker::order_rows_by_legend_sort(
        categories,
        &mut data.category_comparisons,
        state.legend_sort,
        |c| &c.category,
        |c| c.period_b_amount,
    );
    super::picker::order_rows_by_legend_sort(
        categories,
        &mut picker_data.category_comparisons,
        state.legend_sort,
        |c| &c.category,
        |c| c.period_b_amount,
    );

    // data/filters signature → Plot::reset() when it changes
    // (see take_reseed). Labels catch granularity/period switches; the
    // per-category amounts catch every filter change.
    let mut sh = std::collections::hash_map::DefaultHasher::new();
    data.period_a_label.hash(&mut sh);
    data.period_b_label.hash(&mut sh);
    for c in &data.category_comparisons {
        c.category.hash(&mut sh);
        c.period_a_amount.to_bits().hash(&mut sh);
        c.period_b_amount.to_bits().hash(&mut sh);
    }
    let reseed =
        super::charts_common::take_reseed(&mut state.pc_sig, sh.finish(), state.reset_view);

    // Summary strip at the TOP — pinning it below the chart is what kept
    // the plot from filling its container.
    render_summary_strip(ui, &data);
    crate::ui::components::label_muted(
        ui,
        &format!(
            "Bars: {} (top) vs {} (bottom); green = good direction (spend ↓, income ↑)",
            data.period_a_label, data.period_b_label
        ),
    );
    crate::ui::components::label_muted(
        ui,
        "Connector width = proportional change; widens toward Period B.",
    );
    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    // Chart + picker column take the whole remaining viewport — no
    // bottom-strip reserve, so nothing leaves white space at the bottom.
    let avail = ui.available_size_before_wrap();
    let row_height = (avail.y - 8.0).max(280.0);
    ui.allocate_ui_with_layout(
        egui::vec2(avail.x, row_height),
        egui::Layout::left_to_right(egui::Align::TOP),
        |ui| {
            // Left side: the paired horizontal chart, filling the column.
            ui.vertical(|ui| {
                ui.set_max_height(row_height);
                ui.set_width(avail.x - 320.0);
                if data.category_comparisons.is_empty() {
                    // Filtered to nothing, but picker rows still exist —
                    // message here, picker stays reachable on the right.
                    render_empty_state(
                        ui,
                        "No categories match the current filters",
                        Some("Adjust the category/member/vendor filters"),
                    );
                    return;
                }
                render_paired_chart(
                    ui,
                    &data,
                    categories,
                    &cat_color,
                    fallback_color,
                    zero_color,
                    success,
                    error,
                    fg_default,
                    row_height,
                    reseed,
                );
            });

            ui.add_space(crate::ui::theme_tokens::SPACE_5);

            // Right side: the shared category picker (direction-colored
            // |B| − |A| per category), filling the column height.
            ui.vertical(|ui| {
                ui.set_max_height(row_height);
                ui.set_width(300.0);
                // Same shared Category/Cost sort row as the other tabs.
                if render_legend_sort(ui, state) {
                    changed = true;
                }
                // Height left AFTER the sort row — the picker fills it.
                let picker_h = ui.available_size_before_wrap().y.max(120.0);
                let picker_rows: Vec<PickerRow> = picker_data
                    .category_comparisons
                    .iter()
                    .map(|c| {
                        let near_zero = c.difference.abs() < 0.005;
                        let value = if near_zero {
                            "$0.00".to_string()
                        } else if c.difference >= 0.0 {
                            format!("+${:.2}", c.difference)
                        } else {
                            format!("-${:.2}", c.difference.abs())
                        };
                        let value_color = direction_color(
                            categories,
                            &c.category,
                            c.period_a_amount,
                            c.period_b_amount,
                            success,
                            error,
                            fg_default,
                        );
                        PickerRow {
                            label: c.category.clone(),
                            color: c.color,
                            value: Some(value),
                            value_color: Some(value_color),
                        }
                    })
                    .collect();
                if render_category_picker(ui, state, &picker_rows, picker_h) {
                    changed = true;
                }
            });
        },
    );

    changed
}

/// Compact A/B/Difference stats above the chart (moved up from the old
/// bottom summary so the plot can fill the container). Totals are gross
/// magnitudes (Σ |per-category|).
fn render_summary_strip(ui: &mut egui::Ui, data: &PeriodComparisonData) {
    let fg_default = crate::ui::theme::fg_primary();

    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!(
                "{}: ${:.2}",
                data.period_a_label, data.period_a_total
            ))
            .color(fg_default),
        );
        ui.separator();
        ui.label(
            RichText::new(format!(
                "{}: ${:.2}",
                data.period_b_label, data.period_b_total
            ))
            .color(fg_default),
        );
        ui.separator();

        // A == B leaves a float-epsilon difference that rendered
        // as a red "-$0.00 (0.0%)" — below half a cent is zero, and zero
        // is neutral, not error. Totals mix income + spending categories,
        // so the overall difference has NO single good direction — it stays
        // neutral here; per-row direction colors live on the chart and
        // picker (which know each category's sign).
        let near_zero = data.difference.abs() < 0.005;
        let diff_text = if near_zero {
            "$0.00 (0.0%)".to_string()
        } else if data.difference >= 0.0 {
            format!("+${:.2} (+{:.1}%)", data.difference, data.difference_pct)
        } else {
            format!(
                "-${:.2} ({:.1}%)",
                data.difference.abs(),
                data.difference_pct.abs()
            )
        };
        ui.label(RichText::new(format!("Difference: {}", diff_text)).color(fg_default));
    });
}

/// The one Period Comparison chart: categories down the y-axis (sorted
/// biggest-at-top), period A bar above period B bar per row, both
/// extending right on a log $ axis (ln1p), joined by a connector line
/// between the bar tips colored by direction.
#[allow(clippy::too_many_arguments)]
fn render_paired_chart(
    ui: &mut egui::Ui,
    data: &PeriodComparisonData,
    categories: &[Category],
    cat_color: &HashMap<String, eframe::egui::Color32>,
    fallback: eframe::egui::Color32,
    zero_color: eframe::egui::Color32,
    success: eframe::egui::Color32,
    error: eframe::egui::Color32,
    neutral: eframe::egui::Color32,
    chart_h: f32,
    reseed: bool,
) {
    let n = data.category_comparisons.len();
    let max_change = data
        .category_comparisons
        .iter()
        .map(|cat| {
            (super::charts_common::ln1p(cat.period_b_amount)
                - super::charts_common::ln1p(cat.period_a_amount))
            .abs()
        })
        .fold(0.0_f64, f64::max);
    let grid_color = crate::ui::theme::fg_secondary().gamma_multiply(0.5);
    // y = n-1-i puts sorted-desc index 0 at the TOP of the axis.
    let labels_by_y: Vec<String> = data
        .category_comparisons
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

    // Direction colors per row, resolved outside the plot closure.
    let dir_colors: Vec<eframe::egui::Color32> = data
        .category_comparisons
        .iter()
        .map(|c| {
            direction_color(
                categories,
                &c.category,
                c.period_a_amount,
                c.period_b_amount,
                success,
                error,
                neutral,
            )
        })
        .collect();

    let plot = Plot::new("period_comparison_plot")
        .height(chart_h)
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
        plot_ui.vline(
            egui_plot::VLine::new("zero", 0.0)
                .width(1.5_f32)
                .color(zero_color),
        );
        for (i, cat) in data.category_comparisons.iter().enumerate() {
            let y = (n - 1 - i) as f64;
            let base = cat_color.get(&cat.category).copied().unwrap_or(fallback);
            let a_color = base.gamma_multiply(0.55);
            let a_x = super::charts_common::ln1p(cat.period_a_amount);
            let b_x = super::charts_common::ln1p(cat.period_b_amount);
            let connector_from = [a_x, y + 0.18];
            let connector_to = [b_x, y - 0.18];
            let connector_widths =
                super::charts_common::comparison_connector_widths((b_x - a_x).abs(), max_change);
            for (segment, width) in connector_widths.iter().enumerate() {
                let segment_start = segment as f64 / connector_widths.len() as f64;
                let segment_end = (segment + 1) as f64 / connector_widths.len() as f64;
                let start = [
                    connector_from[0] + (connector_to[0] - connector_from[0]) * segment_start,
                    connector_from[1] + (connector_to[1] - connector_from[1]) * segment_start,
                ];
                let end = [
                    connector_from[0] + (connector_to[0] - connector_from[0]) * segment_end,
                    connector_from[1] + (connector_to[1] - connector_from[1]) * segment_end,
                ];
                plot_ui.line(
                    Line::new(format!("conn_{i}_{segment}"), vec![start, end])
                        .color(dir_colors[i])
                        .width(*width)
                        .allow_hover(false),
                );
            }
            let bar_a = Bar::new(y + 0.18, a_x)
                .width(0.32)
                .fill(a_color)
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    crate::ui::components::darker(a_color, 0.18),
                ))
                .name("A");
            let bar_b = Bar::new(y - 0.18, b_x)
                .width(0.32)
                .fill(base)
                .stroke(egui::Stroke::new(
                    1.0_f32,
                    crate::ui::components::darker(base, 0.18),
                ))
                .name("B");
            // bar hover bypasses the plot-level label_formatter
            // (egui_plot 0.37 routes bars through add_rulers_and_text) —
            // its default text showed the raw ln-space coordinate.
            // element_formatter is the only hook for bars; A/B dollar
            // values + the signed difference (sign = direction).
            let category = cat.category.clone();
            let (a_label, b_label) = (data.period_a_label.clone(), data.period_b_label.clone());
            let (a_amt, b_amt) = (cat.period_a_amount, cat.period_b_amount);
            let diff_text = if cat.difference.abs() < 0.005 {
                "$0.00".to_string()
            } else if cat.difference >= 0.0 {
                format!("+${:.2}", cat.difference)
            } else {
                format!("-${:.2}", cat.difference.abs())
            };
            plot_ui.bar_chart(
                BarChart::new(&cat.category, vec![bar_a, bar_b])
                    .horizontal()
                    .element_formatter(Box::new(move |_bar, _chart| {
                        format!(
                            "{}\n{}: ${:.2}\n{}: ${:.2}\nDifference: {}",
                            category, a_label, a_amt, b_label, b_amt, diff_text
                        )
                    })),
            );
        }
    });
}
