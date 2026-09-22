use eframe::egui::{self, RichText};
use crate::db::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_accounts(&self, ui: &mut egui::Ui) {
    crate::ui::components::heading_lg(ui, "Accounts");
    ui.add_space(crate::ui::theme_tokens::SPACE_2);
    for account in &self.accounts {
      ui.horizontal(|ui| {
        ui.label(RichText::new(&account.name));
        ui.label(&account.kind);
        ui.monospace(money(account.balance_cents));
      });
    }
  }
}
