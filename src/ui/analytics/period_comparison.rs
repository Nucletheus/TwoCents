use eframe::egui::{self, RichText};
use egui_plot::{Bar, BarChart, Plot};
use chrono::NaiveDate;
use std::collections::HashMap;

use crate::models::*;
use super::aggregation::*;
use super::state::*;
use super::empty_state::render_empty_state;

/// ponytail: human-readable label for a DatePreset + the date range it
/// resolves to. "Last 3 Months (Apr 23 – Jul 23)" reads better than
/// just "Last 3 Months" when the user is choosing which period to
/// compare against.
fn preset_label_with_range(preset: DatePreset) -> String {
    let (start, end) = preset.date_range();
    match (start, end) {
        (Some(s), Some(e)) => format!("{} ({} – {})", preset.label(), s.format("%b %d"), e.format("%b %d")),
        _ => preset.label().to_string(),
    }
}

pub fn render_period_comparison_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    categories: &[Category],
    state: &mut AnalyticsState,
) {
    if expenses.is_empty() {
        render_empty_state(
            ui,
            "No expenses recorded",
            Some("Add some expenses to compare periods"),
        );
        return;
    }

    // ponytail: themed label helpers for the Period A/B/Mode row.
    ui.horizontal(|ui| {
        crate::ui::components::label_strong(ui, "Period A:");
        egui::ComboBox::from_id_salt("period_a_preset")
            .selected_text(preset_label_with_range(state.comparison_period_a_preset))
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
                    let label = preset_label_with_range(preset);
                    if ui
                        .selectable_label(state.comparison_period_a_preset == preset, label)
                        .clicked()
                    {
                        state.comparison_period_a_preset = preset;
                    }
                }
            });

        ui.add_space(crate::ui::theme_tokens::SPACE_3);
        crate::ui::components::label_strong(ui, "Period B:");
        egui::ComboBox::from_id_salt("period_b_preset")
            .selected_text(preset_label_with_range(state.comparison_period_b_preset))
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
                    let label = preset_label_with_range(preset);
                    if ui
                        .selectable_label(state.comparison_period_b_preset == preset, label)
                        .clicked()
                    {
                        state.comparison_period_b_preset = preset;
                    }
                }
            });

        ui.add_space(crate::ui::theme_tokens::SPACE_3);
        crate::ui::components::label_strong(ui, "Mode:");
        egui::ComboBox::from_id_salt("comparison_mode")
            .selected_text(state.comparison_mode.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.comparison_mode, ComparisonMode::SideBySide, "Side-by-Side");
                ui.selectable_value(&mut state.comparison_mode, ComparisonMode::Overlay, "Overlay");
                ui.selectable_value(&mut state.comparison_mode, ComparisonMode::Delta, "Delta");
            });
    });

    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    // ponytail: build a label -> color lookup so each bar can be
    // filled with the user-assigned category color. The previous
    // version used generic theme colors (fg_muted for Period A,
    // accent for Period B), which didn't tie the chart bars to the
    // swatches in the right panel. Now the chart's colors and the
    // right panel's swatches are pulled from the same map, so the
    // user can read the chart and look up the matching swatch
    // without guessing.
    let cat_parents = category_parent_map(categories);
    let mut cat_color: HashMap<String, eframe::egui::Color32> = HashMap::new();
    for category in categories.iter() {
        let label = category.full_label(&cat_parents);
        cat_color.insert(label, category.color);
    }
    // Orphan fallback for any category that shows up in the data
    // but isn't in the user's category list (e.g. a category that
    // was deleted after the spend happened).
    let fallback_color = crate::ui::components::fg_muted(ui);
    // ponytail: shared chart chrome colors, resolved up-front (can't
    // borrow ui inside plot closures).
    let zero_color = crate::ui::components::border_strong(ui);
    let neg_tint = crate::ui::components::error_color(ui);

    let (start_a, end_a) = state.comparison_period_a_preset.date_range();
    let (start_b, end_b) = state.comparison_period_b_preset.date_range();
    let start_a = start_a.unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let end_a = end_a.unwrap_or_else(|| chrono::Local::now().date_naive());
    let start_b = start_b.unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let end_b = end_b.unwrap_or_else(|| chrono::Local::now().date_naive());

    // ponytail: analytics are spending-only — skip income (positive rows)
    // and excluded transfer/payment categories.
    let excluded = excluded_category_labels(categories);
    let is_spend = |e: &Expense| e.amount_cents < 0 && !excluded.contains(&e.category);

    let filtered_a: Vec<Expense> = expenses
        .iter()
        .filter(|e| {
            is_spend(e)
                && parse_expense_date(&e.date)
                    .is_some_and(|date| date >= start_a && date <= end_a)
        })
        .cloned()
        .collect();
    let filtered_b: Vec<Expense> = expenses
        .iter()
        .filter(|e| {
            is_spend(e)
                && parse_expense_date(&e.date)
                    .is_some_and(|date| date >= start_b && date <= end_b)
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

    if data.period_a_total == 0.0 && data.period_b_total == 0.0 {
        render_empty_state(
            ui,
            "No spending in either period",
            Some("Try different date ranges or add expenses"),
        );
        return;
    }

    // Pass the shared chart chrome to each renderer so the bars pick up
    // the user-assigned category colors.
    let theme = PlotTheme {
        cat_color: &cat_color,
        zero_color,
        neg_tint,
        reset_view: state.reset_view,
    };
    match state.comparison_mode {
        ComparisonMode::SideBySide => render_side_by_side(ui, &data, &theme),
        ComparisonMode::Overlay => render_overlay(ui, &data, &theme),
        ComparisonMode::Delta => render_delta(ui, &data, &theme),
    }

    ui.add_space(crate::ui::theme_tokens::SPACE_3);
    render_summary(ui, &data, &cat_color, fallback_color);
}

/// ponytail: the shared chart chrome (category colors, zero line, negative
/// tint, Reset View) — one struct instead of the same 4 params on every
/// renderer signature.
struct PlotTheme<'a> {
    cat_color: &'a HashMap<String, eframe::egui::Color32>,
    zero_color: eframe::egui::Color32,
    neg_tint: eframe::egui::Color32,
    reset_view: bool,
}

fn render_side_by_side(
    ui: &mut egui::Ui,
    data: &PeriodComparisonData,
    theme: &PlotTheme,
) {
    ui.horizontal(|ui| {
        // The y-axis upper bound is shared across both sub-plots so
        // the user can compare bar heights at a glance.
        let max_value = data
            .category_comparisons
            .iter()
            .map(|c| c.period_a_amount.max(c.period_b_amount))
            .fold(0.0_f64, f64::max);
        let y_ceiling = if max_value <= 0.0 { 100.0 } else { max_value * 1.1 };
        let n = data.category_comparisons.len() as f64;
        let default_bounds = egui_plot::PlotBounds::from_min_max([0.0, 0.0], [n, y_ceiling]);

        ui.vertical(|ui| {
            crate::ui::components::heading_lg(ui, &data.period_a_label);
            crate::ui::components::label_muted(ui, &format!("Total ${:.2}", data.period_a_total));
            ui.add_space(crate::ui::theme_tokens::SPACE_2);
            render_single_period_plot(ui, data, "A", theme, y_ceiling);
        });
        ui.add_space(crate::ui::theme_tokens::SPACE_4);
        ui.vertical(|ui| {
            crate::ui::components::heading_lg(ui, &data.period_b_label);
            crate::ui::components::label_muted(ui, &format!("Total ${:.2}", data.period_b_total));
            ui.add_space(crate::ui::theme_tokens::SPACE_2);
            render_single_period_plot(ui, data, "B", theme, y_ceiling);
        });
    });
}

fn render_single_period_plot(
    ui: &mut egui::Ui,
    data: &PeriodComparisonData,
    period: &str,
    theme: &PlotTheme,
    y_ceiling: f64,
) {
    // ponytail: x-axis formatter — integer x maps to a category name.
    let cat_labels: Vec<String> = data
        .category_comparisons
        .iter()
        .map(|c| c.category.clone())
        .collect();
    let x_formatter = move |x: egui_plot::GridMark, _r: &std::ops::RangeInclusive<f64>| -> String {
        let idx = x.value.round() as usize;
        cat_labels.get(idx).cloned().unwrap_or_default()
    };

    let n = data.category_comparisons.len() as f64;
    let default_bounds = egui_plot::PlotBounds::from_min_max([0.0, 0.0], [n, y_ceiling]);
    let plot = Plot::new(format!("period_{period}_plot").as_str())
        .height(300.0)
        .default_y_bounds(0.0, y_ceiling)
        .show_grid([false, false])
        .x_axis_formatter(x_formatter);
    // Capture fg_muted up-front so we don't try to call it inside
    // the plot closure (which mutably borrows ui).
    let fallback = crate::ui::components::fg_muted(ui);
    plot.show(ui, |plot_ui| {
        // Shared chart chrome: thick zero line + Reset View. No negative
        // tint (period spend is all >= 0).
        super::charts_common::apply_zero_line_and_bounds(
            plot_ui,
            default_bounds,
            None,
            theme.zero_color,
            theme.reset_view,
        );
        // ponytail: one BarChart per category so the x-axis label is
        // the category name. Each chart's bar is filled with the
        // category's user-assigned color — the same color shown in
        // the right panel's swatches, so the user can map a chart
        // bar to a swatch at a glance.
        for (i, cat) in data.category_comparisons.iter().enumerate() {
            let amount = if period == "A" {
                cat.period_a_amount
            } else {
                cat.period_b_amount
            };
            let color = theme.cat_color.get(&cat.category).copied().unwrap_or(fallback);
            let outline = crate::ui::components::darker(color, 0.18);
            let bar = Bar::new(i as f64, amount)
                .name(&cat.category)
                .fill(color)
                .stroke(egui::Stroke::new(1.0_f32, outline))
                .width(0.8);
            plot_ui.bar_chart(BarChart::new(&cat.category, vec![bar]));
        }
    });
}

fn render_overlay(
    ui: &mut egui::Ui,
    data: &PeriodComparisonData,
    theme: &PlotTheme,
) {
    let max_value = data
        .category_comparisons
        .iter()
        .map(|c| c.period_a_amount.max(c.period_b_amount))
        .fold(0.0_f64, f64::max);
    let y_ceiling = if max_value <= 0.0 { 100.0 } else { max_value * 1.1 };
    let n = data.category_comparisons.len() as f64;
    let default_bounds = egui_plot::PlotBounds::from_min_max([0.0, 0.0], [n, y_ceiling]);

    let plot = Plot::new("overlay_plot")
        .height(400.0)
        .default_y_bounds(0.0, y_ceiling)
        .show_grid([false, false]);
    // Capture fg_muted up-front.
    let fallback = crate::ui::components::fg_muted(ui);
    plot.show(ui, |plot_ui| {
        // Shared chart chrome: thick zero line + Reset View. No negative
        // tint (period spend is all >= 0).
        super::charts_common::apply_zero_line_and_bounds(
            plot_ui,
            default_bounds,
            None,
            theme.zero_color,
            theme.reset_view,
        );
        // ponytail: one BarChart per category, each containing two
        // side-by-side bars (one for each period). Both bars use the
        // category's own color, but at slightly different opacities
        // so the user can tell Period A from Period B without a
        // separate color encoding.
        for (i, cat) in data.category_comparisons.iter().enumerate() {
            let x = i as f64;
            let base_color = theme.cat_color.get(&cat.category).copied().unwrap_or(fallback);
            // Period A: full opacity. Period B: 60% opacity. Same
            // hue, different lightness — the user can match them by
            // color but the opacity marks which period is which.
            let period_b_color = eframe::egui::Color32::from_rgba_unmultiplied(
                base_color.r(),
                base_color.g(),
                base_color.b(),
                (base_color.a() as f32 * 0.6) as u8,
            );
            let bar_a_outline = crate::ui::components::darker(base_color, 0.18);
            let bar_b_outline = crate::ui::components::darker(period_b_color, 0.18);
            let bar_a = Bar::new(x - 0.18, cat.period_a_amount)
                .name(&cat.category)
                .fill(base_color)
                .stroke(egui::Stroke::new(1.0_f32, bar_a_outline))
                .width(0.32);
            let bar_b = Bar::new(x + 0.18, cat.period_b_amount)
                .name(&cat.category)
                .fill(period_b_color)
                .stroke(egui::Stroke::new(1.0_f32, bar_b_outline))
                .width(0.32);
            let chart = BarChart::new(&cat.category, vec![bar_a, bar_b]);
            plot_ui.bar_chart(chart);
        }
    });
}

fn render_delta(
    ui: &mut egui::Ui,
    data: &PeriodComparisonData,
    theme: &PlotTheme,
) {
    crate::ui::components::heading_lg(ui, "Difference (B − A)");
    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    let max_abs = data
        .category_comparisons
        .iter()
        .map(|c| c.difference.abs())
        .fold(0.0_f64, f64::max);
    let y_ceiling = if max_abs <= 0.0 { 100.0 } else { max_abs * 1.1 };
    let n = data.category_comparisons.len() as f64;
    let default_bounds = egui_plot::PlotBounds::from_min_max([0.0, -y_ceiling], [n, y_ceiling]);

    let plot = Plot::new("delta_plot")
        .height(400.0)
        .default_y_bounds(-y_ceiling, y_ceiling)
        .show_grid([false, false])
        .x_axis_formatter(make_x_formatter(data));
    // Capture fg_muted up-front.
    let fallback = crate::ui::components::fg_muted(ui);
    plot.show(ui, |plot_ui| {
        // Shared chart chrome: thick zero line + negative tint + Reset View.
        // This is the one chart where values go negative, so we pass the
        // error-color tint to shade the region below zero.
        super::charts_common::apply_zero_line_and_bounds(
            plot_ui,
            default_bounds,
            Some(theme.neg_tint),
            theme.zero_color,
            theme.reset_view,
        );
        for (i, cat) in data.category_comparisons.iter().enumerate() {
            let color = theme.cat_color.get(&cat.category).copied().unwrap_or(fallback);
            let outline = crate::ui::components::darker(color, 0.18);
            let bar = Bar::new(i as f64, cat.difference)
                .name(&cat.category)
                .fill(color)
                .stroke(egui::Stroke::new(1.0_f32, outline))
                .width(0.8);
            plot_ui.bar_chart(BarChart::new(&cat.category, vec![bar]));
        }
    });
}

fn make_x_formatter(
    data: &PeriodComparisonData,
) -> impl Fn(egui_plot::GridMark, &std::ops::RangeInclusive<f64>) -> String + 'static {
    let cat_labels: Vec<String> = data
        .category_comparisons
        .iter()
        .map(|c| c.category.clone())
        .collect();
    move |x, _r| {
        let idx = x.value.round() as usize;
        cat_labels.get(idx).cloned().unwrap_or_default()
    }
}

fn render_summary(
    ui: &mut egui::Ui,
    data: &PeriodComparisonData,
    cat_color: &HashMap<String, eframe::egui::Color32>,
    fallback_color: eframe::egui::Color32,
) {
    let success = crate::ui::components::success_color(ui);
    let error = crate::ui::components::error_color(ui);
    let fg_default = crate::ui::components::fg_default(ui);

    ui.separator();
    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    let label_value = |ui: &mut egui::Ui, label: &str, value: String, color: eframe::egui::Color32| {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(value).color(color));
            });
        });
    };

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            crate::ui::components::section_header(ui, "Summary");
            label_value(
                ui,
                &data.period_a_label,
                format!("${:.2}", data.period_a_total),
                fg_default,
            );
            label_value(
                ui,
                &data.period_b_label,
                format!("${:.2}", data.period_b_total),
                fg_default,
            );

            let diff_text = if data.difference >= 0.0 {
                format!("+${:.2} (+{:.1}%)", data.difference, data.difference_pct)
            } else {
                format!(
                    "-${:.2} ({:.1}%)",
                    data.difference.abs(),
                    data.difference_pct.abs()
                )
            };
            let diff_color = if data.difference >= 0.0 {
                success
            } else {
                error
            };
            label_value(ui, "Difference", diff_text, diff_color);
        });

        ui.add_space(crate::ui::theme_tokens::SPACE_6);

        ui.vertical(|ui| {
            crate::ui::components::section_header(ui, "Category Breakdown");
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for cat in &data.category_comparisons {
                        ui.horizontal(|ui| {
                            // ponytail: swatch uses the category's
                            // own color (not a generic status color)
                            // so the row's color matches the bar
                            // color in the chart above. The
                            // variance text on the right still uses
                            // the success/error theme colors to
                            // communicate the direction.
                            let swatch_color = cat_color
                                .get(&cat.category)
                                .copied()
                                .unwrap_or(fallback_color);
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            // ponytail: outlined swatch (1px darker) matches
                            // the bar chart's silhouette so the user can map
                            // a swatch to its bar at a glance.
                            let swatch_outline = crate::ui::components::darker(swatch_color, 0.18);
                            ui.painter().add(egui::Shape::Rect(egui::epaint::RectShape::new(
                                rect,
                                egui::CornerRadius::same(2),
                                swatch_color,
                                egui::Stroke::new(1.0_f32, swatch_outline),
                                egui::StrokeKind::Inside,
                            )));
                            ui.add_space(crate::ui::theme_tokens::SPACE_1);
                            let display_name = if cat.category.len() > 20 {
                                format!("{}…", &cat.category[..17])
                            } else {
                                cat.category.clone()
                            };
                            ui.label(display_name);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let color = if cat.difference >= 0.0 {
                                        error
                                    } else {
                                        success
                                    };
                                    let text = if cat.difference >= 0.0 {
                                        format!("+${:.2}", cat.difference)
                                    } else {
                                        format!("-${:.2}", cat.difference.abs())
                                    };
                                    ui.label(RichText::new(text).color(color));
                                },
                            );
                        });
                        ui.add_space(crate::ui::theme_tokens::SPACE_1);
                    }
                });
        });
    });
}
