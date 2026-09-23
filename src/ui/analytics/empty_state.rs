use eframe::egui::{self, RichText};

pub fn render_empty_state(ui: &mut egui::Ui, message: &str, hint: Option<&str>) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        
        let icon_color = crate::ui::theme::fg_secondary();
        ui.label(RichText::new("📊").size(48.0).color(icon_color));
        
        ui.add_space(16.0);
        
        ui.label(RichText::new(message).size(16.0).color(crate::ui::theme::fg_primary()));
        
        if let Some(hint_text) = hint {
            ui.add_space(8.0);
            ui.label(RichText::new(hint_text).size(12.0).color(crate::ui::theme::fg_secondary()));
        }
        
        ui.add_space(40.0);
    });
}

pub fn render_no_data_state(ui: &mut egui::Ui) {
    render_empty_state(
        ui,
        "No data for selected filters",
        Some("Try adjusting your date range or removing category filters"),
    );
}

pub fn render_no_budgets_state(ui: &mut egui::Ui) {
    render_empty_state(
        ui,
        "No budget data available",
        Some("Set budgets in the Budgets tab to compare against actual spending"),
    );
}
