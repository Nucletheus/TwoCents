use plotters::prelude::*;

slint::include_modules!();

fn render_analytics_plot() -> slint::Image {
  const W: u32 = 640;
  const H: u32 = 420;
  let mut pixel_buffer = slint::SharedPixelBuffer::new(W, H);
  let size = (pixel_buffer.width(), pixel_buffer.height());
  let backend = BitMapBackend::with_buffer(pixel_buffer.make_mut_bytes(), size);
  let root = backend.into_drawing_area();
  root
    .fill(&RGBColor(20, 24, 32))
    .expect("plot fill");

  let cap_color = RGBColor(220, 220, 230);
  let mut chart = ChartBuilder::on(&root)
    .x_label_area_size(40)
    .y_label_area_size(50)
    .margin(8)
    .caption(
      "Sample spending (plotters) — local data TBD",
      ("sans-serif", 16, FontStyle::Normal, &cap_color).into_text_style(&root),
    )
    .build_cartesian_2d(0.0f64..10.0f64, 0.0f64..3000.0f64)
    .expect("build chart");

  chart
    .configure_mesh()
    .x_labels(6)
    .y_labels(4)
    .x_desc("Month index (demo)")
    .y_desc("Amount")
    .light_line_style(RGBColor(50, 58, 70).stroke_width(1))
    .label_style(
      ("sans-serif", 12, FontStyle::Normal, &RGBColor(150, 160, 180)).into_text_style(&root),
    )
    .draw()
    .expect("mesh");

  let line: Vec<(f64, f64)> = (0..=10)
    .map(|m| m as f64)
    .map(|m| (m, 1200.0 + (m * 0.6).sin() * 500.0))
    .collect();

  chart
    .draw_series(LineSeries::new(
      line,
      RGBColor(90, 160, 255).stroke_width(2),
    ))
    .expect("line series");
  root.present().expect("present");
  drop(chart);
  drop(root);
  slint::Image::from_rgb8(pixel_buffer)
}

fn main() -> Result<(), slint::PlatformError> {
  let app = MainWindow::new()?;
  app.on_render_analytics_plot(render_analytics_plot);
  app.run()
}
