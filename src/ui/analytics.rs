use eframe::egui::{self, TextureHandle};
use plotters::prelude::*;
use crate::models::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_analytics(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.heading("Analytics");
    ui.label("Plotters chart rendered into egui texture.");
    if self.chart_dirty || self.chart.is_none() {
      self.chart = Some(render_plot_texture(ctx, &self.expenses));
      self.chart_dirty = false;
    }
    if let Some(chart) = &self.chart {
      ui.image(chart);
    }
  }
}

fn render_plot_texture(ctx: &egui::Context, expenses: &[Expense]) -> TextureHandle {
  const W: usize = 760;
  const H: usize = 430;
  let mut buffer = vec![255u8; W * H * 3];
  {
    let root = BitMapBackend::with_buffer(&mut buffer, (W as u32, H as u32)).into_drawing_area();
    root.fill(&RGBColor(0x0f, 0x0e, 0x47)).ok();
    let max = expenses.iter().map(|e| e.amount_cents).max().unwrap_or(25_000) as f64 / 100.0;
    let x_max = expenses.len().max(6) as f64;
    let y_max = (max * 1.2).max(100.0);
    let mut chart = ChartBuilder::on(&root)
      .margin(12)
      .caption("Local spending", ("sans-serif", 20, FontStyle::Normal, &RGBColor(0xdd, 0xdd, 0xe8)).into_text_style(&root))
      .x_label_area_size(36)
      .y_label_area_size(56)
      .build_cartesian_2d(0.0..x_max, 0.0..y_max)
      .expect("chart");
    chart.configure_mesh().light_line_style(RGBColor(0x86, 0x86, 0xac)).draw().ok();
    let line = expenses
      .iter()
      .rev()
      .enumerate()
      .map(|(idx, expense)| (idx as f64 + 1.0, expense.amount_cents as f64 / 100.0))
      .collect::<Vec<_>>();
    chart.draw_series(LineSeries::new(line, RGBColor(0x50, 0x50, 0x81).stroke_width(2))).ok();
    root.present().ok();
  }
  let image = egui::ColorImage::from_rgb([W, H], &buffer);
  ctx.load_texture("spending-chart", image, Default::default())
}
