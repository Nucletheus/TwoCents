use eframe::egui::{self, Color32, RichText, TextureHandle};
use egui_extras::{Column, TableBuilder};
use plotters::prelude::*;
use rusqlite::{params, Connection};
use std::{env, fs, path::PathBuf};

const CELL_BG: Color32 = Color32::from_rgb(24, 33, 47);
const CELL_TEXT: Color32 = Color32::from_rgb(226, 232, 240);
const CELL_STROKE: Color32 = Color32::from_rgb(71, 85, 105);
const MODAL_BG: Color32 = Color32::from_rgb(15, 23, 42);
const MODAL_PANEL_BG: Color32 = Color32::from_rgb(20, 30, 48);
const MODAL_ACCENT: Color32 = Color32::from_rgb(96, 165, 250);
const GRID_HEADER_HEIGHT: f32 = 20.0;
const GRID_ROW_HEIGHT: f32 = 20.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
  Dashboard,
  Accounts,
  Expenses,
  Budgets,
  Goals,
  Analytics,
  Settlements,
  Households,
}

#[derive(Clone)]
struct Account {
  name: String,
  kind: String,
  balance_cents: i64,
}

#[derive(Clone)]
struct Expense {
  id: i32,
  date: String,
  amount_input: String,
  amount_cents: i64,
  category: String,
  vendor: String,
  description: String,
}

#[derive(Clone)]
struct ImportRow {
  date: String,
  amount_input: String,
  amount_cents: i64,
  category: String,
  vendor: String,
  description: String,
}

struct TwoCentsApp {
  conn: Connection,
  tab: Tab,
  accounts: Vec<Account>,
  expenses: Vec<Expense>,
  categories: Vec<String>,
  import_rows: Vec<ImportRow>,
  show_import_review: bool,
  bulk_category: String,
  terminal: Vec<String>,
  chart: Option<TextureHandle>,
  chart_dirty: bool,
}

fn main() -> eframe::Result<()> {
  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 780.0]),
    ..Default::default()
  };

  eframe::run_native(
    "TwoCents",
    options,
    Box::new(|cc| Ok(Box::new(TwoCentsApp::new(cc)))),
  )
}

impl TwoCentsApp {
  fn new(cc: &eframe::CreationContext<'_>) -> Self {
    configure_theme(&cc.egui_ctx);
    let conn = open_database().expect("open local SQLite database");
    let mut app = Self {
      conn,
      tab: Tab::Expenses,
      accounts: Vec::new(),
      expenses: Vec::new(),
      categories: Vec::new(),
      import_rows: Vec::new(),
      show_import_review: false,
      bulk_category: String::new(),
      terminal: vec!["[app] eframe UI loaded".to_string()],
      chart: None,
      chart_dirty: true,
    };
    app.reload();
    app
  }

  fn reload(&mut self) {
    self.accounts = load_accounts(&self.conn).unwrap_or_else(|err| {
      self.log(format!("[error] accounts load failed: {err}"));
      Vec::new()
    });
    self.expenses = load_expenses(&self.conn).unwrap_or_else(|err| {
      self.log(format!("[error] expenses load failed: {err}"));
      Vec::new()
    });
    self.categories = load_categories(&self.conn).unwrap_or_else(|err| {
      self.log(format!("[error] categories load failed: {err}"));
      vec!["Uncategorized".to_string()]
    });
    self.chart_dirty = true;
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

  fn import_csv(&mut self) {
    match import_csv_statement(&self.conn) {
      Ok(rows) if rows.is_empty() => self.log("[import] cancelled or no valid rows"),
      Ok(rows) => {
        let duplicates = duplicate_import_count(&self.conn, &rows);
        let ready = rows.iter().filter(|row| import_row_status(row) == "ready").count();
        self.import_rows = rows;
        self.show_import_review = true;
        self.log(format!(
          "[import] loaded {} rows ready={} possible_duplicates={}",
          self.import_rows.len(),
          ready,
          duplicates
        ));
      }
      Err(err) => self.log(format!("[error] import failed: {err}")),
    }
  }

  fn save_import(&mut self) {
    if self.import_rows.is_empty() {
      self.log("[import] nothing to save");
      return;
    }

    self.sync_import_amounts();
    let rows = self.import_rows.clone();
    match save_import_rows(&self.conn, &rows) {
      Ok(count) => {
        self.import_rows.clear();
        self.show_import_review = false;
        self.reload();
        self.log(format!("[import] saved {count} rows"));
      }
      Err(err) => self.log(format!("[error] import save failed: {err}")),
    }
  }

  fn apply_bulk_category(&mut self) {
    let category = self.bulk_category.trim().to_string();
    if category.is_empty() {
      self.log("[bulk] no category entered");
      return;
    }
    for row in &mut self.import_rows {
      row.category = category.clone();
    }
    if ensure_category(&self.conn, &category).is_ok() {
      self.reload();
    }
    self.log(format!(
      "[bulk] applied category '{}' to {} import rows",
      category,
      self.import_rows.len()
    ));
  }

  fn sync_import_amounts(&mut self) {
    for row in &mut self.import_rows {
      if let Some(cents) = parse_amount_cents(&row.amount_input) {
        row.amount_cents = cents;
      }
    }
  }

  fn update_expense_field(&mut self, row_index: usize, field: &str) {
    let Some(row) = self.expenses.get_mut(row_index) else {
      return;
    };
    if field == "amount" {
      if let Some(cents) = parse_amount_cents(&row.amount_input) {
        row.amount_cents = cents;
      }
    }
    let id = row.id;
    match update_expense_row(&self.conn, row, field) {
      Ok(()) => {
        if field == "category" {
          let _ = ensure_category(&self.conn, &row.category);
          self.categories = load_categories(&self.conn).unwrap_or_default();
        }
        self.chart_dirty = true;
        self.log(format!("[edit] saved expense {} {}", id, field));
      }
      Err(err) => self.log(format!("[error] save failed: {err}")),
    }
  }

  fn dashboard_summary(&self) -> String {
    let account_total: i64 = self.accounts.iter().map(|account| account.balance_cents).sum();
    let expense_total: i64 = self.expenses.iter().map(|expense| expense.amount_cents).sum();
    format!(
      "Accounts: {}\nTotal balance: {}\nExpenses loaded: {}\nThis month spend: {}\n\nSQLite: {}",
      self.accounts.len(),
      money(account_total),
      self.expenses.len(),
      money(expense_total),
      db_path().display()
    )
  }
}

impl eframe::App for TwoCentsApp {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();

    egui::Frame::default()
      .fill(Color32::from_rgb(23, 32, 51))
      .corner_radius(12.0)
      .inner_margin(12.0)
      .show(ui, |ui| {
        ui.horizontal(|ui| {
          ui.heading(RichText::new("TwoCents").color(Color32::WHITE));
          ui.label(RichText::new("Rust-only local finance app").color(Color32::from_rgb(190, 208, 247)));
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

    match self.tab {
      Tab::Dashboard => self.ui_dashboard(ui),
      Tab::Accounts => self.ui_accounts(ui),
      Tab::Expenses => self.ui_expenses(ui),
      Tab::Analytics => self.ui_analytics(ui, &ctx),
      Tab::Budgets => placeholder(ui, "Budgets", "Not ported yet. Next after expense/category import workflow."),
      Tab::Goals => placeholder(ui, "Goals", "Not ported yet. Existing web logic remains source."),
      Tab::Settlements => placeholder(ui, "Settlements", "Not ported yet. Shared settlement math moves next."),
      Tab::Households => placeholder(ui, "Households", "Single local household for now. Membership/invites later."),
    }

    self.ui_import_modal(&ctx);
  }
}

impl TwoCentsApp {
  fn ui_dashboard(&self, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
      ui.heading("Dashboard");
      ui.separator();
      ui.monospace(self.dashboard_summary());
    });
  }

  fn ui_accounts(&self, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
      ui.heading("Accounts");
      ui.separator();
      for account in &self.accounts {
        ui.horizontal(|ui| {
          ui.label(RichText::new(&account.name).strong());
          ui.label(&account.kind);
          ui.monospace(money(account.balance_cents));
        });
      }
    });
  }

  fn ui_expenses(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
      ui.heading("Household Expense Spreadsheet");
      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if ui.button("Import CSV Statement").clicked() {
          self.import_csv();
        }
      });
    });
    ui.label("Click any cell and type. Changes save without stealing focus. Tab / Shift+Tab moves across cells.");

    ui.add_space(6.0);
    let mut pending_updates: Vec<(usize, &'static str)> = Vec::new();
    let category_candidates = unique_nonempty_values(self.expenses.iter().map(|expense| expense.category.as_str()));
    let table_height = (ui.available_height() - 190.0).max(220.0);

    let old_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    TableBuilder::new(ui)
      .striped(false)
      .resizable(true)
      .vscroll(true)
      .min_scrolled_height(120.0)
      .max_scroll_height(table_height)
      .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
      .column(Column::exact(110.0))
      .column(Column::exact(100.0))
      .column(Column::exact(170.0))
      .column(Column::exact(190.0))
      .column(Column::remainder())
      .header(GRID_HEADER_HEIGHT, |mut header| {
        header.col(|ui| { ui.strong("Date"); });
        header.col(|ui| { ui.strong("Amount"); });
        header.col(|ui| { ui.strong("Category"); });
        header.col(|ui| { ui.strong("Vendor"); });
        header.col(|ui| { ui.strong("Description"); });
      })
      .body(|mut body| {
        for idx in 0..self.expenses.len() {
          body.row(GRID_ROW_HEIGHT, |mut row| {
            row.col(|ui| {
              if dark_text_edit(ui, &mut self.expenses[idx].date).changed() {
                pending_updates.push((idx, "date"));
              }
            });
            row.col(|ui| {
              if dark_text_edit(ui, &mut self.expenses[idx].amount_input).changed() {
                pending_updates.push((idx, "amount"));
              }
            });
            row.col(|ui| {
              ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let response = dark_text_edit(ui, &mut self.expenses[idx].category);
                if response.changed()
                  || autocomplete_on_tab(ui, &response, &mut self.expenses[idx].category, &category_candidates)
                {
                  pending_updates.push((idx, "category"));
                }
                egui::ComboBox::from_id_salt(("expense-cat", self.expenses[idx].id))
                  .selected_text("▾")
                  .show_ui(ui, |ui| {
                    for category in self.categories.clone() {
                      if ui.selectable_label(false, &category).clicked() {
                        self.expenses[idx].category = category;
                        pending_updates.push((idx, "category"));
                        ui.close();
                      }
                    }
                  });
              });
            });
            row.col(|ui| {
              if dark_text_edit(ui, &mut self.expenses[idx].vendor).changed() {
                pending_updates.push((idx, "vendor"));
              }
            });
            row.col(|ui| {
              if dark_text_edit(ui, &mut self.expenses[idx].description).changed() {
                pending_updates.push((idx, "description"));
              }
            });
          });
        }
      });
    ui.spacing_mut().item_spacing = old_spacing;

    for (idx, field) in pending_updates {
      self.update_expense_field(idx, field);
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Terminal").strong());
    egui::Frame::default()
      .fill(Color32::from_rgb(13, 17, 23))
      .corner_radius(8.0)
      .inner_margin(8.0)
      .show(ui, |ui| {
        egui::ScrollArea::vertical().max_height(150.0).stick_to_bottom(true).show(ui, |ui| {
          ui.monospace(RichText::new(self.terminal_text()).color(Color32::from_rgb(216, 255, 224)));
        });
      });
  }

  fn ui_import_modal(&mut self, ctx: &egui::Context) {
    if !self.show_import_review {
      return;
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
      self.show_import_review = false;
      return;
    }

    let mut open = self.show_import_review;
    egui::Window::new("Review Statement Import")
      .open(&mut open)
      .resizable(true)
      .default_size([1060.0, 620.0])
      .frame(
        egui::Frame::window(&ctx.global_style())
          .fill(MODAL_BG)
          .stroke(egui::Stroke::new(2.0, MODAL_ACCENT))
          .corner_radius(10.0)
          .inner_margin(egui::Margin::symmetric(14, 12)),
      )
      .show(ctx, |ui| {
        egui::Frame::new()
          .fill(Color32::from_rgb(30, 41, 70))
          .stroke(egui::Stroke::new(1.0, Color32::from_rgb(59, 130, 246)))
          .corner_radius(6.0)
          .inner_margin(egui::Margin::symmetric(10, 6))
          .show(ui, |ui| {
            ui.horizontal(|ui| {
              ui.label(RichText::new("Import Review").strong().color(Color32::WHITE));
              ui.separator();
              ui.label(RichText::new("Edits here are staged until Save Reviewed Import.").color(CELL_TEXT));
              ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Cancel Review").clicked() {
                  self.show_import_review = false;
                }
                if ui.add(egui::Button::new("Submit Import").fill(Color32::from_rgb(37, 99, 235))).clicked() {
                  self.save_import();
                }
              });
            });
          });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
          ui.label("Bulk category:");
          dark_text_edit(ui, &mut self.bulk_category);
          if ui.button("Apply").clicked() {
            self.apply_bulk_category();
          }
          for category in self.categories.clone() {
            if ui.small_button(&category).clicked() {
              self.bulk_category = category;
              self.apply_bulk_category();
            }
          }
        });
        ui.label("Edit import rows. Free-type category or pick chip above. Save loads rows into main spreadsheet.");

        let mut pending_amount_updates = Vec::new();
        let category_candidates = unique_nonempty_values(self.import_rows.iter().map(|row| row.category.as_str()));
        let review_table_height = (ui.available_height() - 96.0).max(180.0);
        egui::Frame::new()
          .fill(MODAL_PANEL_BG)
          .stroke(egui::Stroke::new(1.0, CELL_STROKE))
          .corner_radius(6.0)
          .inner_margin(egui::Margin::same(6))
          .show(ui, |ui| {
            let old_spacing = ui.spacing().item_spacing;
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            TableBuilder::new(ui)
              .striped(false)
              .resizable(true)
              .vscroll(true)
              .min_scrolled_height(120.0)
              .max_scroll_height(review_table_height)
              .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
              .column(Column::exact(105.0))
              .column(Column::exact(105.0))
              .column(Column::exact(150.0))
              .column(Column::exact(180.0))
              .column(Column::remainder())
              .header(GRID_HEADER_HEIGHT, |mut header| {
                header.col(|ui| { ui.strong("Date"); });
                header.col(|ui| { ui.strong("Amount"); });
                header.col(|ui| { ui.strong("Category"); });
                header.col(|ui| { ui.strong("Vendor"); });
                header.col(|ui| { ui.strong("Description"); });
              })
              .body(|mut body| {
                for idx in 0..self.import_rows.len() {
                  body.row(GRID_ROW_HEIGHT, |mut row| {
                    row.col(|ui| { dark_text_edit(ui, &mut self.import_rows[idx].date); });
                    row.col(|ui| {
                      if dark_text_edit(ui, &mut self.import_rows[idx].amount_input).changed() {
                        pending_amount_updates.push(idx);
                      }
                    });
                    row.col(|ui| {
                      ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        let response = dark_text_edit(ui, &mut self.import_rows[idx].category);
                        autocomplete_on_tab(ui, &response, &mut self.import_rows[idx].category, &category_candidates);
                        egui::ComboBox::from_id_salt(("import-cat", idx))
                          .selected_text("▾")
                          .show_ui(ui, |ui| {
                            for category in self.categories.clone() {
                              if ui.selectable_label(false, &category).clicked() {
                                self.import_rows[idx].category = category;
                                ui.close();
                              }
                            }
                          });
                      });
                    });
                    row.col(|ui| { dark_text_edit(ui, &mut self.import_rows[idx].vendor); });
                    row.col(|ui| { dark_text_edit(ui, &mut self.import_rows[idx].description); });
                  });
                }
              });
            ui.spacing_mut().item_spacing = old_spacing;
          });

        for idx in pending_amount_updates {
          if let Some(row) = self.import_rows.get_mut(idx) {
            if let Some(cents) = parse_amount_cents(&row.amount_input) {
              row.amount_cents = cents;
            }
          }
        }

        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
          if ui.button("Cancel").clicked() {
            self.show_import_review = false;
          }
          if ui.add(egui::Button::new("Save Reviewed Import").fill(Color32::from_rgb(37, 99, 235))).clicked() {
            self.save_import();
          }
        });
      });
    self.show_import_review = open && self.show_import_review;
  }

  fn ui_analytics(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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

fn tab_button(ui: &mut egui::Ui, current: &mut Tab, tab: Tab, label: &str) {
  if ui.selectable_label(*current == tab, label).clicked() {
    *current = tab;
  }
}

fn dark_text_edit(ui: &mut egui::Ui, value: &mut String) -> egui::Response {
  let response = egui::Frame::new()
    .fill(CELL_BG)
    .stroke(egui::Stroke::new(1.0, CELL_STROKE))
    .corner_radius(0.0)
    .inner_margin(egui::Margin::symmetric(2, 0))
    .show(ui, |ui| {
      ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
      ui.add(
        egui::TextEdit::singleline(value)
          .desired_width(f32::INFINITY)
          .frame(
            egui::Frame::new()
              .fill(CELL_BG)
              .stroke(egui::Stroke::new(0.0, Color32::TRANSPARENT))
              .inner_margin(egui::Margin::same(0)),
          )
          .margin(egui::Margin::same(0))
          .text_color(CELL_TEXT)
          .background_color(CELL_BG),
      )
    })
    .inner;
  response
}

fn autocomplete_on_tab(
  ui: &mut egui::Ui,
  response: &egui::Response,
  value: &mut String,
  candidates: &[String],
) -> bool {
  if !response.has_focus() || value.trim().is_empty() {
    return false;
  }

  let prefix = value.trim().to_lowercase();
  let match_value = candidates
    .iter()
    .find(|candidate| {
      let candidate_lower = candidate.to_lowercase();
      candidate_lower.starts_with(&prefix) && candidate_lower != prefix
    })
    .cloned();

  if let Some(match_value) = match_value {
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
      *value = match_value;
      return true;
    }
  }

  false
}

fn unique_nonempty_values<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
  let mut unique = Vec::new();
  for value in values {
    let trimmed = value.trim();
    if !trimmed.is_empty() && !unique.iter().any(|existing: &String| existing.eq_ignore_ascii_case(trimmed)) {
      unique.push(trimmed.to_owned());
    }
  }
  unique
}

fn configure_theme(ctx: &egui::Context) {
  let mut visuals = egui::Visuals::dark();
  visuals.panel_fill = Color32::from_rgb(17, 24, 39);
  visuals.window_fill = Color32::from_rgb(24, 32, 46);
  visuals.extreme_bg_color = Color32::from_rgb(12, 17, 27);
  visuals.faint_bg_color = Color32::from_rgb(30, 41, 59);
  visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(24, 32, 46);
  visuals.widgets.inactive.bg_fill = Color32::from_rgb(31, 41, 55);
  visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(31, 41, 55);
  visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 58, 82);
  visuals.widgets.active.bg_fill = Color32::from_rgb(59, 82, 124);
  visuals.widgets.open.bg_fill = Color32::from_rgb(38, 50, 72);
  visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(226, 232, 240);
  visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(226, 232, 240);
  visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
  visuals.widgets.active.fg_stroke.color = Color32::WHITE;
  visuals.selection.bg_fill = Color32::from_rgb(57, 102, 184);
  visuals.hyperlink_color = Color32::from_rgb(125, 175, 255);
  visuals.override_text_color = Some(Color32::from_rgb(226, 232, 240));
  ctx.set_visuals(visuals);

  let mut style = (*ctx.global_style()).clone();
  style.spacing.item_spacing = egui::vec2(6.0, 4.0);
  style.spacing.button_padding = egui::vec2(8.0, 4.0);
  style.visuals.widgets.inactive.corner_radius = 3.0.into();
  style.visuals.widgets.hovered.corner_radius = 3.0.into();
  style.visuals.widgets.active.corner_radius = 3.0.into();
  ctx.set_global_style(style);
}

fn placeholder(ui: &mut egui::Ui, title: &str, body: &str) {
  egui::ScrollArea::vertical().show(ui, |ui| {
    ui.heading(title);
    ui.separator();
    ui.label(body);
  });
}

fn db_path() -> PathBuf {
  let base = env::var_os("APPDATA")
    .map(PathBuf::from)
    .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
  base.join("TwoCents").join("twocents.sqlite")
}

fn open_database() -> rusqlite::Result<Connection> {
  let path = db_path();
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?;
  }

  let conn = Connection::open(path)?;
  conn.execute_batch(
    "
    CREATE TABLE IF NOT EXISTS accounts (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL,
      kind TEXT NOT NULL,
      balance_cents INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS expenses (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      account_id INTEGER REFERENCES accounts(id),
      description TEXT NOT NULL,
      vendor TEXT,
      category TEXT NOT NULL,
      amount_cents INTEGER NOT NULL,
      date TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS categories (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL UNIQUE
    );
    CREATE TABLE IF NOT EXISTS vendor_category_rules (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      vendor_pattern TEXT NOT NULL UNIQUE,
      category TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    ",
  )?;
  let _ = conn.execute("ALTER TABLE expenses ADD COLUMN vendor TEXT", []);

  let account_count: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))?;
  if account_count == 0 {
    conn.execute("INSERT INTO accounts (name, kind, balance_cents) VALUES (?1, ?2, ?3)", params!["Household Checking", "checking", 426_550])?;
    conn.execute("INSERT INTO accounts (name, kind, balance_cents) VALUES (?1, ?2, ?3)", params!["Shared Savings", "savings", 1_240_000])?;
  }

  let expense_count: i64 = conn.query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))?;
  if expense_count == 0 {
    for (account_id, description, vendor, category, amount_cents, date) in [
      (1, "Groceries", "Groceries", "Food", 18_642, "2026-05-01"),
      (1, "Electric bill", "Electric Company", "Utilities", 14_280, "2026-05-03"),
      (1, "Date night", "Restaurant", "Dining", 9_875, "2026-05-09"),
      (1, "Gas", "Gas Station", "Transport", 6_122, "2026-05-12"),
      (1, "Internet", "Internet Provider", "Utilities", 7_999, "2026-05-15"),
    ] {
      conn.execute(
        "INSERT INTO expenses (account_id, description, vendor, category, amount_cents, date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![account_id, description, vendor, category, amount_cents, date],
      )?;
    }
  }

  for category in ["Food", "Utilities", "Dining", "Transport", "Uncategorized"] {
    ensure_category(&conn, category)?;
  }
  for (pattern, category) in [("grocery", "Food"), ("market", "Food"), ("electric", "Utilities"), ("internet", "Utilities"), ("restaurant", "Dining"), ("gas", "Transport")] {
    conn.execute("INSERT OR IGNORE INTO vendor_category_rules (vendor_pattern, category) VALUES (?1, ?2)", params![pattern, category])?;
  }
  Ok(conn)
}

fn load_accounts(conn: &Connection) -> rusqlite::Result<Vec<Account>> {
  let mut stmt = conn.prepare("SELECT name, kind, balance_cents FROM accounts ORDER BY id")?;
  let rows = stmt
    .query_map([], |row| {
      Ok(Account {
        name: row.get(0)?,
        kind: row.get(1)?,
        balance_cents: row.get(2)?,
      })
    })?
    .collect();
  rows
}

fn load_expenses(conn: &Connection) -> rusqlite::Result<Vec<Expense>> {
  let mut stmt = conn.prepare(
    "SELECT id, date, amount_cents, category, COALESCE(vendor, ''), description FROM expenses ORDER BY date DESC, id DESC",
  )?;
  let rows = stmt
    .query_map([], |row| {
      let amount_cents = row.get(2)?;
      Ok(Expense {
        id: row.get(0)?,
        date: row.get(1)?,
        amount_input: money(amount_cents),
        amount_cents,
        category: row.get(3)?,
        vendor: row.get(4)?,
        description: row.get(5)?,
      })
    })?
    .collect();
  rows
}

fn load_categories(conn: &Connection) -> rusqlite::Result<Vec<String>> {
  let mut stmt = conn.prepare("SELECT name FROM categories ORDER BY name")?;
  let rows = stmt.query_map([], |row| row.get(0))?.collect();
  rows
}

fn ensure_category(conn: &Connection, category: &str) -> rusqlite::Result<()> {
  let category = category.trim();
  if !category.is_empty() {
    conn.execute("INSERT OR IGNORE INTO categories (name) VALUES (?1)", params![category])?;
  }
  Ok(())
}

fn update_expense_row(conn: &Connection, row: &Expense, field: &str) -> rusqlite::Result<()> {
  match field {
    "date" => { conn.execute("UPDATE expenses SET date = ?1 WHERE id = ?2", params![row.date, row.id])?; }
    "amount" => { conn.execute("UPDATE expenses SET amount_cents = ?1 WHERE id = ?2", params![row.amount_cents, row.id])?; }
    "category" => {
      ensure_category(conn, &row.category)?;
      conn.execute("UPDATE expenses SET category = ?1 WHERE id = ?2", params![row.category, row.id])?;
    }
    "vendor" => { conn.execute("UPDATE expenses SET vendor = ?1 WHERE id = ?2", params![row.vendor, row.id])?; }
    "description" => { conn.execute("UPDATE expenses SET description = ?1 WHERE id = ?2", params![row.description, row.id])?; }
    _ => {}
  }
  Ok(())
}

fn save_import_rows(conn: &Connection, rows: &[ImportRow]) -> rusqlite::Result<usize> {
  for row in rows {
    ensure_category(conn, &row.category)?;
    conn.execute(
      "INSERT INTO expenses (account_id, description, vendor, category, amount_cents, date) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
      params![row.description, row.vendor, row.category, row.amount_cents, row.date],
    )?;
    if row.category != "Uncategorized" {
      conn.execute(
        "INSERT OR IGNORE INTO vendor_category_rules (vendor_pattern, category) VALUES (?1, ?2)",
        params![row.vendor.to_lowercase(), row.category],
      )?;
    }
  }
  Ok(rows.len())
}

fn money(cents: i64) -> String {
  let sign = if cents < 0 { "-" } else { "" };
  let cents = cents.abs();
  format!("{sign}${}.{:02}", cents / 100, cents % 100)
}

fn parse_amount_cents(raw: &str) -> Option<i64> {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return None;
  }
  let negative = trimmed.starts_with('-') || (trimmed.starts_with('(') && trimmed.ends_with(')'));
  let cleaned = trimmed
    .replace(['$', ',', '(', ')', ' '], "")
    .trim_start_matches('-')
    .to_string();
  let amount = cleaned.parse::<f64>().ok()?;
  let cents = (amount * 100.0).round() as i64;
  Some(if negative { -cents } else { cents }.abs())
}

fn normalize_date(raw: &str) -> String {
  let trimmed = raw.trim();
  let parts: Vec<&str> = trimmed.split(['/', '-']).collect();
  if parts.len() == 3 && parts[0].len() == 4 {
    return format!("{:0>4}-{:0>2}-{:0>2}", parts[0], parts[1], parts[2]);
  }
  if parts.len() == 3 {
    return format!("{:0>4}-{:0>2}-{:0>2}", parts[2], parts[0], parts[1]);
  }
  trimmed.to_string()
}

fn normalize_vendor(description: &str) -> String {
  description
    .split(['*', '-', '#'])
    .next()
    .unwrap_or(description)
    .trim()
    .to_string()
}

fn normalize_header(header: &str) -> String {
  header.trim().to_lowercase().replace([' ', '_', '-'], "")
}

fn find_column(headers: &[String], candidates: &[&str]) -> Option<usize> {
  headers.iter().position(|header| {
    let normalized = normalize_header(header);
    candidates.iter().any(|candidate| normalized.contains(candidate))
  })
}

fn category_for_vendor(conn: &Connection, vendor: &str) -> String {
  let normalized = vendor.to_lowercase();
  let mut stmt = match conn.prepare("SELECT vendor_pattern, category FROM vendor_category_rules ORDER BY id") {
    Ok(stmt) => stmt,
    Err(_) => return "Uncategorized".to_string(),
  };
  let rules = stmt
    .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
    .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
    .unwrap_or_default();
  rules
    .into_iter()
    .find_map(|(pattern, category)| normalized.contains(&pattern.to_lowercase()).then_some(category))
    .unwrap_or_else(|| "Uncategorized".to_string())
}

fn import_csv_statement(conn: &Connection) -> Result<Vec<ImportRow>, String> {
  let Some(path) = rfd::FileDialog::new()
    .set_title("Import statement CSV")
    .add_filter("CSV statements", &["csv"])
    .pick_file()
  else {
    return Ok(Vec::new());
  };

  let mut reader = csv::ReaderBuilder::new()
    .flexible(true)
    .from_path(&path)
    .map_err(|err| format!("Could not open CSV: {err}"))?;
  let headers = reader
    .headers()
    .map_err(|err| format!("Could not read CSV headers: {err}"))?
    .iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
  let date_idx = find_column(&headers, &["date", "posted"]);
  let amount_idx = find_column(&headers, &["amount", "debit", "withdrawal", "charge"]);
  let description_idx = find_column(&headers, &["description", "memo", "details", "transaction", "payee", "merchant"]);
  let category_idx = find_column(&headers, &["category"]);

  let mut rows = Vec::new();
  for record in reader.records() {
    let record = record.map_err(|err| format!("Bad CSV row: {err}"))?;
    let date = date_idx.and_then(|idx| record.get(idx)).map(normalize_date).unwrap_or_else(|| "2026-01-01".to_string());
    let description = description_idx.and_then(|idx| record.get(idx)).unwrap_or("").trim().to_string();
    let amount_cents = amount_idx.and_then(|idx| record.get(idx)).and_then(parse_amount_cents).unwrap_or(0);
    if description.is_empty() || amount_cents == 0 {
      continue;
    }
    let vendor = normalize_vendor(&description);
    let category = category_idx
      .and_then(|idx| record.get(idx))
      .map(str::trim)
      .filter(|value| !value.is_empty())
      .map(str::to_string)
      .unwrap_or_else(|| category_for_vendor(conn, &vendor));
    rows.push(ImportRow {
      date,
      amount_input: money(amount_cents),
      amount_cents,
      category,
      vendor,
      description,
    });
  }
  Ok(rows)
}

fn import_row_status(row: &ImportRow) -> &'static str {
  if row.date.trim().is_empty() || row.amount_cents == 0 || row.description.trim().is_empty() {
    "needs edit"
  } else if row.category.trim().is_empty() || row.category == "Uncategorized" {
    "needs category"
  } else {
    "ready"
  }
}

fn duplicate_import_count(conn: &Connection, rows: &[ImportRow]) -> usize {
  rows
    .iter()
    .filter(|row| {
      conn
        .query_row(
          "SELECT COUNT(*) FROM expenses WHERE date = ?1 AND amount_cents = ?2 AND lower(COALESCE(vendor, '')) = lower(?3) AND lower(description) = lower(?4)",
          params![row.date, row.amount_cents, row.vendor, row.description],
          |count_row| count_row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0
    })
    .count()
}

fn render_plot_texture(ctx: &egui::Context, expenses: &[Expense]) -> TextureHandle {
  const W: usize = 760;
  const H: usize = 430;
  let mut buffer = vec![255u8; W * H * 3];
  {
    let root = BitMapBackend::with_buffer(&mut buffer, (W as u32, H as u32)).into_drawing_area();
    root.fill(&RGBColor(16, 22, 34)).ok();
    let max = expenses.iter().map(|e| e.amount_cents).max().unwrap_or(25_000) as f64 / 100.0;
    let x_max = expenses.len().max(6) as f64;
    let y_max = (max * 1.2).max(100.0);
    let mut chart = ChartBuilder::on(&root)
      .margin(12)
      .caption("Local spending", ("sans-serif", 20, FontStyle::Normal, &RGBColor(230, 235, 245)).into_text_style(&root))
      .x_label_area_size(36)
      .y_label_area_size(56)
      .build_cartesian_2d(0.0..x_max, 0.0..y_max)
      .expect("chart");
    chart.configure_mesh().light_line_style(RGBColor(50, 58, 70)).draw().ok();
    let line = expenses
      .iter()
      .rev()
      .enumerate()
      .map(|(idx, expense)| (idx as f64 + 1.0, expense.amount_cents as f64 / 100.0))
      .collect::<Vec<_>>();
    chart.draw_series(LineSeries::new(line, RGBColor(90, 160, 255).stroke_width(2))).ok();
    root.present().ok();
  }
  let image = egui::ColorImage::from_rgb([W, H], &buffer);
  ctx.load_texture("spending-chart", image, Default::default())
}
