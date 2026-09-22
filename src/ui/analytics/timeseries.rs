use eframe::egui::{self, Color32};
use egui_plot::{Bar, BarChart, GridMark, Plot, VPlacement};
use chrono::{Datelike, NaiveDate};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;

use crate::models::*;
use super::state::*;
use super::aggregation::*;
use super::empty_state::{render_no_data_state, render_empty_state};

/// ponytail: per-period stacked bar chart. Each bar is one period on
/// the x-axis (driven by `state.granularity` and the user's date
/// range). The bar is split into segments colored by the
/// user-assigned category color, so the user can read the breakdown
/// of a single day/week/month at a glance — matching the reference
/// screenshot where each category is its own colored slice.
///
/// Implementation note: egui_plot's `BarChart::stack_on` only shifts
/// the *first* chart's bars up by the height of the others — the
/// other charts don't get rendered. To produce a true stacked bar
/// (where every category shows up as its own colored segment), we
/// build one BarChart per category, with each bar's `base_offset`
/// set to the running sum of the categories below it. egui_plot
/// then draws each chart's bars at their absolute positions
/// (base_offset..base_offset+value), so the visual stack emerges
/// without needing `stack_on` at all.
pub fn render_time_series_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    categories: &[Category],
    state: &AnalyticsState,
) {
    if expenses.is_empty() {
        render_empty_state(
            ui,
            "No expenses recorded",
            Some("Add some expenses to see analytics"),
        );
        return;
    }

    let Some(start) = state.date_start else {
        render_empty_state(
            ui,
            "No start date set",
            Some("Select a date range in the filters above"),
        );
        return;
    };
    let Some(end) = state.date_end else {
        render_empty_state(
            ui,
            "No end date set",
            Some("Select a date range in the filters above"),
        );
        return;
    };

    let filtered = filter_expenses(expenses, state, &excluded_category_labels(categories));
    let buckets = aggregate_time_series_by_category(&filtered, categories, state.granularity, start, end);

    if buckets.is_empty() {
        render_no_data_state(ui);
        return;
    }

    let has_data = buckets.iter().any(|b| b.total > 0.0);
    if !has_data {
        render_empty_state(
            ui,
            "No spending in selected period",
            Some("Try a different date range or adjust your filters"),
        );
        return;
    }

    // ponytail: grid lines are off. The previous muted grid still drew
    // 10+ horizontal lines bisecting every bar at dense ranges (year
    // of daily), which read as a busy background. Without the grid,
    // the chart is clean stacked bars on the page background. Tick
    // marks at the axes are enough to anchor values.

    // ponytail: tooltip. We replace egui_plot's default text tooltip
    // (returned by `label_formatter`) with a custom rich tooltip
    // that shows each contributing category as a swatch + label +
    // amount row. The text version couldn't render color swatches,
    // so the user couldn't tell which segment was which color.
    let buckets_for_tooltip = buckets.clone();
    let granularity = state.granularity;
    let label_formatter = move |_name: &str, _value: &egui_plot::PlotPoint| -> String {
        // Return empty string so egui_plot's default tooltip
        // doesn't render. We draw our own.
        String::new()
    };

    // ponytail: build a per-category Vec<Bar> with manually-computed
    // `base_offset` values. For each period, we walk the categories
    // sorted by amount descending and assign each bar a `base_offset`
    // equal to the running sum of the categories below it. Largest
    // slice at the bottom, smallest at the top — matches the
    // reference screenshot.
    let cat_parents = category_parent_map(categories);

    // Build a label -> color lookup for the user's category list.
    let mut cat_color: std::collections::HashMap<String, Color32> =
        std::collections::HashMap::new();
    for category in categories.iter() {
        let label = category.full_label(&cat_parents);
        cat_color.insert(label, category.color);
    }

    let mut by_category: BTreeMap<String, Vec<Bar>> = BTreeMap::new();
    let mut contributed_categories: BTreeSet<String> = BTreeSet::new();

    for (period_idx, bucket) in buckets.iter().enumerate() {
        // Sort this period's segments by amount descending so the
        // largest slice is at the bottom of the stack.
        let mut segments = bucket.segments.clone();
        segments.retain(|s| s.amount > 0.0);
        segments.sort_by(|a, b| {
            b.amount
                .partial_cmp(&a.amount)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut running_total: f64 = 0.0;
        for segment in &segments {
            let color = cat_color
                .get(&segment.category)
                .copied()
                .unwrap_or(Color32::from_rgb(128, 128, 128));
            // ponytail: scale bar width with the period count so dense
            // ranges (e.g. a year of daily buckets) don't render as a
            // thin comb of bars with huge empty gaps. n=4 -> 0.8, n=12
            // -> 0.67, n=30 -> 0.27, n>=40 -> 0.2 (readable floor).
            let n = buckets.len() as f64;
            let bar_w = (8.0 / n.max(1.0)).clamp(0.2, 0.8);
            // ponytail: outline is the bar's own color, deepened by 18% in
            // HSVA value space. A red bar gets a dark-red outline, a
            // green bar gets a dark-green outline. The 1px stroke
            // defines the bar's top + sides (bottom is hidden by the
            // segment below in a stacked chart). When the bar is the
            // topmost segment, the outline draws a clean silhouette
            // around the bar's upper edge.
            let outline = crate::ui::components::darker(color, 0.18);
            let bar = Bar::new(period_idx as f64, segment.amount)
                .base_offset(running_total)
                .width(bar_w)
                .fill(color)
                .stroke(egui::Stroke::new(1.0_f32, outline))
                .name(&segment.category);
            by_category
                .entry(segment.category.clone())
                .or_default()
                .push(bar);
            running_total += segment.amount;
            contributed_categories.insert(segment.category.clone());
        }
    }

    if by_category.is_empty() {
        render_no_data_state(ui);
        return;
    }

    // ponytail: y-axis dollar formatter.
    let y_formatter =
        |y: GridMark, _range: &RangeInclusive<f64>| -> String { format!("${}", y.value as i64) };

    // ponytail: x-axis formatter — maps the integer period index back to
    // a date label formatted for the user's selected granularity.
    //
    // egui_plot's auto-tick algorithm places fractional ticks when the
    // chart is wide (e.g. ticks at 3.7, 4.2, 4.7) and at low bucket
    // counts (e.g. n=1 -> ticks at 0.0, 0.2, 0.4, 0.6 within a
    // (-0.4, 0.6) bounds, all rounding to bucket 0). Mapping those via
    // `x.value.round() as usize` and printing the bucket's date made
    // the same label appear multiple times across the axis.
    //
    // Fix: stateful suppression. Track the last bucket index that got
    // a label, and skip every tick that rounds to the same index. With
    // this, the user sees exactly one label per unique bucket, no
    // matter how many fractional ticks egui_plot places between them.
    let buckets_for_x: Vec<NaiveDate> = buckets.iter().map(|b| b.date).collect();
    let granularity_for_x = state.granularity;
    // ponytail: last_idx is a Cell<Option<i32>> captured by the closure.
    // Each tick call checks whether the rounded index differs from the
    // last one we labeled. If it does, emit the label and update; if
    // not, return empty so the duplicate tick goes unlabeled.
    let last_idx: std::cell::Cell<Option<i32>> = std::cell::Cell::new(None);
    let x_formatter = move |x: GridMark, r: &RangeInclusive<f64>| -> String {
        // Skip ticks that fall outside the visible x range. egui_plot
        // can place ticks at integer indices that extend past the
        // current viewport (especially after zoom), which produced
        // labels for buckets the user couldn't see. The `r` arg is
        // the visible x-axis range.
        if x.value < *r.start() || x.value > *r.end() {
            return String::new();
        }
        let idx = x.value.round() as i32;
        if last_idx.get() == Some(idx) {
            return String::new();
        }
        last_idx.set(Some(idx));
        buckets_for_x
            .get(idx as usize)
            .map(|d| format_date_label(*d, granularity_for_x))
            .unwrap_or_default()
    };

    // ponytail: y-axis must start at 0 — a stacked bar chart without
    // that floor lets a single $200 segment at the top make every
    // $5 segment invisible.
    let max_stack = buckets.iter().map(|b| b.total).fold(0.0_f64, f64::max);
    let y_ceiling = if max_stack <= 0.0 { 100.0 } else { max_stack * 1.1 };

    let text_color = crate::ui::components::fg_default(ui);
    // ponytail: thick zero line color, resolved up-front (can't borrow
    // ui inside the plot closure).
    let zero_color = crate::ui::components::border_strong(ui);
    // ponytail: pin the x-axis to the actual data range so the bars fill
    // the width and date labels spread out instead of squishing. n buckets
    // span [0, n-1]; pad by 0.5 on each side so the edge bars aren't clipped.
    let n_buckets = buckets.len();
    let x_min = -0.5;
    let x_max = if n_buckets > 0 {
        (n_buckets - 1) as f64 + 0.5
    } else {
        0.5
    };
    let default_bounds =
        egui_plot::PlotBounds::from_min_max([x_min, 0.0], [x_max, y_ceiling]);
    let plot = Plot::new("time_series_stacked_bars")
        .height(360.0)
        .allow_zoom(true)
        .allow_scroll(true)
        .allow_boxed_zoom(true)
        .default_y_bounds(0.0, y_ceiling)
        .default_x_bounds(x_min, x_max)
        .show_grid([false, false])
        .x_axis_formatter(x_formatter)
        .y_axis_formatter(y_formatter)
        .label_formatter(label_formatter)
        .x_axis_position(VPlacement::Bottom);

    // ponytail: build one BarChart per category and render them in
    // turn. Each chart draws its bars at their pre-computed
    // `base_offset` heights, so the visual stack emerges as the
    // categories layer on top of each other. No `stack_on` is
    // needed because the offsets are already correct.
    //
    // BarChart is not Clone (it owns a Box<dyn Fn>), so we move each
    // chart out of the vec one at a time and pass it by ownership
    // to `bar_chart`, which takes ownership.
    let per_category_charts: Vec<BarChart> = by_category
        .into_iter()
        .map(|(cat_name, cat_bars)| {
            let color = cat_color
                .get(&cat_name)
                .copied()
                .unwrap_or(Color32::from_rgb(128, 128, 128));
            BarChart::new(&cat_name, cat_bars).color(color)
        })
        .collect();

    // ponytail: custom hover tooltip. We don't use egui_plot's
    // default text tooltip because it can't render color swatches
    // — and a swatch is the only way to tell which segment in a
    // stack is which category. Instead, we read the cursor's
    // position via `ui.ctx().input(|i| i.pointer.hover_pos())`,
    // transform it to plot coordinates via `plot_ui.transform()`,
    // find the closest bar, and render a popup at the cursor.
    //
    // This needs to happen *inside* the `plot.show` closure because
    // that's where `plot_ui` (and therefore the transform) is
    // available. We use `egui::Tooltip::always_open` so the
    // tooltip can have rich content (rows with swatches + labels +
    // amounts) rather than just text.
    let mut chart_iter = per_category_charts.into_iter();
    let fg_default_for_tooltip = text_color;
    let bg_color_for_tooltip = crate::ui::components::bg_canvas(ui);
    let border_color_for_tooltip = crate::ui::components::border_default(ui);
    plot.show(ui, |plot_ui| {
        // Shared chart chrome: thick zero line + Reset View. No negative
        // tint (stacked spend is all >= 0).
        super::charts_common::apply_zero_line_and_bounds(
            plot_ui,
            default_bounds,
            None,
            zero_color,
            state.reset_view,
        );
        if let Some(first) = chart_iter.next() {
            plot_ui.bar_chart(first);
            for chart in chart_iter {
                plot_ui.bar_chart(chart);
            }
        }

        // Custom hover tooltip: read the cursor position, transform
        // it to plot coordinates, find the closest bucket, and show
        // a popup with the per-category breakdown.
        let hover_pos = plot_ui.ctx().input(|i| i.pointer.hover_pos());
        if let Some(hover_pos) = hover_pos {
            // Check if the cursor is inside the plot's frame.
            if plot_ui.response().rect.contains(hover_pos) {
                // Transform screen position to plot coordinates. The
                // plot's transform lives inside `plot_ui`.
                let transform = plot_ui.transform();
                let plot_value = transform.value_from_position(hover_pos);
                let idx = plot_value.x.round() as usize;
                if let Some(bucket) = buckets_for_tooltip.get(idx) {
                    // Show a popup at the cursor with the bucket
                    // breakdown. `always_open` keeps the popup alive
                    // even when no widget is hovered.
                    let layer_id = plot_ui.response().layer_id;
                    let widget_id = plot_ui.response().id;
                    egui::Tooltip::always_open(
                        plot_ui.ctx().clone(),
                        layer_id,
                        widget_id,
                        egui::PopupAnchor::Pointer,
                    )
                    .gap(8.0)
                    .show(|ui| {
                        egui::Frame::popup(ui.style())
                            .fill(bg_color_for_tooltip)
                            .stroke(egui::Stroke::new(1.0_f32, border_color_for_tooltip))
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.set_max_width(280.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} — Total ${:.2}",
                                        format_date_label(bucket.date, granularity),
                                        bucket.total
                                    ))
                                    .strong()
                                    .color(fg_default_for_tooltip),
                                );
                                ui.add_space(4.0);
                                // Sort segments largest-first so the
                                // tooltip shows the same visual order
                                // as the stacked bar.
                                let mut segments = bucket.segments.clone();
                                segments.sort_by(|a, b| {
                                    b.amount
                                        .partial_cmp(&a.amount)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                                for segment in &segments {
                                    // Strip the "Parent › " prefix so each row reads
                                    // as a bare subcategory (matches the budget chart
                                    // and the user's preference). Hover still shows
                                    // the full name via on_hover_text.
                                    let label = subcategory_label(&segment.category);
                                    ui.horizontal(|ui| {
                                        let (rect, _) = ui.allocate_exact_size(
                                            egui::vec2(10.0, 10.0),
                                            egui::Sense::hover(),
                                        );
                                        let color = cat_color
                                            .get(&segment.category)
                                            .copied()
                                            .unwrap_or(Color32::from_rgb(128, 128, 128));
                                        ui.painter().rect_filled(rect, 2.0, color);
                                        ui.add_space(6.0);
                                        // Single left-aligned line. `truncate` clips
                                        // long names instead of wrapping — wrapping
                                        // was what produced the indented second line.
                                        ui.add(
                                            egui::Label::new(
                                                egui::RichText::new(label)
                                                    .color(fg_default_for_tooltip),
                                            )
                                            .truncate(),
                                        )
                                        .on_hover_text(&segment.category);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(
                                                egui::Align::Center,
                                            ),
                                            |ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "${:.2}",
                                                        segment.amount
                                                    ))
                                                    .color(fg_default_for_tooltip),
                                                );
                                            },
                                        );
                                    });
                                    ui.add_space(2.0);
                                }
                            });
                    });
                }
            }
        }
    });
}

fn format_date_label(date: NaiveDate, granularity: AnalyticsGranularity) -> String {
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

// ponytail: strip the "Parent › " prefix so dense tooltips/axes show the
// bare subcategory. The full name is still available on hover / in panels.
fn subcategory_label(full: &str) -> String {
    match full.rfind(crate::models::CATEGORY_LABEL_SEP) {
        Some(i) => full[i + crate::models::CATEGORY_LABEL_SEP.len()..].to_string(),
        None => full.to_string(),
    }
}
