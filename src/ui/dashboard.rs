use eframe::egui;
use crate::db::*;
use crate::models::*;
use crate::TwoCentsApp;

impl TwoCentsApp {
  pub fn ui_dashboard(&self, ui: &mut egui::Ui) {
    crate::ui::components::heading_lg(ui, "Dashboard");
    ui.add_space(8.0);
    if self.expenses.is_empty() {
      // ponytail: real empty state instead of "Household: X, Expenses: 0, ...".
      crate::ui::components::empty_state(
        ui,
        "No expenses yet",
        "Import a CSV statement or add your first expense to get started.",
      );
      return;
    }
    ui.separator();
    ui.monospace(self.dashboard_summary());
  }

  pub fn dashboard_summary(&self) -> String {
    let account_total: i64 = self.accounts.iter().map(|account| account.balance_cents).sum();
    // ponytail: spending = debit rows only, magnitude — income (positive)
    // and excluded transfer/payment categories never enter the sums.
    let excluded = excluded_category_labels(&self.categories);
    let expense_total: i64 = self.expenses.iter()
      .filter(|expense| expense.amount_cents < 0 && !excluded.contains(&expense.category))
      .map(|expense| -expense.amount_cents)
      .sum();
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
