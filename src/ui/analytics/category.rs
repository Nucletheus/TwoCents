use eframe::egui::{self, Color32, Pos2, RichText};

use super::aggregation::*;
use super::empty_state::render_no_data_state;
use super::state::*;
use crate::models::*;

pub fn render_category_breakdown_chart(
    ui: &mut egui::Ui,
    expenses: &[Expense],
    categories: &[Category],
    state: &mut AnalyticsState,
) -> bool {
    let mut state_changed = false;
    let excluded = excluded_category_labels(categories);

    // the legend's source must NOT be filtered by the category
    // selection — it aggregates with date/member/vendor filters only.
    // Filtering it by selection erased every other row the moment you
    // clicked one, making multi-select impossible (the rows you'd click
    // next stopped existing). The donut still renders the fully-filtered
    // totals, so clicking categories visibly reshapes the chart.
    let mut legend_state = state.clone();
    legend_state.selected_categories.clear();
    let legend_pool = filter_expenses(expenses, &legend_state, &excluded);
    let legend_totals = aggregate_by_category(&legend_pool, categories);

    if legend_totals.is_empty() {
        render_no_data_state(ui);
        return false;
    }

    // Order legend + donut identically so wedge ↔ row maps 1:1 — shared
    // Category/Cost sort (one selection across all three tabs), cost key
    // = the signed amount (helper compares |cost|, so Cost ↓ = biggest
    // spend first). Orphans (categories deleted after spend) append by
    // cost, then ride the sort.
    let mut ordered: Vec<CategoryTotal> = legend_totals.clone();
    super::picker::order_rows_by_legend_sort(
        categories,
        &mut ordered,
        state.legend_sort,
        |t| &t.category,
        |t| t.amount,
    );

    let filtered = filter_expenses(expenses, state, &excluded);
    let donut_data = aggregate_by_category(&filtered, categories);
    let donut_used: Vec<CategoryTotal> = ordered
        .iter()
        .filter_map(|t| {
            donut_data
                .iter()
                .find(|d| d.category == t.category)
                .cloned()
        })
        .collect();

    let total: f64 = donut_used.iter().map(|c| c.amount).sum();
    // expense amounts are negative (spend), so the signed total
    // is <= 0 and the old `total > 0` gate drew zero slices. Wedge shares
    // and legend percentages use the ABS sum — a spending donut is shares
    // of magnitude, not of the signed net. The footer Total stays signed.
    let legend_total: f64 = legend_totals.iter().map(|c| c.amount).sum();
    let abs_total: f64 = donut_used.iter().map(|c| c.amount.abs()).sum();
    let legend_abs: f64 = legend_totals.iter().map(|c| c.amount.abs()).sum();

    // picker fills the container height — available minus the
    // fixed chrome above (heading/hint/sort) and below (separator/Total/
    // status) the list, so no white space is left at the bottom.
    let avail = ui.available_size_before_wrap();
    let picker_h = (avail.y - 180.0).max(160.0);

    ui.horizontal(|ui| {
        // Left side: Donut chart
        ui.vertical(|ui| {
            let chart_size = 400.0;
            let (rect, donut_response) =
                ui.allocate_exact_size(egui::vec2(chart_size, chart_size), egui::Sense::hover());

            // Draw donut chart
            let center = rect.center();
            let outer_radius = chart_size * 0.4;
            let inner_radius = chart_size * 0.25;

            let mut start_angle = -std::f32::consts::FRAC_PI_2; // Start from top
            let mut slice_hit: Vec<(f32, f32, &super::aggregation::CategoryTotal)> = Vec::new();

            for cat in &donut_used {
                let slice_angle = if abs_total > 0.0 {
                    (cat.amount.abs() / abs_total) as f32 * std::f32::consts::TAU
                } else {
                    0.0
                };
                let end_angle = start_angle + slice_angle;

                // Draw slice
                draw_donut_slice(
                    ui,
                    center,
                    inner_radius,
                    outer_radius,
                    start_angle,
                    end_angle,
                    cat.color,
                );

                slice_hit.push((start_angle, end_angle, cat));
                start_angle = end_angle;
            }

            // Draw center circle (donut hole)
            ui.painter()
                .circle_filled(center, inner_radius, ui.visuals().panel_fill);

            // Draw total in center
            let total_text = if donut_used.is_empty() {
                "No data\nfor selection".to_string()
            } else {
                format!("${:.2}", total)
            };
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                total_text,
                egui::FontId::proportional(18.0),
                crate::ui::theme::fg_primary(),
            );

            // pointer hit-test — angle (atan2, normalized against
            // the same -π/2 start) + radius decide which slice is hovered.
            // The donut is painter-drawn, so egui can't tooltip it for us.
            if let Some(pos) = donut_response.hover_pos() {
                let dx = pos.x - center.x;
                let dy = pos.y - center.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist >= inner_radius && dist <= outer_radius {
                    let mut angle = dy.atan2(dx);
                    if angle < -std::f32::consts::FRAC_PI_2 {
                        angle += std::f32::consts::TAU;
                    }
                    if let Some(&(_, _, cat)) =
                        slice_hit.iter().find(|(s, e, _)| angle >= *s && angle < *e)
                    {
                        donut_response.on_hover_text(format!(
                            "{}\n${:.2} ({:.1}%)",
                            cat.category, cat.amount, cat.percentage
                        ));
                    }
                }
            }
        });

        ui.add_space(20.0);

        // Right side: Legend
        ui.vertical(|ui| {
            crate::ui::components::heading_lg(ui, "Categories");
            ui.add_space(8.0);

            ui.label(
                RichText::new("Click categories to filter (multi-select)")
                    .size(11.0)
                    .color(crate::ui::theme::fg_secondary()),
            );
            ui.add_space(4.0);

            // Shared Category/Cost sort row — same widget + same stored
            // selection as Budget vs Actual and Period Comparison.
            if super::picker::render_legend_sort(ui, state) {
                state_changed = true;
            }
            ui.add_space(4.0);

            // Legend rows = the shared category picker (side placement).
            let picker_rows: Vec<super::picker::PickerRow> = ordered
                .iter()
                .map(|cat| {
                    // abs — a negative (spend) pool still has shares.
                    let pct = if legend_abs > 0.0 {
                        cat.amount.abs() / legend_abs * 100.0
                    } else {
                        0.0
                    };
                    super::picker::PickerRow {
                        label: cat.category.clone(),
                        color: cat.color,
                        value: Some(format!("${:.2}   {:.1}%", cat.amount, pct)),
                        value_color: None,
                    }
                })
                .collect();
            if super::picker::render_category_picker(ui, state, &picker_rows, picker_h) {
                state_changed = true;
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);

            // Total
            ui.horizontal(|ui| {
                // egui hardwires strong() text to widgets.active's
                // fg_stroke (contrast-on-accent = near-black here), so strong
                // labels must carry an explicit palette color.
                ui.label(
                    RichText::new("Total:")
                        .strong()
                        .color(crate::ui::theme::fg_primary()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("${:.2}", legend_total))
                            .strong()
                            .color(crate::ui::theme::fg_primary()),
                    );
                });
            });

            // Show filter status
            if !state.selected_categories.is_empty() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let mode_text = match state.category_filter_mode {
                        FilterMode::Include => "Showing only",
                        FilterMode::Exclude => "Excluding",
                    };
                    ui.label(
                        RichText::new(format!(
                            "{} {} categories",
                            mode_text,
                            state.selected_categories.len()
                        ))
                        .size(11.0)
                        .color(crate::ui::theme::fg_secondary()),
                    );

                    if crate::ui::popups::styled_button(ui, "Clear", false).clicked() {
                        state.selected_categories.clear();
                        state_changed = true;
                    }
                });
            }
        });
    });

    state_changed
}

fn draw_donut_slice(
    ui: &mut egui::Ui,
    center: Pos2,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: Color32,
) {
    let painter = ui.painter();
    let num_segments = ((end_angle - start_angle).abs() * 20.0).max(3.0) as usize;

    let mut points = Vec::new();

    // Outer arc
    for i in 0..=num_segments {
        let angle = start_angle + (end_angle - start_angle) * (i as f32 / num_segments as f32);
        let x = center.x + outer_radius * angle.cos();
        let y = center.y + outer_radius * angle.sin();
        points.push(Pos2::new(x, y));
    }

    // Inner arc (reverse direction)
    for i in (0..=num_segments).rev() {
        let angle = start_angle + (end_angle - start_angle) * (i as f32 / num_segments as f32);
        let x = center.x + inner_radius * angle.cos();
        let y = center.y + inner_radius * angle.sin();
        points.push(Pos2::new(x, y));
    }

    // Close the shape
    points.push(points[0]);

    // 1px outline on each slice in the same color, 18%
    // darker. Defines the slice boundary so adjacent wedges read
    // as distinct slices, not as a smooth gradient.
    let slice_outline = crate::ui::components::darker(color, 0.18);
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::new(1.0_f32, slice_outline),
    ));
}
