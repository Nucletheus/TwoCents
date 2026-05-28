use eframe::egui;
use crate::db::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_dashboard(&self, ui: &mut egui::Ui) {
    ui.heading("Dashboard");
    ui.separator();
    ui.monospace(self.dashboard_summary());
  }

  pub fn dashboard_summary(&self) -> String {
    let account_total: i64 = self.accounts.iter().map(|account| account.balance_cents).sum();
    let expense_total: i64 = self.expenses.iter().map(|expense| expense.amount_cents).sum();
    format!(
      "Household: {}\nAccounts: {}\nTotal balance: {}\nExpenses loaded: {}\nThis month spend: {}\n\nSQLite: {}",
      self.household_name,
      self.accounts.len(),
      money(account_total),
      self.expenses.len(),
      money(expense_total),
      db_path().display()
    )
  }
}
