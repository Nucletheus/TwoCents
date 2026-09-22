use eframe::egui::{self};
use egui_plot::{BoxElem, BoxPlot, BoxSpread, Plot, PlotPoint};
use chrono::NaiveDate;

use crate::models::*;
use super::aggregation::*;
use super::state::*;
use super::empty_state::{render_no_data_state, render_empty_state};

pub fn render_candlestick_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    categories: &[Category],
    state: &AnalyticsState,
) {
    if expenses.is_empty() {
        render_empty_state(
            ui,
            "No expenses recorded",
            Some("Add some expenses to see spending velocity"),
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
    let candlesticks = aggregate_candlestick(&filtered, state.granularity, start, end);

    if candlesticks.is_empty() {
        render_no_data_state(ui);
        return;
    }

    // Capture theme colors up-front so the plot's mutable borrow of `ui`
    // doesn't conflict with the palette accessors.
    let success = crate::ui::components::success_color(ui);
    let error = crate::ui::components::error_color(ui);
    let fg_default = crate::ui::components::fg_default(ui);
    // ponytail: thick zero line color, resolved up-front (can't borrow
    // ui inside the plot closure).
    let zero_color = crate::ui::components::border_strong(ui);
    let n = candlesticks.len() as f64;

    // Tooltip: read the bucket list (cloned) at the closure's call site.
    // egui_plot passes us a `PlotPoint` whose `x` is the integer bucket
    // index and `y` is the closest bar's value. We render the period
    // label, OHLC values, and a single-line summary.
    let tooltip_buckets = candlesticks.clone();
    let label_formatter = move |name: &str, point: &PlotPoint| -> String {
        let idx = point.x.round() as usize;
        if let Some(c) = tooltip_buckets.get(idx) {
            let trend = if c.close >= c.open { "▲ spending up" } else { "▼ spending down" };
            format!(
                "{}\nOpen:  ${:.2}\nHigh:  ${:.2}\nLow:   ${:.2}\nClose: ${:.2}\n{}",
                c.label, c.open, c.high, c.low, c.close, trend
            )
        } else {
            format!("{}: {:.2}", name, point.y)
        }
    };

    // X-axis formatter: bucket index -> period label.
    let x_labels: Vec<NaiveDate> = candlesticks.iter().map(|c| c.period_start).collect();
    let x_formatter = move |x: egui_plot::GridMark, _r: &std::ops::RangeInclusive<f64>| -> String {
        let idx = x.value.round() as usize;
        x_labels
            .get(idx)
            .map(|d| d.format("%b %d").to_string())
            .unwrap_or_default()
    };

    // Build the BoxPlot. egui_plot's BoxPlot draws the OHLC layout:
    //   - whiskers from low (lower_whisker) to high (upper_whisker)
    //   - box (Q1..Q3) — we set both to min(open,close) and max(open,close)
    //   - median line — we set it equal to the open so the line falls on
    //     the open edge (no extra line drawn through the body, which
    //     would just duplicate info the box already shows).
    //
    // This is a pragmatic adaptation: the BoxPlot primitive draws
    // whiskers+box+median, which is the right shape for a candlestick
    // (low..high = wick, open..close = body).
    let mut boxes: Vec<BoxElem> = Vec::with_capacity(candlesticks.len());
    for (i, c) in candlesticks.iter().enumerate() {
        let body_low = c.open.min(c.close);
        let body_high = c.open.max(c.close);
        // ponytail: spending-velocity polarity. close >= open means
        // cumulative spend ended higher than it started — i.e. you
        // spent more in the second half of the period than the first.
        // For personal-finance tracking, that's a *bad* outcome, so
        // we color it with the theme's error color. close < open means
        // spending slowed, which is good → success color.
        let color = if c.close >= c.open { error } else { success };
        // BoxSpread(lower_whisker, Q1, median, Q3, upper_whisker).
        // Whiskers cover the low..high range (the wick). Q1..Q3 are the
        // body (open..close). Median = open so the median line lays on
        // the open edge, not in the middle of the body.
        let spread = BoxSpread::new(c.low, body_low, c.open, body_high, c.high);
        let box_elem = BoxElem::new(i as f64, spread)
            .box_width(0.6)
            .fill(color)
            .stroke(egui::Stroke::new(1.0_f32, color));
        boxes.push(box_elem);
    }

    // Summary stats: counts of bullish vs bearish periods. The "trend"
    // text below the chart reuses the same numbers.
    let total_periods = candlesticks.len();
    let bullish_count = candlesticks.iter().filter(|c| c.close >= c.open).count();
    let bearish_count = total_periods - bullish_count;
    let total_close: f64 = candlesticks.iter().map(|c| c.close).sum();
    let avg_close = if total_periods > 0 { total_close / total_periods as f64 } else { 0.0 };

    // ponytail: muted grid. The candlestick chart is dense — many
    // boxes close together at monthly/weekly granularity — so we need
    // to dial the grid back hard. grid_color = faint border, fade
    // low. show_grid is true on x so the user sees vertical
    // category-boundary lines that help them pick out individual
    // periods.
    let grid_color = crate::ui::components::border_default(ui);
    let max_high = candlesticks.iter().map(|c| c.high).fold(0.0_f64, f64::max);
    let y_ceiling = if max_high <= 0.0 { 100.0 } else { max_high * 1.1 };
    let default_bounds =
        egui_plot::PlotBounds::from_min_max([0.0, 0.0], [n, y_ceiling]);

    Plot::new("candlestick_plot")
        .height(360.0)
        .allow_zoom(true)
        .allow_scroll(true)
        .default_y_bounds(0.0, y_ceiling)
        .show_grid([false, false])
        .x_axis_formatter(x_formatter)
        .label_formatter(label_formatter)
        .show(ui, |plot_ui| {
            // Shared chart chrome: thick zero line + Reset View. No
            // negative tint (cumulative spend is all >= 0).
            super::charts_common::apply_zero_line_and_bounds(
                plot_ui,
                default_bounds,
                None,
                zero_color,
                state.reset_view,
            );
            plot_ui.box_plot(BoxPlot::new("Spending", boxes));
        });

    ui.add_space(crate::ui::theme_tokens::SPACE_3);

    // ponytail: spending-velocity summary. bullish = close >= open =
    // spending went up = bad = error color. bearish = spending went
    // down = good = success color. The chart body uses the same
    // mapping (see the box-paint loop above), so the summary numbers
    // match the bars the user is looking at.
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Spending up").size(11.0).color(fg_default));
            ui.label(egui::RichText::new(format!("{}", bullish_count)).size(20.0).strong().color(error));
        });
        ui.add_space(crate::ui::theme_tokens::SPACE_5);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Spending down").size(11.0).color(fg_default));
            ui.label(egui::RichText::new(format!("{}", bearish_count)).size(20.0).strong().color(success));
        });
        ui.add_space(crate::ui::theme_tokens::SPACE_5);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Avg close").size(11.0).color(fg_default));
            ui.label(egui::RichText::new(format!("${:.2}", avg_close)).size(20.0).strong().color(fg_default));
        });
    });

    ui.add_space(crate::ui::theme_tokens::SPACE_2);
    // Caveat: "open" here is the cumulative-spend-at-the-first-transaction
    // of the period, not a finance-style open. The chart still reads as
    // bullish-vs-bearish velocity. Keep the explainer here so users
    // don't read it as a stock chart.
    crate::ui::components::label_muted(
        ui,
        "Wick = high/low cumulative spend within the period. Body = open/close. Red means the period's closing spend was higher than its opening spend (spending up). Green means spending slowed (spending down).",
    );
}
