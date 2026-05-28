use eframe::egui::{self, RichText, TextureHandle};
use std::{env, fs, path::PathBuf};
use rusqlite::Connection;

mod models;
mod db;
mod ui;

use crate::models::*;
use crate::db::*;
use crate::ui::widgets::*;
use crate::ui::placeholder;


struct TwoCentsApp {
  conn: Connection,
  household_id: i64,
  household_name: String,
  tab: Tab,
  accounts: Vec<Account>,
  expenses: Vec<Expense>,
  categories: Vec<Category>,
  members: Vec<HouseholdMember>,
  new_member_name: String,
  editing_self_name: String,
  import_rows: Vec<ImportRow>,
  pub show_import_review: bool,
  csv_import_rx: Option<std::sync::mpsc::Receiver<Option<std::path::PathBuf>>>,
  pub bulk_category: String,
  pub current_theme_is_dark: Option<bool>,
  pub terminal: Vec<String>,
  chart: Option<TextureHandle>,
  chart_dirty: bool,
  autocomplete_selection: usize,
  expense_sort: ExpenseSort,
  show_category_settings: bool,
  new_parent_category_name: String,
  new_subcategory_name: String,
  new_subcategory_parent_id: Option<i64>,
  category_color_popup: Option<CategoryColorPopup>,
  member_color_popup: Option<MemberColorPopup>,
  expense_grid_selection: Option<GridSelection>,
  expense_grid_drag: Option<GridSelectDrag>,
  expense_grid_edit_cell: Option<(GridColumn, usize)>,
  expense_grid_edit_original: Option<String>,
  expense_grid_typeahead: Option<char>,
  expense_grid_scroll_offset: f32,
  import_grid_selection: Option<GridSelection>,
  import_grid_drag: Option<GridSelectDrag>,
  import_grid_edit_cell: Option<(GridColumn, usize)>,
  import_grid_typeahead: Option<char>,
  import_grid_scroll_offset: f32,
  startup_window_frames: u8,
  active_expense_cell: Option<(GridColumn, usize)>,
  pending_grid_keyboard: Option<GridPendingKeyboard>,
  pending_grid_focus_target: Option<(GridColumn, usize)>,
  show_delete_expense_confirm: bool,
  delete_expense_indices: Vec<usize>,
  show_delete_import_confirm: bool,
  delete_import_indices: Vec<usize>,
  cached_category_candidates: Vec<String>,
  cached_member_candidates: Vec<String>,
  cached_vendor_candidates: Vec<String>,
  cached_description_candidates: Vec<String>,
  cached_sorted_expense_indices: Vec<usize>,
  cached_import_vendor_candidates: Vec<String>,
  cached_import_description_candidates: Vec<String>,
}

fn main() -> eframe::Result<()> {
  if env::var("TWOCENTS_RESET_WINDOW").as_deref() == Ok("1") {
    if let Ok(appdata) = env::var("APPDATA") {
      let path = PathBuf::from(appdata).join("egui").join("data").join("TwoCents");
      let _ = fs::remove_dir_all(path);
    }
  }

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_app_id("TwoCents")
      .with_inner_size([1180.0, 780.0])
      .with_min_inner_size([720.0, 560.0])
      .with_active(true)
      .with_visible(true),
    centered: true,
    ..Default::default()
  };

  eframe::run_native(
    "TwoCents",
    options,
    Box::new(|cc| Ok(Box::new(TwoCentsApp::new(cc)))),
  )
}

impl TwoCentsApp {
  fn new(_cc: &eframe::CreationContext<'_>) -> Self {
    let conn = open_database().expect("open local SQLite database");
    let (household_id, household_name) = load_active_household(&conn).expect("load default household");
    let mut app = Self {
      conn,
      household_id,
      household_name,
      tab: Tab::Expenses,
      accounts: Vec::new(),
      expenses: Vec::new(),
      categories: Vec::new(),
      members: Vec::new(),
      new_member_name: String::new(),
      editing_self_name: String::new(),
      import_rows: Vec::new(),
      csv_import_rx: None,
      show_import_review: false,
      bulk_category: String::new(),
      current_theme_is_dark: None,
      terminal: vec!["[app] eframe UI loaded".to_string()],
      chart: None,
      chart_dirty: true,
      autocomplete_selection: 0,
      expense_sort: ExpenseSort::default(),
      show_category_settings: false,
      new_parent_category_name: String::new(),
      new_subcategory_name: String::new(),
      new_subcategory_parent_id: None,
      category_color_popup: None,
      member_color_popup: None,
      expense_grid_selection: None,
      expense_grid_drag: None,
      expense_grid_edit_cell: None,
      expense_grid_edit_original: None,
      expense_grid_typeahead: None,
      expense_grid_scroll_offset: 0.0,
      import_grid_selection: None,
      import_grid_drag: None,
      import_grid_edit_cell: None,
      import_grid_typeahead: None,
      import_grid_scroll_offset: 0.0,
      startup_window_frames: 0,
      active_expense_cell: None,
      pending_grid_keyboard: None,
      pending_grid_focus_target: None,
      show_delete_expense_confirm: false,
      delete_expense_indices: Vec::new(),
      show_delete_import_confirm: false,
      delete_import_indices: Vec::new(),
      cached_category_candidates: Vec::new(),
      cached_member_candidates: Vec::new(),
      cached_vendor_candidates: Vec::new(),
      cached_description_candidates: Vec::new(),
      cached_sorted_expense_indices: Vec::new(),
      cached_import_vendor_candidates: Vec::new(),
      cached_import_description_candidates: Vec::new(),
    };
    app.reload();
    app
  }

  fn autocomplete_candidates_for_column(&self, column: GridColumn) -> Vec<String> {
    match column {
      GridColumn::Member => self.cached_member_candidates.clone(),
      GridColumn::Category => self.cached_category_candidates.clone(),
      GridColumn::Vendor => self.cached_vendor_candidates.clone(),
      GridColumn::Description => self.cached_description_candidates.clone(),
      GridColumn::Date | GridColumn::Amount => Vec::new(),
    }
  }

  fn autocomplete_suggestions(value: &str, candidates: &[String]) -> Vec<String> {
    autocomplete_suggestions_list(value, candidates)
  }

  fn expense_autocomplete_active(&self, column: GridColumn, expense_idx: usize) -> bool {
    let Some(row) = self.expenses.get(expense_idx) else {
      return false;
    };
    let candidates = self.autocomplete_candidates_for_column(column);
    let value = match column {
      GridColumn::Member => &row.member,
      GridColumn::Category => &row.category,
      GridColumn::Vendor => &row.vendor,
      GridColumn::Description => &row.description,
      GridColumn::Date | GridColumn::Amount => return false,
    };
    !Self::autocomplete_suggestions(value, &candidates).is_empty()
  }

  fn tab_fill_expense_cell(&mut self, column: GridColumn, expense_idx: usize, selected_index: usize) {
    let candidates = self.autocomplete_candidates_for_column(column);
    let Some(row) = self.expenses.get_mut(expense_idx) else {
      return;
    };
    let value = match column {
      GridColumn::Member => &mut row.member,
      GridColumn::Category => &mut row.category,
      GridColumn::Vendor => &mut row.vendor,
      GridColumn::Description => &mut row.description,
      GridColumn::Date | GridColumn::Amount => return,
    };
    let suggestions = Self::autocomplete_suggestions(value, &candidates);
    if let Some(suggestion) = suggestions.get(selected_index) {
      *value = suggestion.clone();
    }
  }

  fn commit_expense_cell(
    &mut self,
    column: GridColumn,
    expense_idx: usize,
    pending_updates: &mut Vec<(usize, &'static str)>,
    pending_category_commits: &mut Vec<usize>,
    pending_member_commits: &mut Vec<usize>,
  ) {
    self.expense_grid_edit_original = None;
    let targets = self.expense_commit_targets(column, expense_idx);
    match column {
      GridColumn::Date => {
        self.apply_expense_field_value(&targets, "date", expense_idx);
        pending_updates.extend(targets.into_iter().map(|row| (row, "date")));
      }
      GridColumn::Amount => {
        self.apply_expense_field_value(&targets, "amount", expense_idx);
        pending_updates.extend(targets.into_iter().map(|row| (row, "amount")));
      }
      GridColumn::Member => {
        let value = self
          .expenses
          .get(expense_idx)
          .map(|row| row.member.clone())
          .unwrap_or_default();
        self.apply_expense_member_value(&targets, &value);
        pending_member_commits.extend(targets);
      }
      GridColumn::Category => {
        let value = self
          .expenses
          .get(expense_idx)
          .map(|row| row.category.clone())
          .unwrap_or_default();
        self.apply_expense_category_value(&targets, &value);
        pending_category_commits.extend(targets);
      }
      GridColumn::Vendor => {
        self.apply_expense_field_value(&targets, "vendor", expense_idx);
        pending_updates.extend(targets.into_iter().map(|row| (row, "vendor")));
      }
      GridColumn::Description => {
        self.apply_expense_field_value(&targets, "description", expense_idx);
        pending_updates.extend(targets.into_iter().map(|row| (row, "description")));
      }
    }
  }

  fn apply_pending_grid_keyboard(
    &mut self,
    sorted_indices: &[usize],
    pending_updates: &mut Vec<(usize, &'static str)>,
    pending_category_commits: &mut Vec<usize>,
    pending_member_commits: &mut Vec<usize>,
  ) {
    let Some(pending) = self.pending_grid_keyboard.take() else {
      return;
    };
    if matches!(pending.action, GridKeyboardAction::Tab { .. }) {
      self.tab_fill_expense_cell(pending.column, pending.expense_idx, self.autocomplete_selection);
    }
    self.commit_expense_cell(
      pending.column,
      pending.expense_idx,
      pending_updates,
      pending_category_commits,
      pending_member_commits,
    );
    self.pending_grid_focus_target =
      grid_nav_target(sorted_indices, pending.expense_idx, pending.column, pending.action.to_nav());
  }

  fn reload(&mut self) {
    self.accounts = load_accounts(&self.conn, self.household_id).unwrap_or_else(|err| {
      self.log(format!("[error] accounts load failed: {err}"));
      Vec::new()
    });
    self.expenses = load_expenses(&self.conn, self.household_id).unwrap_or_else(|err| {
      self.log(format!("[error] expenses load failed: {err}"));
      Vec::new()
    });
    let _ = seed_default_categories(&self.conn, self.household_id);
    self.categories = load_categories(&self.conn, self.household_id).unwrap_or_else(|err| {
      self.log(format!("[error] categories load failed: {err}"));
      Vec::new()
    });
    self.members = load_household_members(&self.conn, self.household_id).unwrap_or_else(|err| {
      self.log(format!("[error] members load failed: {err}"));
      Vec::new()
    });
    self.editing_self_name = self
      .members
      .iter()
      .find(|member| member.is_self)
      .map(|member| member.name.clone())
      .unwrap_or_else(|| "Me".to_string());
    self.chart_dirty = true;
    self.rebuild_cached_candidates();
    self.rebuild_sorted_expense_indices();
  }

  fn rebuild_sorted_expense_indices(&mut self) {
    self.cached_sorted_expense_indices = sorted_expense_indices(
      &self.expenses,
      self.expense_sort,
      self.expense_grid_edit_cell,
      self.expense_grid_edit_original.as_deref(),
    );
  }

  fn rebuild_cached_candidates(&mut self) {
    self.cached_category_candidates = category_assignable_labels(&self.categories);
    self.cached_member_candidates = self.members.iter().map(|member| member.name.clone()).collect();
    self.cached_vendor_candidates = unique_nonempty_values(self.expenses.iter().map(|expense| expense.vendor.as_str()));
    self.cached_description_candidates = unique_nonempty_values(self.expenses.iter().map(|expense| expense.description.as_str()));
  }

  fn reload_categories(&mut self) {
    self.categories = load_categories(&self.conn, self.household_id).unwrap_or_default();
  }

  fn log(&mut self, line: impl Into<String>) {
    self.terminal.push(line.into());
    if self.terminal.len() > 250 {
      self.terminal.drain(0..50);
    }
  }

  fn terminal_text(&self) -> String {
    self.terminal.join("\n")
  }

  fn delete_expenses_by_indices(&mut self, indices: &[usize]) {
    let mut sorted_indices = indices.to_vec();
    sorted_indices.sort_by(|a, b| b.cmp(a)); // sort descending
    let mut logs = Vec::new();
    let mut db_deleted = false;
    
    if let Ok(tx) = self.conn.transaction() {
      let mut success = true;
      for &idx in &sorted_indices {
        if let Some(expense) = self.expenses.get(idx) {
          let res = tx.execute(
            "DELETE FROM expenses WHERE id = ?1 AND household_id = ?2",
            rusqlite::params![expense.id, self.household_id],
          );
          if let Err(err) = res {
            logs.push(format!("[error] db delete failed for idx {idx}: {err}"));
            success = false;
            break;
          }
        }
      }
      if success {
        if let Err(err) = tx.commit() {
          logs.push(format!("[error] transaction commit failed: {err}"));
        } else {
          db_deleted = true;
        }
      }
    } else {
      logs.push("[error] could not start db transaction".to_string());
    }

    for log_msg in logs {
      self.log(log_msg);
    }
    if db_deleted {
      self.log(format!("[expenses] deleted {} row(s)", sorted_indices.len()));
    }
    self.expense_grid_selection = None;
    self.reload();
  }

  fn clear_expense_grid_selection(&mut self) {
    self.expense_grid_selection = None;
    self.expense_grid_drag = None;
    self.expense_grid_edit_cell = None;
    self.expense_grid_typeahead = None;
  }

  fn apply_expense_grid_typeahead(&mut self, column: GridColumn, row: usize) {
    let Some(ch) = self.expense_grid_typeahead.take() else {
      return;
    };
    let Some(expense) = self.expenses.get_mut(row) else {
      return;
    };
    match column {
      GridColumn::Date => {
        expense.date.clear();
        expense.date.push(ch);
      }
      GridColumn::Amount => {
        expense.amount_input.clear();
        expense.amount_input.push(ch);
      }
      GridColumn::Member => {
        expense.member.clear();
        expense.member.push(ch);
      }
      GridColumn::Category => {
        expense.category.clear();
        expense.category.push(ch);
      }
      GridColumn::Vendor => {
        expense.vendor.clear();
        expense.vendor.push(ch);
      }
      GridColumn::Description => {
        expense.description.clear();
        expense.description.push(ch);
      }
    }
  }

  fn expense_commit_targets(&self, column: GridColumn, active_row: usize) -> Vec<usize> {
    grid_commit_targets(&self.expense_grid_selection, column, active_row)
  }

  fn apply_expense_member_value(&mut self, indices: &[usize], value: &str) {
    let value = value.trim().to_string();
    for &idx in indices {
      if let Some(row) = self.expenses.get_mut(idx) {
        row.member = value.clone();
      }
    }
  }

  fn apply_expense_category_value(&mut self, indices: &[usize], value: &str) {
    let value = value.trim().to_string();
    for &idx in indices {
      if let Some(row) = self.expenses.get_mut(idx) {
        row.category = value.clone();
      }
    }
  }

  fn apply_expense_field_value(&mut self, indices: &[usize], field: &str, source_idx: usize) {
    let Some(source) = self.expenses.get(source_idx).cloned() else {
      return;
    };
    for &idx in indices {
      let Some(row) = self.expenses.get_mut(idx) else {
        continue;
      };
      match field {
        "date" => row.date = source.date.clone(),
        "amount" => {
          row.amount_input = source.amount_input.clone();
          row.amount_cents = source.amount_cents;
        }
        "vendor" => row.vendor = source.vendor.clone(),
        "description" => row.description = source.description.clone(),
        _ => {}
      }
    }
  }
}

impl eframe::App for TwoCentsApp {
  fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
    visuals.panel_fill.to_normalized_gamma_f32()
  }

  fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
    if self.tab != Tab::Expenses || self.show_import_review {
      self.active_expense_cell = None;
      return;
    }
    let Some((column, expense_idx)) = self.active_expense_cell else {
      return;
    };

    let mut keyboard_action = None;
    raw_input.events.retain(|event| {
      match event {
        egui::Event::Key {
          key: egui::Key::Enter,
          pressed: true,
          modifiers,
          ..
        } if !modifiers.any() => {
          if self.expense_autocomplete_active(column, expense_idx) {
            keyboard_action = Some(GridKeyboardAction::Tab { shift: false });
            false
          } else if self.expense_grid_selection.as_ref().is_some_and(|selection| {
            selection.column == column && selection.rows.len() > 1
          }) {
            true
          } else {
            keyboard_action = Some(GridKeyboardAction::Enter);
            false
          }
        }
        egui::Event::Key {
          key: egui::Key::Tab,
          pressed: true,
          modifiers,
          ..
        } => {
          keyboard_action = Some(GridKeyboardAction::Tab {
            shift: modifiers.shift,
          });
          false
        }
        _ => true,
      }
    });

    if let Some(action) = keyboard_action {
      self.pending_grid_keyboard = Some(GridPendingKeyboard {
        column,
        expense_idx,
        action,
      });
      ctx.request_repaint();
    }
  }

  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if let Some(rx) = &self.csv_import_rx {
      if let Ok(path_opt) = rx.try_recv() {
        self.csv_import_rx = None;
        if let Some(path) = path_opt {
          self.process_csv_file(&path);
        } else {
          self.log("[import] cancelled");
        }
      }
      ctx.request_repaint();
    }

    let is_dark = ctx.global_style().visuals.dark_mode;
    if self.current_theme_is_dark != Some(is_dark) {
      self.current_theme_is_dark = Some(is_dark);
      configure_theme(ctx, is_dark);
    }

    if self.startup_window_frames < 4 {
      ensure_root_window_visible(ctx, self.startup_window_frames == 0);
      self.startup_window_frames += 1;
      ctx.request_repaint();
    }
  }

  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();

    let header_width = ui.available_width();
    egui::Frame::default()
      .fill(ui.visuals().window_fill)
      .corner_radius(12.0)
      .inner_margin(12.0)
      .show(ui, |ui| {
        ui.set_width(header_width);
        ui.horizontal(|ui| {
          ui.heading(RichText::new("TwoCents").color(ui.visuals().strong_text_color()));
          ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("Couples focused finance app").color(ui.visuals().selection.bg_fill));
          });
        });
      });

    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
      tab_button(ui, &mut self.tab, Tab::Dashboard, "Dashboard");
      tab_button(ui, &mut self.tab, Tab::Accounts, "Accounts");
      tab_button(ui, &mut self.tab, Tab::Expenses, "Expenses");
      tab_button(ui, &mut self.tab, Tab::Budgets, "Budgets");
      tab_button(ui, &mut self.tab, Tab::Goals, "Goals");
      tab_button(ui, &mut self.tab, Tab::Analytics, "Analytics");
      tab_button(ui, &mut self.tab, Tab::Settlements, "Settlements");
      tab_button(ui, &mut self.tab, Tab::Households, "Households");
    });
    ui.separator();

    const TERMINAL_LOG_HEIGHT: f32 = 120.0;
    const TERMINAL_RESERVED: f32 = TERMINAL_LOG_HEIGHT + 34.0;
    const CONTENT_TERMINAL_GAP: f32 = 4.0;
    let content_height =
      (ui.available_height() - TERMINAL_RESERVED - CONTENT_TERMINAL_GAP).max(200.0);
    ui.allocate_ui_with_layout(
      egui::vec2(ui.available_width(), content_height),
      egui::Layout::top_down(egui::Align::LEFT),
      |ui| {
        ui.set_min_height(content_height);
        ui.set_height(content_height);
        if self.tab == Tab::Expenses {
          self.ui_expenses(ui);
        } else {
          egui::ScrollArea::vertical()
            .id_salt(("tab_content", format!("{:?}", self.tab)))
            .auto_shrink([false, false])
            .show(ui, |ui| {
              match self.tab {
                Tab::Dashboard => self.ui_dashboard(ui),
                Tab::Accounts => self.ui_accounts(ui),
                Tab::Analytics => self.ui_analytics(ui, &ctx),
                Tab::Budgets => {
                  placeholder(ui, "Budgets", "Not ported yet. Next after expense/category import workflow.")
                }
                Tab::Goals => placeholder(ui, "Goals", "Not ported yet. Existing web logic remains source."),
                Tab::Settlements => {
                  placeholder(ui, "Settlements", "Not ported yet. Shared settlement math moves next.")
                }
                Tab::Households => self.ui_households(ui),
                Tab::Expenses => unreachable!("expenses tab uses dedicated layout"),
              }
            });
        }
      },
    );

    ui.add_space(CONTENT_TERMINAL_GAP);
    self.ui_terminal(ui);
    self.ui_import_modal(&ctx);
    self.ui_category_settings_window(&ctx);
    self.ui_category_color_popup(&ctx);
    self.ui_member_color_popup(&ctx);
    self.ui_delete_confirmations(&ctx);
  }
}

impl TwoCentsApp {
  fn ui_terminal(&self, ui: &mut egui::Ui) {
    ui.label(RichText::new("Terminal").strong().color(ui.visuals().selection.bg_fill));
    egui::Frame::default()
      .fill(ui.visuals().panel_fill)
      .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_fill))
      .corner_radius(6.0)
      .inner_margin(8.0)
      .show(ui, |ui| {
        let text = self.terminal_text();
        egui::ScrollArea::vertical()
          .id_salt("terminal_scroll")
          .max_height(120.0)
          .stick_to_bottom(true)

          .show(ui, |ui| {
            ui.add(
              egui::TextEdit::multiline(&mut text.as_str())
                .font(egui::TextStyle::Monospace)
                .desired_rows(6)
                .desired_width(ui.available_width())
                .text_color(ui.visuals().text_color())
                .interactive(false),
            );
          });
      });
  }
}

fn tab_button(ui: &mut egui::Ui, current: &mut Tab, tab: Tab, label: &str) {
  if ui.selectable_label(*current == tab, label).clicked() {
    *current = tab;
  }
}