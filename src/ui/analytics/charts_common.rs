use eframe::egui::{self, Color32};
use egui_plot::{HLine, PlotBounds, PlotPoints, Polygon, PlotUi};

/// ponytail: shared chart chrome so every analytics graph behaves the
/// same way. Call this INSIDE the `Plot::show` closure, BEFORE the data
/// items, so the negative tint sits behind the bars.
///
/// * Draws a thick zero line at y=0 so the user has a fixed orientation
///   point after zooming or clearing filters.
/// * If `tint` is `Some(color)`, paints a 10%-alpha band below y=0 across
///   the full x-range — only charts whose data dip negative pass a tint
///   (e.g. Period Comparison Delta). Positive-only charts pass `None`.
/// * If `reset_view` is set, restores the chart's default (full-data)
///   bounds so "Reset View" works uniformly across all charts.
///
/// `zero_color` is the line color (caller resolves it from the theme,
/// since `PlotUi` doesn't hand us a `&Ui`/`&Context` cheaply here).
pub fn apply_zero_line_and_bounds(
    plot_ui: &mut PlotUi,
    bounds: PlotBounds,
    tint: Option<Color32>,
    zero_color: Color32,
    reset_view: bool,
) {
    let x_min = bounds.min()[0];
    let x_max = bounds.max()[0];
    let y_min = bounds.min()[1];

    // Negative tint: a convex quad from y=0 down to y_min, spanning the
    // full data x-range.
    if let Some(color) = tint {
        if y_min < 0.0 {
            let pts: Vec<[f64; 2]> =
                vec![[x_min, 0.0], [x_min, y_min], [x_max, y_min], [x_max, 0.0]];
            plot_ui.polygon(
                Polygon::new("negative_tint", PlotPoints::from(pts))
                    .fill_color(color)
                    .stroke(egui::Stroke::new(0.0_f32, Color32::TRANSPARENT)),
            );
        }
    }

    // Thick, noticeable zero line.
    plot_ui.hline(HLine::new("zero", 0.0).width(1.5_f32).color(zero_color));

    if reset_view {
        plot_ui.set_plot_bounds(bounds);
    }
}

/// ponytail: 10%-alpha version of a theme color, for the negative band.
pub fn tint_of(color: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 26)
}
