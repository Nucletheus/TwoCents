use eframe::egui;

pub mod widgets;
pub mod grid;
pub mod dashboard;
pub mod accounts;
pub mod expenses;
pub mod analytics;
pub mod households;
pub mod import_modal;
pub mod duplicates_modal;
pub mod popups;

pub mod budgets;
pub mod theme;

// Future slots:
// pub mod goals;
// pub mod settlements;

pub fn placeholder(ui: &mut egui::Ui, title: &str, body: &str) {
  ui.heading(title);
  ui.separator();
  ui.label(body);
}
