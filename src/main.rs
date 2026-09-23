// Lint relaxations: the remaining clippy findings are cosmetic
// (needless_borrow, collapsible_if, dead_code, etc.). Allowed globally;
// remove each from this list as it gets addressed.
#![allow(
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::collapsible_if,
    clippy::empty_line_after_doc_comments,
    clippy::manual_flatten,
    clippy::manual_is_multiple_of,
    clippy::needless_lifetimes,
    clippy::needless_range_loop,
    clippy::redundant_pattern_matching,
    clippy::unnecessary_cast,
    clippy::unnecessary_map_or,
    clippy::unnecessary_sort_by,
    clippy::while_let_on_iterator,
    clippy::clone_on_copy,
    clippy::ptr_arg,
    clippy::upper_case_acronyms,
    dead_code,
    unused_variables
)]

use chrono::Datelike;
use eframe::egui::{self};
use rusqlite::{params, Connection};
use std::{collections::HashMap, env, fs, path::PathBuf};

mod db;
mod models;
mod ui;

use crate::db::*;
use crate::models::*;
use crate::ui::theme;
use crate::ui::widgets::*;

/// Cached week counts for the current budget year
#[derive(Clone)]
struct WeekCountCache {
    year: i32,
    weeks_in_month: [i32; 13],  // index 1-12
    weeks_in_quarter: [i32; 5], // index 1-4
    total_weeks: i32,
    week_to_month: [i32; 54], // week number → month (1-12)
}

impl WeekCountCache {
    fn build(year: i32) -> Self {
        let mut weeks_in_month = [0i32; 13];
        let mut weeks_in_quarter = [0i32; 5];
        let mut week_to_month = [0i32; 54];

        for month in 1..=12 {
            let first_day = chrono::NaiveDate::from_ymd_opt(year, month as u32, 1).unwrap();
            let last_day = if month == 12 {
                chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - chrono::Duration::days(1)
            } else {
                chrono::NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1).unwrap()
                    - chrono::Duration::days(1)
            };

            let mut weeks = std::collections::HashSet::new();
            let mut current = first_day;
            while current <= last_day {
                let week_num = current.iso_week().week() as i32;
                weeks.insert(week_num);
                week_to_month[week_num as usize] = month as i32;
                current += chrono::Duration::days(1);
            }

            weeks_in_month[month as usize] = weeks.len() as i32;
            let quarter = (month - 1) / 3 + 1;
            weeks_in_quarter[quarter as usize] += weeks.len() as i32;
        }

        let dec31 = chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
        let total_weeks = dec31.iso_week().week() as i32;

        WeekCountCache {
            year,
            weeks_in_month,
            weeks_in_quarter,
            total_weeks,
            week_to_month,
        }
    }
}

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
    editing_household_name: String,
    import_rows: Vec<ImportRow>,
    pub show_import_review: bool,
    import_account_name: String,
    import_detected_account: String,
    csv_import_rx: Option<std::sync::mpsc::Receiver<Option<std::path::PathBuf>>>,
    pub selected_theme: theme::ThemePreset,
    autocomplete_selection: usize,
    expense_sort: ExpenseSort,
    new_parent_category_name: String,
    adding_subcategory_to: Option<i64>,
    inline_subcategory_name: String,
    category_color_popup: Option<CategoryColorPopup>,
    member_color_popup: Option<MemberColorPopup>,
    pub expense_grid_state: GridState,
    pub import_grid_state: GridState,
    startup_window_frames: u8,
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
    import_sort: ExpenseSort,
    cached_sorted_import_indices: Vec<usize>,
    pending_expense_sort: Option<ExpenseSort>,
    pending_import_sort: Option<ExpenseSort>,
    pub show_duplicate_review: bool,
    pub duplicate_rows: Vec<Expense>,
    pub duplicate_grid_state: GridState,
    pub cached_sorted_duplicate_indices: Vec<usize>,
    pub pending_duplicate_sort: Option<ExpenseSort>,
    pub duplicate_sort: ExpenseSort,
    pub duplicate_deleted_ids: std::collections::HashSet<i32>,
    pub budgets: std::collections::HashMap<String, i64>,
    pub budget_year: i32,
    pub budget_month: i32,
    pub budget_week: u32,
    pub budget_quarter: u32,
    pub budget_filter: BudgetFilter,
    pub budget_granularity: BudgetGranularity,
    pub budget_editing_category: Option<String>,
    pub budget_editing_input: String,
    pub variant_mode: theme::VariantMode,
    /// Cumulative right-edge fractions for the 5 budget columns (4 dividers).
    /// default: [0.30, 0.45, 0.62, 0.79]
    pub budget_col_dividers: [f32; 4],
    week_cache: Option<WeekCountCache>,
    cached_budget_snapshots: Vec<BudgetSnapshot>,
    pub category_splits: Vec<CategorySplit>,
    /// Deferred grid edits: committing to SQLite on every keystroke caused a
    /// synchronous write each frame while typing. Changes accumulate here and
    /// flush 400ms after the last change, or immediately when the editing
    /// cell closes.
    deferred_field_updates: Vec<(usize, &'static str)>,
    deferred_category_commits: Vec<usize>,
    deferred_member_commits: Vec<usize>,
    expense_last_pending_len: usize,
    expense_flush_at: Option<std::time::Instant>,
    pub analytics_state: crate::ui::analytics::state::AnalyticsState,
}

static PANIC_LOGS: std::sync::OnceLock<std::sync::Mutex<Vec<String>>> = std::sync::OnceLock::new();

pub fn push_panic_log(msg: String) {
    if let Some(mutex) = PANIC_LOGS.get() {
        if let Ok(mut guard) = mutex.lock() {
            guard.push(msg);
        }
    }
}

pub fn take_panic_logs() -> Vec<String> {
    if let Some(mutex) = PANIC_LOGS.get() {
        if let Ok(mut guard) = mutex.lock() {
            return std::mem::take(&mut *guard);
        }
    }
    Vec::new()
}

struct InAppLogger;

impl log::Log for InAppLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let msg = format!("{}", record.args());
            // Filter out noisy, harmless Vulkan registry loader warning messages on Windows
            if msg.contains("windows_read_data_files_in_registry")
                || msg.contains("Registry lookup failed")
                || msg.contains("Loader Message")
                || msg.contains("objects: (type: INSTANCE")
            {
                return;
            }
            push_panic_log(format!("[{}] {}", record.level(), msg));
        }
    }

    fn flush(&self) {}
}

static LOGGER: InAppLogger = InAppLogger;

fn load_icon_svg(svg_bytes: &[u8]) -> egui::IconData {
    // resvg re-exports usvg and tiny_skia; using those re-exports keeps a
    // single version of each crate in the dependency graph.
    let opt = resvg::usvg::Options::default();
    let rtree = resvg::usvg::Tree::from_data(svg_bytes, &opt).expect("parse SVG icon");
    let icon_size = 64u32;
    let max_dim = rtree.size().width().max(rtree.size().height());
    let scale = icon_size as f32 / max_dim as f32;
    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(icon_size, icon_size).expect("create icon pixmap");
    resvg::render(
        &rtree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    egui::IconData {
        width: pixmap.width(),
        height: pixmap.height(),
        rgba: pixmap.data().to_vec(),
    }
}

fn load_icon() -> egui::IconData {
    load_icon_svg(include_bytes!("ui/balanced-icon.svg"))
}

fn main() -> eframe::Result<()> {
    PANIC_LOGS.set(std::sync::Mutex::new(Vec::new())).ok();
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Warn))
        .ok();

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        let location = info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
        push_panic_log(format!("[panic] {msg}{location}"));
        default_hook(info);
    }));

    if env::var("TWOCENTS_RESET_WINDOW").as_deref() == Ok("1") {
        if let Ok(appdata) = env::var("APPDATA") {
            // eframe 0.34 FileStorage: %APPDATA%\<app_id>\data
            let path = PathBuf::from(appdata).join("TwoCents").join("data");
            let _ = fs::remove_dir_all(path);
        }
    }

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("TwoCents")
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([720.0, 560.0])
            .with_active(true)
            .with_visible(true)
            .with_icon(icon),
        // centered is off — with the persistence feature it would
        // stomp the remembered window position on every launch. First launch
        // falls back to the OS default placement.
        ..Default::default()
    };

    eframe::run_native(
        "TwoCents",
        options,
        Box::new(|cc| {
            // Debug-only checker that flags virtualized-table rows as "changed id"
            // whenever they scroll out of the row window; it also paints 2px red
            // rects over the grid while scrolling. Virtualized grids legitimately
            // stop creating out-of-view rows between passes — not an id bug.
            // (`Style::debug` itself only exists in debug builds.)
            #[cfg(debug_assertions)]
            cc.egui_ctx
                .all_styles_mut(|s| s.debug.warn_if_rect_changes_id = false);
            // egui's proportional chain is Ubuntu-Light → emojis (no
            // Hack), and ↑↓ — used in the sort labels and variance values — only
            // exist in Hack, so they rendered as tofu. Per-glyph fallback: Ubuntu-Light
            // still wins where it has glyphs; Hack supplies the rest.
            let mut fonts = egui::FontDefinitions::default();
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(1, "Hack".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(TwoCentsApp::new(cc)))
        }),
    )
}

impl TwoCentsApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let conn = open_database().expect("open local SQLite database");
        let (household_id, household_name) =
            load_active_household(&conn).expect("load default household");
        // restore last session's theme before the first frame —
        // the old hardcoded init reset to One Dark every launch.
        let selected_theme = get_setting(&conn, "theme_preset")
            .and_then(|v| theme::ThemePreset::from_name(&v))
            .unwrap_or(theme::ThemePreset::OneDark);
        let variant_mode = get_setting(&conn, "variant_mode")
            .map(|v| theme::VariantMode::from_key(&v))
            .unwrap_or(theme::VariantMode::System);
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
            editing_household_name: String::new(),
            import_rows: Vec::new(),
            csv_import_rx: None,
            show_import_review: false,
            import_account_name: String::new(),
            import_detected_account: String::new(),
            selected_theme,
            variant_mode,
            autocomplete_selection: 0,
            expense_sort: ExpenseSort::default(),
            new_parent_category_name: String::new(),
            adding_subcategory_to: None,
            inline_subcategory_name: String::new(),
            category_color_popup: None,
            member_color_popup: None,
            expense_grid_state: GridState::default(),
            import_grid_state: GridState::default(),
            startup_window_frames: 0,
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
            import_sort: ExpenseSort::default(),
            cached_sorted_import_indices: Vec::new(),
            pending_expense_sort: None,
            pending_import_sort: None,
            show_duplicate_review: false,
            duplicate_rows: Vec::new(),
            duplicate_grid_state: GridState::default(),
            cached_sorted_duplicate_indices: Vec::new(),
            pending_duplicate_sort: None,
            duplicate_sort: ExpenseSort::default(),
            duplicate_deleted_ids: std::collections::HashSet::new(),
            budgets: std::collections::HashMap::new(),
            // default to the current period at app start so the
            // Budgets tab opens on the right month / week / quarter. The user
            // can then change the view and the new value sticks for the rest
            // of the session.
            budget_year: chrono::Local::now().year(),
            budget_month: chrono::Local::now().month() as i32,
            budget_week: chrono::Local::now().iso_week().week(),
            budget_quarter: ((chrono::Local::now().month() - 1) / 3 + 1) as u32,
            budget_filter: BudgetFilter::All,
            budget_granularity: BudgetGranularity::Monthly,
            budget_editing_category: None,
            budget_editing_input: String::new(),
            budget_col_dividers: [0.30, 0.45, 0.62, 0.79],
            week_cache: None,
            cached_budget_snapshots: Vec::new(),
            category_splits: Vec::new(),
            deferred_field_updates: Vec::new(),
            deferred_category_commits: Vec::new(),
            deferred_member_commits: Vec::new(),
            expense_last_pending_len: 0,
            expense_flush_at: None,
            analytics_state: crate::ui::analytics::state::AnalyticsState::default(),
        };

        // Load persisted analytics state
        if let Ok(Some(row)) = load_analytics_state(&app.conn) {
            app.analytics_state = crate::ui::analytics::state::AnalyticsState::from_row(&row);
        }

        app.reload();
        app
    }

    fn reload(&mut self) {
        // pending grid edits reference expense indices; flush before
        // the row vec is rebuilt or they'd hit the wrong rows.
        self.flush_deferred_expense_commits();
        self.accounts =
            load_accounts(&self.conn, self.household_id).unwrap_or_else(|err| Vec::new());
        self.expenses =
            load_expenses(&self.conn, self.household_id).unwrap_or_else(|err| Vec::new());
        let _ = seed_default_categories(&self.conn, self.household_id);
        self.categories =
            load_categories(&self.conn, self.household_id).unwrap_or_else(|err| Vec::new());
        self.members =
            load_household_members(&self.conn, self.household_id).unwrap_or_else(|err| Vec::new());
        self.editing_self_name = self
            .members
            .iter()
            .find(|member| member.is_self)
            .map(|member| member.name.clone())
            .unwrap_or_else(|| "Me".to_string());
        self.editing_household_name = self.household_name.clone();
        self.rebuild_cached_candidates();
        self.rebuild_sorted_expense_indices();
        self.rebuild_sorted_import_indices();
        self.rebuild_sorted_duplicate_indices();

        // Cache budget snapshots once per reload (was re-queried per category before)
        self.cached_budget_snapshots =
            load_budget_snapshots_for_year(&self.conn, self.household_id, self.budget_year)
                .unwrap_or_default();
        self.category_splits =
            load_category_splits(&self.conn, self.household_id).unwrap_or_default();

        let month_code = match self.budget_granularity {
            BudgetGranularity::Yearly => 0,
            BudgetGranularity::Monthly => self.budget_month,
            BudgetGranularity::Quarterly => 20 + self.budget_quarter as i32,
            BudgetGranularity::Weekly => 100 + self.budget_week as i32,
        };

        // Compute budgets from snapshots (annual cap + overrides → default per period)
        let assignable = category_assignable_labels(&self.categories);
        let mut budgets: HashMap<String, i64> = HashMap::new();
        for cat in &assignable {
            let amount = self.compute_budget_for_category(cat);
            if amount > 0 {
                budgets.insert(cat.clone(), amount);
            }
        }

        // Fallback: if no snapshot data exists, try legacy budgets table
        if budgets.is_empty() {
            if let Ok(list) =
                load_budgets(&self.conn, self.household_id, self.budget_year, month_code)
            {
                for b in list {
                    budgets.insert(b.category, b.amount_cents);
                }
            }
        }

        self.budgets = budgets;
    }

    fn rebuild_sorted_expense_indices(&mut self) {
        self.cached_sorted_expense_indices = sorted_grid_indices(&self.expenses, self.expense_sort);
    }

    fn rebuild_sorted_import_indices(&mut self) {
        self.cached_sorted_import_indices =
            sorted_grid_indices(&self.import_rows, self.import_sort);
    }

    pub fn rebuild_sorted_duplicate_indices(&mut self) {
        self.cached_sorted_duplicate_indices =
            sorted_grid_indices(&self.duplicate_rows, self.duplicate_sort);
    }

    fn rebuild_cached_candidates(&mut self) {
        self.cached_category_candidates = category_assignable_labels(&self.categories);
        self.cached_member_candidates = self
            .members
            .iter()
            .map(|member| member.name.clone())
            .collect();
        self.cached_vendor_candidates =
            unique_nonempty_values(self.expenses.iter().map(|expense| expense.vendor.as_str()));
        self.cached_description_candidates = unique_nonempty_values(
            self.expenses
                .iter()
                .map(|expense| expense.description.as_str()),
        );
    }

    fn reload_categories(&mut self) {
        self.categories = load_categories(&self.conn, self.household_id).unwrap_or_default();
    }

    // ── Budget Snapshot Computation ──────────────────────────────────────

    fn delete_expenses_by_indices(&mut self, indices: &[usize]) {
        // flush deferred grid edits BEFORE deleting — they reference
        // indices into self.expenses, which shift the moment rows are removed.
        self.flush_deferred_expense_commits();
        let mut sorted_indices = indices.to_vec();
        sorted_indices.sort_by(|a, b| b.cmp(&a)); // sort descending
        let mut logs = Vec::new();

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
                }
            }
        } else {
            logs.push("[error] could not start db transaction".to_string());
        }

        for log_msg in logs {
            eprintln!("{log_msg}");
        }
        self.expense_grid_state.clear_selection();
        self.reload();
    }
}

impl eframe::App for TwoCentsApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if self.show_import_review {
            // The import modal is open. If the expense grid still has an active edit cell
            // or focused TextEdit, it will swallow ALL keyboard input meant for the import
            // grid. Clear the expense grid's state completely and surrender focus so the
            // import modal's widgets can claim it cleanly.
            if self.expense_grid_state.edit_cell.is_some()
                || self.expense_grid_state.active_cell.is_some()
                || self.expense_grid_state.selection.is_some()
            {
                self.expense_grid_state.edit_cell = None;
                self.expense_grid_state.edit_original = None;
                self.expense_grid_state.active_cell = None;
                self.expense_grid_state.selection = None;
                // Surrender whatever widget currently holds keyboard focus so the import
                // grid can claim it on the next frame.
                if let Some(id) = ctx.memory(|m| m.focused()) {
                    ctx.memory_mut(|m| m.surrender_focus(id));
                }
            }

            // handle_raw_input early-returns unless a cell is active;
            // guard here so the 4 candidate-Vec clones don't happen every frame
            // with nothing being edited.
            if self.import_grid_state.active_cell.is_some() {
                let cached_members = self.cached_member_candidates.clone();
                let cached_categories = self.cached_category_candidates.clone();
                let cached_vendors = self.cached_import_vendor_candidates.clone();
                let cached_descriptions = self.cached_import_description_candidates.clone();
                let candidates_fn = move |col| match col {
                    GridColumn::Member => cached_members.clone(),
                    GridColumn::Category => cached_categories.clone(),
                    GridColumn::Vendor => cached_vendors.clone(),
                    GridColumn::Description => cached_descriptions.clone(),
                    _ => Vec::new(),
                };
                self.import_grid_state.handle_raw_input(
                    ctx,
                    raw_input,
                    &self.import_rows,
                    candidates_fn,
                );
            }
        } else if self.tab == Tab::Expenses && self.expense_grid_state.active_cell.is_some() {
            let cached_members = self.cached_member_candidates.clone();
            let cached_categories = self.cached_category_candidates.clone();
            let cached_vendors = self.cached_vendor_candidates.clone();
            let cached_descriptions = self.cached_description_candidates.clone();
            let candidates_fn = move |col| match col {
                GridColumn::Member => cached_members.clone(),
                GridColumn::Category => cached_categories.clone(),
                GridColumn::Vendor => cached_vendors.clone(),
                GridColumn::Description => cached_descriptions.clone(),
                _ => Vec::new(),
            };
            self.expense_grid_state
                .handle_raw_input(ctx, raw_input, &self.expenses, candidates_fn);
        } else {
            self.expense_grid_state.active_cell = None;
        }
    }

    // eframe 0.35+ removed App::update; logic runs before each ui() and
    // also while hidden on repaint — no painting allowed here.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for log_msg in take_panic_logs() {
            eprintln!("{log_msg}");
        }

        if let Some(rx) = &self.csv_import_rx {
            if let Ok(path_opt) = rx.try_recv() {
                self.csv_import_rx = None;
                if let Some(path) = path_opt {
                    self.process_csv_file(&path);
                }
            }
            // 50ms poll instead of a max-fps repaint loop while the
            // native file dialog is open.
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        let use_dark = match self.variant_mode {
            theme::VariantMode::Dark => true,
            theme::VariantMode::Light => false,
            theme::VariantMode::System => {
                // Live OS theme from the windowing system (winit feeds it from the
                // user's Windows setting and it updates mid-session); the registry
                // query in system_is_dark is the fallback when it's unknown.
                ctx.input(|i| i.raw.system_theme)
                    .map_or_else(theme::system_is_dark, |t| t == egui::Theme::Dark)
            }
        };
        theme::configure_theme(ctx, self.selected_theme, use_dark);

        if self.startup_window_frames < 4 {
            // Recovery only — never re-center; eframe persistence restores the
            // remembered size/position (incl. which monitor) at startup.
            ensure_root_window_visible(ctx);
            self.startup_window_frames += 1;
            ctx.request_repaint();
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let modal_open = self.show_import_review || self.show_duplicate_review;

        // Paint background fill manually, then allocate a padded content area
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, theme::bg_primary());
        let inner = egui::Rect::from_min_max(
            egui::pos2(full.left() + 24.0, full.top() + 8.0),
            egui::pos2(full.right() - 24.0, full.bottom() - 8.0),
        );
        // allocate_ui_at_rect was removed in egui 0.36 — scope_builder with
        // max_rect places + allocates the child rect the same way.
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            self.ui_inner(ui, &ctx, modal_open);
        });
    }
}

impl TwoCentsApp {
    fn ui_inner(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, modal_open: bool) {
        let available = ui.available_width();
        let inner_w = available;

        // --- Header box ---
        let prev_theme = self.selected_theme;
        let prev_variant = self.variant_mode;
        crate::ui::components::card(ui).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(crate::ui::components::heading_xl_text(ui, "TwoCents"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let vm = &mut self.variant_mode;
                    // ☾/☀/⚙ glyphs are missing from the app's font set and
                    // render blank (especially in dark mode) — plain words always show.
                    let btn_label = match vm {
                        theme::VariantMode::Dark => "Dark",
                        theme::VariantMode::Light => "Light",
                        theme::VariantMode::System => "System",
                    };
                    let btn_hover = match vm {
                        theme::VariantMode::Dark => "Dark mode (click to cycle)",
                        theme::VariantMode::Light => "Light mode (click to cycle)",
                        theme::VariantMode::System => "Follow system (click to cycle)",
                    };
                    if ui.button(btn_label).on_hover_text(btn_hover).clicked() {
                        *vm = match vm {
                            theme::VariantMode::Dark => theme::VariantMode::Light,
                            theme::VariantMode::Light => theme::VariantMode::System,
                            theme::VariantMode::System => theme::VariantMode::Dark,
                        };
                    }
                    theme::theme_selector_ui(&mut self.selected_theme, ui);
                    ui.add_space(crate::ui::theme_tokens::SPACE_2);
                    crate::ui::components::label_muted(ui, "Couples focused finance app");
                });
            });
        });
        // persist immediately when the pick changes — theme must
        // survive relaunch (crash-safe; no reliance on exit hooks).
        if self.selected_theme != prev_theme || self.variant_mode != prev_variant {
            let _ = set_setting(&self.conn, "theme_preset", self.selected_theme.name());
            let _ = set_setting(&self.conn, "variant_mode", self.variant_mode.as_key());
        }

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            tab_button(ui, &mut self.tab, Tab::Expenses, "Expenses");
            tab_button(ui, &mut self.tab, Tab::Budgets, "Budgets");
            tab_button(ui, &mut self.tab, Tab::Analytics, "Analytics");
            tab_button(ui, &mut self.tab, Tab::Settlements, "Settlements");
            tab_button(ui, &mut self.tab, Tab::Households, "Household");
        });
        ui.separator();

        // --- Content area — fills the full window below the chrome ---
        let content_h = ui.available_height().max(200.0);
        ui.allocate_ui_with_layout(
            egui::vec2(inner_w, content_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_height(content_h);
                ui.set_height(content_h);
                if self.tab == Tab::Expenses {
                    self.ui_expenses(ui);
                } else if self.tab == Tab::Budgets {
                    self.ui_budgets(ui);
                } else {
                    egui::ScrollArea::vertical()
                        .id_salt(("tab_content", format!("{:?}", self.tab)))
                        .auto_shrink([false, false])
                        .show(ui, |ui| match self.tab {
                            Tab::Accounts | Tab::Goals => {}
                            Tab::Analytics => self.ui_analytics(ui),
                            Tab::Budgets => unreachable!("budgets tab uses dedicated layout"),
                            Tab::Settlements => self.ui_settlements(ui),
                            Tab::Households => self.ui_households(ui),
                            Tab::Expenses => unreachable!("expenses tab uses dedicated layout"),
                        });
                }
            },
        );

        self.ui_category_color_popup(&ctx);
        self.ui_member_color_popup(&ctx);
        self.ui_delete_confirmations(&ctx);

        // When the import modal is open, paint a dim overlay AND place a full-screen
        // interactive rect at Order::Middle to absorb all pointer events so the
        // background cannot be clicked or focused. The Window at Order::Foreground
        // sits above this blocker and remains fully interactive.
        // We deliberately do NOT use add_enabled_ui — it propagates into ctx-level
        // Windows and would break the import grid's cell editing.
        if modal_open {
            let screen = ctx.content_rect();
            let bg_layer =
                egui::LayerId::new(egui::Order::Background, egui::Id::new("import_blocker"));
            // theme-aware dim overlay. Solid black/white at 50% alpha
            // instead of multiplying the panel color (which was muddy on every
            // theme). In dark mode the dim is a translucent black; in light mode
            // a translucent dark gray.
            let overlay = if theme::active_is_dark() {
                egui::Color32::from_black_alpha(160)
            } else {
                egui::Color32::from_black_alpha(120)
            };
            ctx.layer_painter(bg_layer)
                .rect_filled(screen, 0.0, overlay);
            // Interaction blocker: a transparent clickable Area that eats all pointer events
            // over the background so nothing underneath can be focused or clicked.
            egui::Area::new(egui::Id::new("import_blocker_area"))
                .order(egui::Order::Background)
                .fixed_pos(screen.min)
                .show(&ctx, |ui| {
                    let _ = ui.allocate_rect(screen, egui::Sense::click_and_drag());
                });
        }

        // The import review Window renders at Order::Foreground — above everything.
        self.ui_import_review(&ctx);
        self.ui_duplicate_review(&ctx);
    }

    // ── Budget Snapshot Computation ──────────────────────────────────────

    fn total_periods(&self, granularity: BudgetGranularity) -> i32 {
        match granularity {
            BudgetGranularity::Yearly => 1,
            BudgetGranularity::Quarterly => 4,
            BudgetGranularity::Monthly => 12,
            BudgetGranularity::Weekly => 52,
        }
    }

    fn current_period(&self, granularity: BudgetGranularity) -> i32 {
        let now = chrono::Local::now();
        match granularity {
            BudgetGranularity::Yearly => 1,
            BudgetGranularity::Quarterly => ((now.month() - 1) / 3 + 1) as i32,
            BudgetGranularity::Monthly => now.month() as i32,
            BudgetGranularity::Weekly => now.iso_week().week() as i32,
        }
    }

    fn period_end(&self, granularity: BudgetGranularity) -> i32 {
        match granularity {
            BudgetGranularity::Yearly => 1,
            BudgetGranularity::Quarterly => 4,
            BudgetGranularity::Monthly => 12,
            BudgetGranularity::Weekly => chrono::NaiveDate::from_ymd_opt(self.budget_year, 12, 31)
                .map(|d| d.iso_week().week() as i32)
                .unwrap_or(52),
        }
    }

    fn period_code(&self, granularity: BudgetGranularity, period: i32) -> i32 {
        match granularity {
            BudgetGranularity::Yearly => 0,
            BudgetGranularity::Quarterly => 20 + period,
            BudgetGranularity::Monthly => period,
            BudgetGranularity::Weekly => 100 + period,
        }
    }

    fn is_period_past(&self, granularity: BudgetGranularity, period: i32) -> bool {
        let now = chrono::Local::now();
        let current_year = now.year();
        if self.budget_year < current_year {
            return true;
        }
        if self.budget_year > current_year {
            return false;
        }
        period < self.current_period(granularity)
    }

    fn annual_cap(&self, category: &str, year: i32, snaps: &[BudgetSnapshot]) -> i64 {
        // Yearly-cap override lives in the viewed year's snapshots; legacy
        // budgets-table rows (year, code 0) are the fallback.
        if let Some(s) = snaps
            .iter()
            .find(|s| s.category == category && s.period_code == 0 && s.is_override)
        {
            return s.amount_cents;
        }
        load_budgets(&self.conn, self.household_id, year, 0)
            .ok()
            .and_then(|list| list.into_iter().find(|b| b.category == category))
            .map(|b| b.amount_cents)
            .unwrap_or(0)
    }

    /// pure pricing core, extracted so the year-honest rule is
    /// testable without a TwoCentsApp. Pricing a year with no rows (empty
    /// snaps, cap 0) yields $0 — budgets never bleed across years.
    fn price_budget(
        category: &str,
        code: i32,
        snaps: &[BudgetSnapshot],
        cap: i64,
        periods: i64,
    ) -> i64 {
        if let Some(s) = snaps
            .iter()
            .find(|s| s.category == category && s.period_code == code)
        {
            return s.amount_cents;
        }
        if cap == 0 {
            return 0;
        }
        cap / periods.max(1)
    }

    fn compute_budget_for_category(&self, category: &str) -> i64 {
        self.compute_budget_for(
            category,
            self.budget_granularity,
            self.budget_year,
            None,
            &self.cached_budget_snapshots,
        )
    }

    /// period-parameterized budget lookup so the analytics Budget
    /// vs Actual panel can price ANY week/month/quarter/year, not just the
    /// Budgets tab's currently-selected one. `period` overrides the
    /// self.budget_* fields (analytics passes its own view period). `snaps`
    /// must be the viewed YEAR's snapshots — the year-honest fix: pricing
    /// used to read the Budgets-tab year's cache regardless of `year`,
    /// which showed current-year budgets for e.g. 2008.
    fn compute_budget_for(
        &self,
        category: &str,
        gran: BudgetGranularity,
        year: i32,
        period: Option<i32>,
        snaps: &[BudgetSnapshot],
    ) -> i64 {
        let code = match gran {
            BudgetGranularity::Yearly => 0,
            BudgetGranularity::Quarterly => 20 + period.unwrap_or(self.budget_quarter as i32),
            BudgetGranularity::Monthly => period.unwrap_or(self.budget_month),
            BudgetGranularity::Weekly => 100 + period.unwrap_or(self.budget_week as i32),
        };

        // Check for override first
        if let Some(amount) = snaps
            .iter()
            .find(|s| s.category == category && s.period_code == code)
            .map(|s| s.amount_cents)
        {
            return amount;
        }

        // Past periods with no snapshot → no budget was set → $0
        // (only applies when the viewed period is the CURRENT one; analytics
        // views of past periods treat missing snapshots as "use the cap")
        if period.is_none() && gran != BudgetGranularity::Yearly {
            let cur_period = match gran {
                BudgetGranularity::Quarterly => self.budget_quarter as i32,
                BudgetGranularity::Monthly => self.budget_month,
                BudgetGranularity::Weekly => self.budget_week as i32,
                _ => unreachable!(),
            };
            if self.is_period_past(gran, cur_period) {
                return 0;
            }
        }

        let cap = self.annual_cap(category, year, snaps);
        Self::price_budget(category, code, snaps, cap, self.total_periods(gran) as i64)
    }

    /// Date window of a budget period lives in `models::budget_period_date_range`
    /// now — shared with Period Comparison's granularity period lists.

    fn get_snapshot_amount(&self, category: &str, period_code: i32) -> Option<i64> {
        self.cached_budget_snapshots
            .iter()
            .find(|s| s.category == category && s.period_code == period_code)
            .map(|s| s.amount_cents)
    }

    fn sum_past_periods(&self, category: &str, gran: BudgetGranularity) -> i64 {
        let cur = self.current_period(gran);
        let mut sum = 0i64;
        for p in 1..cur {
            let code = self.period_code(gran, p);
            if let Some(amount) = self.get_snapshot_amount(category, code) {
                sum += amount;
            }
        }
        sum
    }

    fn propagate_to_other_granularities(
        &mut self,
        category: &str,
        source_gran: BudgetGranularity,
        amount: i64,
    ) {
        match source_gran {
            BudgetGranularity::Monthly => self.propagate_monthly_to_others(category, amount),
            BudgetGranularity::Quarterly => self.propagate_quarterly_to_others(category, amount),
            BudgetGranularity::Weekly => self.propagate_weekly_to_others(category, amount),
            BudgetGranularity::Yearly => {} // Handled separately in save_budget_with_forward_propagation
        }
    }

    fn propagate_monthly_to_others(&mut self, category: &str, monthly_amount: i64) {
        let current_month = self.current_period(BudgetGranularity::Monthly);
        let current_quarter = self.current_period(BudgetGranularity::Quarterly);
        let current_week = self.current_period(BudgetGranularity::Weekly);
        let cache = self.get_week_cache();

        // Save future months
        for m in (current_month + 1)..=12 {
            let code = self.period_code(BudgetGranularity::Monthly, m);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                monthly_amount,
                false,
            );
        }

        // Calculate and save quarterly amounts (sum of monthly amounts)
        for q in current_quarter..=4 {
            let start_month = (q - 1) * 3 + 1;
            let mut quarter_amount = 0i64;

            for m in start_month..start_month + 3 {
                if m < current_month {
                    quarter_amount += self.get_monthly_amount(category, m);
                } else {
                    quarter_amount += monthly_amount;
                }
            }

            let code = self.period_code(BudgetGranularity::Quarterly, q);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                quarter_amount,
                false,
            );
        }

        // Calculate and save yearly amount (sum of all months)
        let mut yearly_amount = 0i64;
        for m in 1..=12 {
            if m < current_month {
                yearly_amount += self.get_monthly_amount(category, m);
            } else {
                yearly_amount += monthly_amount;
            }
        }
        self.apply_budget_override(category, 0, yearly_amount);

        // Calculate and save weekly amounts
        let end_week = cache.total_weeks;
        for w in current_week..=end_week {
            let week_month = cache.week_to_month[w as usize];
            let weeks_in_this_month = cache.weeks_in_month[week_month as usize];

            let per_week = if weeks_in_this_month > 0 {
                (monthly_amount as f64 / weeks_in_this_month as f64).round() as i64
            } else {
                0
            };

            // Check if this is the last week of the month
            let is_last_week_of_month = if w < end_week {
                cache.week_to_month[(w + 1) as usize] != week_month
            } else {
                true
            };

            // Adjust last week to make total exact
            let adjusted_per_week = if is_last_week_of_month {
                let weeks_so_far_in_month = (current_week..=w)
                    .filter(|&wk| cache.week_to_month[wk as usize] == week_month)
                    .count() as i64
                    - 1;
                monthly_amount - (per_week * weeks_so_far_in_month)
            } else {
                per_week
            };

            let code = self.period_code(BudgetGranularity::Weekly, w);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                adjusted_per_week,
                false,
            );
        }
    }

    fn propagate_quarterly_to_others(&mut self, category: &str, quarterly_amount: i64) {
        let current_quarter = self.current_period(BudgetGranularity::Quarterly);
        let current_week = self.current_period(BudgetGranularity::Weekly);
        let cache = self.get_week_cache();

        // Save future quarters
        for q in (current_quarter + 1)..=4 {
            let code = self.period_code(BudgetGranularity::Quarterly, q);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                quarterly_amount,
                false,
            );
        }

        // Calculate and save yearly amount (sum of all quarters)
        let mut yearly_amount = 0i64;
        for q in 1..=4 {
            if q < current_quarter {
                yearly_amount += self.get_quarterly_amount(category, q);
            } else {
                yearly_amount += quarterly_amount;
            }
        }
        self.apply_budget_override(category, 0, yearly_amount);

        // Calculate and save weekly amounts
        let end_week = cache.total_weeks;
        for w in current_week..=end_week {
            let week_month = cache.week_to_month[w as usize];
            let week_quarter = (week_month - 1) / 3 + 1;
            let weeks_in_this_quarter = cache.weeks_in_quarter[week_quarter as usize];

            let per_week = if weeks_in_this_quarter > 0 {
                (quarterly_amount as f64 / weeks_in_this_quarter as f64).round() as i64
            } else {
                0
            };

            // Check if this is the last week of the quarter
            let is_last_week_of_quarter = if w < end_week {
                let next_week_month = cache.week_to_month[(w + 1) as usize];
                let next_week_quarter = (next_week_month - 1) / 3 + 1;
                next_week_quarter != week_quarter
            } else {
                true
            };

            // Adjust last week to make total exact
            let adjusted_per_week = if is_last_week_of_quarter {
                let quarter_start_week = (1..=w)
                    .rev()
                    .find(|&wk| {
                        let wk_month = cache.week_to_month[wk as usize];
                        let wk_quarter = (wk_month - 1) / 3 + 1;
                        wk_quarter != week_quarter
                    })
                    .map(|wk| wk + 1)
                    .unwrap_or(current_week);

                let weeks_so_far_in_quarter = (quarter_start_week..=w).count() as i64 - 1;
                quarterly_amount - (per_week * weeks_so_far_in_quarter)
            } else {
                per_week
            };

            let code = self.period_code(BudgetGranularity::Weekly, w);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                adjusted_per_week,
                false,
            );
        }

        // Calculate and save monthly amounts (derived from weekly)
        let current_month = self.current_period(BudgetGranularity::Monthly);
        for m in current_month..=12 {
            let mut month_amount = 0i64;
            for w in 1..=end_week {
                if cache.week_to_month[w as usize] == m {
                    month_amount += self.get_weekly_amount(category, w);
                }
            }

            let code = self.period_code(BudgetGranularity::Monthly, m);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                month_amount,
                false,
            );
        }
    }

    fn propagate_weekly_to_others(&mut self, category: &str, weekly_amount: i64) {
        let current_week = self.current_period(BudgetGranularity::Weekly);
        let cache = self.get_week_cache();
        let end_week = cache.total_weeks;

        // Save future weeks
        for w in (current_week + 1)..=end_week {
            let code = self.period_code(BudgetGranularity::Weekly, w);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                weekly_amount,
                false,
            );
        }

        // Calculate and save monthly amounts (sum of weekly amounts)
        let current_month = self.current_period(BudgetGranularity::Monthly);
        for m in current_month..=12 {
            let mut month_amount = 0i64;
            for w in 1..=end_week {
                if cache.week_to_month[w as usize] == m {
                    if w < current_week {
                        month_amount += self.get_weekly_amount(category, w);
                    } else {
                        month_amount += weekly_amount;
                    }
                }
            }

            let code = self.period_code(BudgetGranularity::Monthly, m);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                month_amount,
                false,
            );
        }

        // Calculate and save quarterly amounts (sum of weekly amounts)
        let current_quarter = self.current_period(BudgetGranularity::Quarterly);
        for q in current_quarter..=4 {
            let start_month = (q - 1) * 3 + 1;
            let mut quarter_amount = 0i64;

            for w in 1..=end_week {
                let week_month = cache.week_to_month[w as usize];
                if week_month >= start_month && week_month < start_month + 3 {
                    if w < current_week {
                        quarter_amount += self.get_weekly_amount(category, w);
                    } else {
                        quarter_amount += weekly_amount;
                    }
                }
            }

            let code = self.period_code(BudgetGranularity::Quarterly, q);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                quarter_amount,
                false,
            );
        }

        // Calculate and save yearly amount (sum of all weekly amounts)
        let mut yearly_amount = 0i64;
        for w in 1..=end_week {
            if w < current_week {
                yearly_amount += self.get_weekly_amount(category, w);
            } else {
                yearly_amount += weekly_amount;
            }
        }
        self.apply_budget_override(category, 0, yearly_amount);
    }

    fn propagate_yearly_to_others(&mut self, category: &str, yearly_amount: i64) {
        let current_week = self.current_period(BudgetGranularity::Weekly);
        let cache = self.get_week_cache();

        // Calculate past sum (from monthly budgets before current month)
        let current_month = self.current_period(BudgetGranularity::Monthly);
        let past_sum = self.sum_past_periods(category, BudgetGranularity::Monthly);
        let remaining_budget = (yearly_amount - past_sum).max(0);

        // Count total remaining weeks
        let total_remaining_weeks = cache.total_weeks - current_week + 1;

        // Calculate per-week amount (rounded)
        let per_week = if remaining_budget > 0 && total_remaining_weeks > 0 {
            (remaining_budget as f64 / total_remaining_weeks as f64).round() as i64
        } else {
            0
        };

        // Calculate total that will be distributed
        let total_distributed = per_week * total_remaining_weeks as i64;
        let rounding_error = remaining_budget - total_distributed;

        // Save future weeks
        let end_week = cache.total_weeks;
        for w in current_week..=end_week {
            let mut week_amount = per_week;

            // Add rounding error to last week of year
            if w == end_week {
                week_amount += rounding_error;
            }

            let code = self.period_code(BudgetGranularity::Weekly, w);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                week_amount,
                false,
            );
        }

        // Calculate and save monthly amounts (sum of weekly amounts)
        for m in current_month..=12 {
            let mut month_amount = 0i64;
            for w in current_week..=end_week {
                if cache.week_to_month[w as usize] == m {
                    month_amount += self.get_weekly_amount(category, w);
                }
            }

            let code = self.period_code(BudgetGranularity::Monthly, m);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                month_amount,
                false,
            );
        }

        // Calculate and save quarterly amounts (sum of weekly amounts)
        let current_quarter = self.current_period(BudgetGranularity::Quarterly);
        for q in current_quarter..=4 {
            let start_month = (q - 1) * 3 + 1;
            let mut quarter_amount = 0i64;

            for w in current_week..=end_week {
                let week_month = cache.week_to_month[w as usize];
                if week_month >= start_month && week_month < start_month + 3 {
                    quarter_amount += self.get_weekly_amount(category, w);
                }
            }

            let code = self.period_code(BudgetGranularity::Quarterly, q);
            let _ = save_budget_snapshot(
                &self.conn,
                self.household_id,
                category,
                self.budget_year,
                code,
                quarter_amount,
                false,
            );
        }
    }

    fn get_week_cache(&mut self) -> WeekCountCache {
        if self.week_cache.is_none() || self.week_cache.as_ref().unwrap().year != self.budget_year {
            self.week_cache = Some(WeekCountCache::build(self.budget_year));
        }
        self.week_cache.as_ref().unwrap().clone()
    }

    fn get_monthly_amount(&self, category: &str, month: i32) -> i64 {
        let code = self.period_code(BudgetGranularity::Monthly, month);
        self.get_snapshot_amount(category, code).unwrap_or(0)
    }

    fn get_quarterly_amount(&self, category: &str, quarter: i32) -> i64 {
        let code = self.period_code(BudgetGranularity::Quarterly, quarter);
        self.get_snapshot_amount(category, code).unwrap_or(0)
    }

    fn get_weekly_amount(&self, category: &str, week: i32) -> i64 {
        let code = self.period_code(BudgetGranularity::Weekly, week);
        self.get_snapshot_amount(category, code).unwrap_or(0)
    }

    fn apply_budget_override(&mut self, category: &str, period_code: i32, amount_cents: i64) {
        let _ = save_budget_snapshot(
            &self.conn,
            self.household_id,
            category,
            self.budget_year,
            period_code,
            amount_cents,
            true,
        );

        // Only delete computed defaults for this specific period
        if period_code > 0 {
            let _ = self.conn.execute(
        "DELETE FROM budget_snapshots WHERE household_id=?1 AND category=?2 AND year=?3 AND period_code=?4 AND is_override=0",
        params![self.household_id, category, self.budget_year, period_code],
      );
        }
    }
}

// underline-style tab (Notion). Active tab gets a 2px accent
// underline; inactive tabs are muted. No fill, no pill.
fn tab_button(ui: &mut egui::Ui, current: &mut Tab, tab: Tab, label: &str) {
    let selected = *current == tab;
    let padding = egui::Margin::symmetric(crate::ui::theme_tokens::SPACE_2 as i8, 0);
    let frame = egui::Frame::NONE.inner_margin(padding);
    let response = frame
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(crate::ui::components::tab_label(ui, label, selected))
                    .selectable(false)
                    .sense(egui::Sense::click()),
            )
            .interact(egui::Sense::click())
        })
        .inner;
    if response.clicked() {
        *current = tab;
    }
    crate::ui::components::tab_underline(ui, response.rect, selected);
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    fn snap(year: i32, code: i32, cents: i64, is_override: bool) -> BudgetSnapshot {
        BudgetSnapshot {
            id: 0,
            category: "Food".into(),
            year,
            period_code: code,
            amount_cents: cents,
            is_override,
        }
    }

    #[test]
    fn budget_never_bleeds_across_years() {
        // 2026 has a yearly cap override of $1,200 and a March override of $150.
        let snaps_2026 = [snap(2026, 0, 120_000, true), snap(2026, 3, 15_000, true)];

        // The old bug: pricing 2008 with 2026's data. Pricing 2008 uses 2008's
        // (empty) snapshots + a 2008 cap of 0 → $0, never 2026's numbers.
        assert_eq!(TwoCentsApp::price_budget("Food", 3, &[], 0, 12), 0);

        // 2026 with its own data: snapshot override wins; cap divides otherwise.
        assert_eq!(
            TwoCentsApp::price_budget("Food", 3, &snaps_2026, 120_000, 12),
            15_000
        );
        assert_eq!(
            TwoCentsApp::price_budget("Food", 5, &snaps_2026, 120_000, 12),
            10_000
        );

        // Zero cap → $0 when no snapshot matches (other category).
        assert_eq!(TwoCentsApp::price_budget("Rent", 0, &snaps_2026, 0, 1), 0);
    }

    #[test]
    fn cap_lookup_is_year_scoped() {
        let conn = Connection::open_in_memory().expect("db");
        conn.execute_batch(
      "CREATE TABLE budgets (id INTEGER PRIMARY KEY, household_id INTEGER, category TEXT, amount_cents INTEGER, year INTEGER, month INTEGER, UNIQUE(household_id, category, year, month));
       CREATE TABLE budget_snapshots (id INTEGER PRIMARY KEY, household_id INTEGER, category TEXT, year INTEGER, period_code INTEGER, amount_cents INTEGER, is_override INTEGER, UNIQUE(household_id, category, year, period_code));",
    ).expect("schema");
        save_budget_snapshot(&conn, 1, "Food", 2026, 0, 120_000, true).expect("save");
        save_budget(&conn, 1, "Rent", 30_000, 2026, 0).expect("save legacy");

        // The viewed year's snapshots carry the cap; a year with no rows gives
        // none — the app must query exactly that year, never reuse another's.
        let owned = load_budget_snapshots_for_year(&conn, 1, 2026).expect("load");
        assert!(owned
            .iter()
            .any(|s| s.period_code == 0 && s.is_override && s.amount_cents == 120_000));
        assert!(load_budget_snapshots_for_year(&conn, 1, 2008)
            .expect("load")
            .is_empty());

        // Legacy fallback is year-scoped too.
        let legacy_cap = |year: i32| -> i64 {
            load_budgets(&conn, 1, year, 0)
                .ok()
                .and_then(|l| l.into_iter().find(|b| b.category == "Rent"))
                .map(|b| b.amount_cents)
                .unwrap_or(0)
        };
        assert_eq!(legacy_cap(2026), 30_000);
        assert_eq!(legacy_cap(2008), 0);
    }
}
