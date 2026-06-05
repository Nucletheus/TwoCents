use eframe::egui::{self, RichText};
use crate::db::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_accounts(&self, ui: &mut egui::Ui) {
    ui.heading("Accounts");
    ui.separator();
    for account in &self.accounts {
      ui.horizontal(|ui| {
        ui.label(RichText::new(&account.name));
        ui.label(&account.kind);
        ui.monospace(money(account.balance_cents));
      });
    }
  }
}
