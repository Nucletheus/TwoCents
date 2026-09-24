use chrono::{Datelike, Duration, NaiveDate};
use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// GridRow trait — abstracts the editable fields shared by Expense & ImportRow
// ---------------------------------------------------------------------------
pub trait GridRow {
    fn row_date(&self) -> &str;
    fn row_date_mut(&mut self) -> &mut String;
    fn row_amount_input(&self) -> &str;
    fn row_amount_input_mut(&mut self) -> &mut String;
    fn row_amount_cents(&self) -> i64;
    fn set_row_amount_cents(&mut self, cents: i64);
    fn row_member(&self) -> &str;
    fn row_member_mut(&mut self) -> &mut String;
    fn row_category(&self) -> &str;
    fn row_category_mut(&mut self) -> &mut String;
    fn row_vendor(&self) -> &str;
    fn row_vendor_mut(&mut self) -> &mut String;
    fn row_description(&self) -> &str;
    fn row_description_mut(&mut self) -> &mut String;
    fn row_account(&self) -> &str;
    fn row_account_mut(&mut self) -> &mut String;
}

pub const CATEGORY_LABEL_SEP: &str = " › ";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Accounts,
    Expenses,
    Budgets,
    Goals,
    Analytics,
    Settlements,
    Households,
}

#[derive(Clone)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub balance_cents: i64,
    pub csv_name: Option<String>,
}

#[derive(Clone)]
pub struct Expense {
    pub id: i32,
    pub date: String,
    pub amount_input: String,
    pub amount_cents: i64,
    pub member: String,
    pub category: String,
    pub vendor: String,
    pub description: String,
    pub account_id: i64,
    pub account: String,
}

impl GridRow for Expense {
    fn row_date(&self) -> &str {
        &self.date
    }
    fn row_date_mut(&mut self) -> &mut String {
        &mut self.date
    }
    fn row_amount_input(&self) -> &str {
        &self.amount_input
    }
    fn row_amount_input_mut(&mut self) -> &mut String {
        &mut self.amount_input
    }
    fn row_amount_cents(&self) -> i64 {
        self.amount_cents
    }
    fn set_row_amount_cents(&mut self, cents: i64) {
        self.amount_cents = cents;
    }
    fn row_member(&self) -> &str {
        &self.member
    }
    fn row_member_mut(&mut self) -> &mut String {
        &mut self.member
    }
    fn row_category(&self) -> &str {
        &self.category
    }
    fn row_category_mut(&mut self) -> &mut String {
        &mut self.category
    }
    fn row_vendor(&self) -> &str {
        &self.vendor
    }
    fn row_vendor_mut(&mut self) -> &mut String {
        &mut self.vendor
    }
    fn row_description(&self) -> &str {
        &self.description
    }
    fn row_description_mut(&mut self) -> &mut String {
        &mut self.description
    }
    fn row_account(&self) -> &str {
        &self.account
    }
    fn row_account_mut(&mut self) -> &mut String {
        &mut self.account
    }
}

#[derive(Clone)]
pub struct ImportRow {
    pub date: String,
    pub amount_input: String,
    pub amount_cents: i64,
    pub member: String,
    pub category: String,
    pub vendor: String,
    pub description: String,
    pub account: String,
}

impl GridRow for ImportRow {
    fn row_date(&self) -> &str {
        &self.date
    }
    fn row_date_mut(&mut self) -> &mut String {
        &mut self.date
    }
    fn row_amount_input(&self) -> &str {
        &self.amount_input
    }
    fn row_amount_input_mut(&mut self) -> &mut String {
        &mut self.amount_input
    }
    fn row_amount_cents(&self) -> i64 {
        self.amount_cents
    }
    fn set_row_amount_cents(&mut self, cents: i64) {
        self.amount_cents = cents;
    }
    fn row_member(&self) -> &str {
        &self.member
    }
    fn row_member_mut(&mut self) -> &mut String {
        &mut self.member
    }
    fn row_category(&self) -> &str {
        &self.category
    }
    fn row_category_mut(&mut self) -> &mut String {
        &mut self.category
    }
    fn row_vendor(&self) -> &str {
        &self.vendor
    }
    fn row_vendor_mut(&mut self) -> &mut String {
        &mut self.vendor
    }
    fn row_description(&self) -> &str {
        &self.description
    }
    fn row_description_mut(&mut self) -> &mut String {
        &mut self.description
    }
    fn row_account(&self) -> &str {
        &self.account
    }
    fn row_account_mut(&mut self) -> &mut String {
        &mut self.account
    }
}

#[derive(Clone)]
pub struct HouseholdMember {
    pub id: i64,
    pub name: String,
    pub is_self: bool,
    pub color: Color32,
}

#[derive(Clone)]
pub struct MemberColorPopup {
    pub id: i64,
    pub name: String,
    pub color: Color32,
}

#[derive(Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: Color32,
    pub excluded: bool,
}

impl Category {
    pub fn full_label<'a>(&self, parents: &HashMap<i64, &'a Category>) -> String {
        match self.parent_id {
            None => self.name.clone(),
            Some(parent_id) => {
                let parent_name = parents
                    .get(&parent_id)
                    .map(|parent| parent.name.as_str())
                    .unwrap_or("?");
                format!("{parent_name}{CATEGORY_LABEL_SEP}{}", self.name)
            }
        }
    }
}

pub fn category_parent_map<'a>(categories: &'a [Category]) -> HashMap<i64, &'a Category> {
    categories
        .iter()
        .map(|category| (category.id, category))
        .collect()
}

pub fn sorted_parent_category_ids(categories: &[Category]) -> Vec<i64> {
    let mut parent_ids: Vec<i64> = categories
        .iter()
        .filter(|category| category.parent_id.is_none())
        .map(|category| category.id)
        .collect();
    parent_ids.sort_by(|a, b| {
        let name_a = categories
            .iter()
            .find(|category| category.id == *a)
            .map(|category| category.name.as_str())
            .unwrap_or("");
        let name_b = categories
            .iter()
            .find(|category| category.id == *b)
            .map(|category| category.name.as_str())
            .unwrap_or("");
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });
    parent_ids
}

pub fn category_assignable_labels(categories: &[Category]) -> Vec<String> {
    let parents = category_parent_map(categories);
    let mut parent_has_children = HashSet::new();
    for category in categories {
        if let Some(parent_id) = category.parent_id {
            parent_has_children.insert(parent_id);
        }
    }
    let mut labels: Vec<String> = categories
        .iter()
        .filter_map(|category| {
            if category.parent_id.is_some() {
                Some(category.full_label(&parents))
            } else if !parent_has_children.contains(&category.id) {
                Some(category.name.clone())
            } else {
                None
            }
        })
        .collect();
    labels.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    labels
}

pub fn find_category_by_label<'a>(categories: &'a [Category], label: &str) -> Option<&'a Category> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parents = category_parent_map(categories);
    categories.iter().find(|category| {
        category.full_label(&parents).eq_ignore_ascii_case(trimmed)
            || (category.parent_id.is_none() && category.name.eq_ignore_ascii_case(trimmed))
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GridColumn {
    Date,
    Amount,
    Member,
    Category,
    Vendor,
    Description,
    Account,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExpenseSortColumn {
    Date,
    Amount,
    Member,
    Category,
    Vendor,
    Description,
    Account,
}

impl From<ExpenseSortColumn> for GridColumn {
    fn from(column: ExpenseSortColumn) -> Self {
        match column {
            ExpenseSortColumn::Date => GridColumn::Date,
            ExpenseSortColumn::Amount => GridColumn::Amount,
            ExpenseSortColumn::Member => GridColumn::Member,
            ExpenseSortColumn::Category => GridColumn::Category,
            ExpenseSortColumn::Vendor => GridColumn::Vendor,
            ExpenseSortColumn::Description => GridColumn::Description,
            ExpenseSortColumn::Account => GridColumn::Account,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum GridNav {
    NextRow,
    PrevRow,
    NextCol,
    PrevCol,
}

#[derive(Clone)]
pub struct GridSelection {
    pub column: GridColumn,
    pub rows: Vec<usize>,
}

#[derive(Clone, Copy)]
pub struct GridSelectDrag {
    pub column: GridColumn,
    pub anchor: usize,
    pub end: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ExpenseSort {
    pub column: ExpenseSortColumn,
    pub ascending: bool,
}

impl Default for ExpenseSort {
    fn default() -> Self {
        Self {
            column: ExpenseSortColumn::Date,
            ascending: false,
        }
    }
}

#[derive(Clone)]
pub struct CategoryColorPopup {
    pub id: i64,
    pub name: String,
    pub color: Color32,
}

#[derive(Clone, Copy, Debug)]
pub struct GridPendingKeyboard {
    pub column: GridColumn,
    pub expense_idx: usize,
    pub action: GridKeyboardAction,
}

#[derive(Clone, Copy, Debug)]
pub enum GridKeyboardAction {
    Enter,
    Tab { shift: bool },
}

impl GridKeyboardAction {
    pub fn to_nav(self) -> GridNav {
        match self {
            Self::Enter => GridNav::NextRow,
            Self::Tab { shift: true } => GridNav::PrevCol,
            Self::Tab { shift: false } => GridNav::NextCol,
        }
    }
}

#[derive(Default, Clone)]
pub struct GridState {
    pub selection: Option<GridSelection>,
    pub drag: Option<GridSelectDrag>,
    pub edit_cell: Option<(GridColumn, usize)>,
    pub edit_original: Option<String>,
    pub typeahead: Option<char>,
    pub scroll_offset: f32,
    pub pending_focus_target: Option<(GridColumn, usize)>,
    pub pending_keyboard: Option<GridPendingKeyboard>,
    pub active_cell: Option<(GridColumn, usize)>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Budget {
    pub id: i64,
    pub category: String,
    pub amount_cents: i64,
    pub year: i32,
    pub month: i32,
}

#[derive(Debug, Clone)]
pub struct BudgetSnapshot {
    pub id: i64,
    pub category: String,
    pub year: i32,
    pub period_code: i32,
    pub amount_cents: i64,
    pub is_override: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BudgetFilter {
    All,
    OverBudget,
    UnderBudget,
    Unbudgeted,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BudgetGranularity {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone)]
pub struct CategorySplit {
    pub id: i64,
    pub category: String,
    pub member_name: String,
    pub percentage: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoDomain {
    Household,
    Settlements,
}

impl UndoDomain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Household => "household",
            Self::Settlements => "settlements",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "household" => Some(Self::Household),
            "settlements" => Some(Self::Settlements),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DestructiveConfirm {
    Category {
        id: i64,
        label: String,
        descendant_count: usize,
    },
    Member {
        id: i64,
        name: String,
    },
    Settlement {
        label: String,
        include_subcategories: bool,
        descendant_count: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingSettlementEdit {
    pub category: String,
    pub member_name: String,
    pub before: Option<f64>,
    pub value: f64,
}

impl PendingSettlementEdit {
    pub fn matches(&self, category: &str, member_name: &str) -> bool {
        self.category == category && self.member_name == member_name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingColorUndo {
    pub action_id: i64,
    pub target_id: i64,
    pub category: bool,
    pub before: HouseholdState,
    pub last_after: HouseholdState,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UndoCategory {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color_rgb: i32,
    pub excluded: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UndoMember {
    pub id: i64,
    pub name: String,
    pub is_self: bool,
    pub color_rgb: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UndoExpenseLink {
    pub id: i64,
    pub category: Option<String>,
    pub member: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UndoVendorRule {
    pub id: i64,
    pub vendor_pattern: String,
    pub category: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UndoSplit {
    pub id: i64,
    pub category: String,
    pub member_name: String,
    pub percentage: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HouseholdState {
    pub household_name: String,
    pub categories: Vec<UndoCategory>,
    pub members: Vec<UndoMember>,
    pub expense_links: Vec<UndoExpenseLink>,
    pub vendor_rules: Vec<UndoVendorRule>,
    pub splits: Vec<UndoSplit>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SettlementState {
    pub splits: Vec<UndoSplit>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UndoAction {
    HouseholdRename {
        before: HouseholdState,
        after: HouseholdState,
    },
    MemberAdd {
        before: HouseholdState,
        after: HouseholdState,
    },
    MemberDelete {
        before: HouseholdState,
        after: HouseholdState,
    },
    MemberRename {
        before: HouseholdState,
        after: HouseholdState,
    },
    MemberColor {
        before: HouseholdState,
        after: HouseholdState,
    },
    CategoryAdd {
        before: HouseholdState,
        after: HouseholdState,
    },
    CategoryDelete {
        before: HouseholdState,
        after: HouseholdState,
    },
    CategoryRename {
        before: HouseholdState,
        after: HouseholdState,
    },
    CategoryColor {
        before: HouseholdState,
        after: HouseholdState,
    },
    CategoryExclusion {
        before: HouseholdState,
        after: HouseholdState,
    },
    SettlementClear {
        before: SettlementState,
        after: SettlementState,
    },
    SettlementEqualSplit {
        before: SettlementState,
        after: SettlementState,
    },
    SettlementPercentage {
        before: SettlementState,
        after: SettlementState,
    },
}

impl UndoAction {
    pub fn domain(&self) -> UndoDomain {
        match self {
            Self::HouseholdRename { .. }
            | Self::MemberAdd { .. }
            | Self::MemberDelete { .. }
            | Self::MemberRename { .. }
            | Self::MemberColor { .. }
            | Self::CategoryAdd { .. }
            | Self::CategoryDelete { .. }
            | Self::CategoryRename { .. }
            | Self::CategoryColor { .. }
            | Self::CategoryExclusion { .. } => UndoDomain::Household,
            Self::SettlementClear { .. }
            | Self::SettlementEqualSplit { .. }
            | Self::SettlementPercentage { .. } => UndoDomain::Settlements,
        }
    }

    pub fn household_states(&self) -> Option<(&HouseholdState, &HouseholdState)> {
        match self {
            Self::HouseholdRename { before, after }
            | Self::MemberAdd { before, after }
            | Self::MemberDelete { before, after }
            | Self::MemberRename { before, after }
            | Self::MemberColor { before, after }
            | Self::CategoryAdd { before, after }
            | Self::CategoryDelete { before, after }
            | Self::CategoryRename { before, after }
            | Self::CategoryColor { before, after }
            | Self::CategoryExclusion { before, after } => Some((before, after)),
            _ => None,
        }
    }

    pub fn settlement_states(&self) -> Option<(&SettlementState, &SettlementState)> {
        match self {
            Self::SettlementClear { before, after }
            | Self::SettlementEqualSplit { before, after }
            | Self::SettlementPercentage { before, after } => Some((before, after)),
            _ => None,
        }
    }
}

// ---- Excluded categories ----------------------------------------------------
//
// any category can be flagged "excluded" in Settings (children
// inherit via the tree walk). Excluded rows store 0 and show "—" — they
// never enter budgets, analytics, or settlements. INCOME_PARENT is special
// for signs only: anything under it is a positive credit.

pub const INCOME_PARENT: &str = "Income";

/// Labels (name + full_label) of every category flagged excluded or under
/// an excluded ancestor — left out of all spending math and shown as "—".
pub fn excluded_category_labels(categories: &[Category]) -> HashSet<String> {
    let parents = category_parent_map(categories);
    let mut out = HashSet::new();
    for category in categories {
        let mut cursor = Some(category.id);
        let mut excluded = false;
        while let Some(id) = cursor {
            let Some(current) = parents.get(&id).copied() else {
                break;
            };
            if current.excluded {
                excluded = true;
                break;
            }
            cursor = current.parent_id;
        }
        if excluded {
            out.insert(category.name.clone());
            out.insert(category.full_label(&parents));
        }
    }
    out
}
pub fn budget_period_date_range(
    gran: BudgetGranularity,
    year: i32,
    period: i32,
) -> (NaiveDate, NaiveDate) {
    match gran {
        BudgetGranularity::Yearly => (year_start(year), year_end(year)),
        BudgetGranularity::Quarterly => {
            let start_month = ((period - 1) * 3 + 1) as u32;
            let end_month = start_month + 2;
            let end = if end_month == 12 {
                year_end(year)
            } else {
                NaiveDate::from_ymd_opt(year, (end_month + 1) as u32, 1).unwrap()
                    - Duration::days(1)
            };
            (NaiveDate::from_ymd_opt(year, start_month, 1).unwrap(), end)
        }
        BudgetGranularity::Monthly => {
            let end = if period == 12 {
                year_end(year)
            } else {
                NaiveDate::from_ymd_opt(year, (period + 1) as u32, 1).unwrap() - Duration::days(1)
            };
            (
                NaiveDate::from_ymd_opt(year, period as u32, 1).unwrap(),
                end,
            )
        }
        BudgetGranularity::Weekly => budget_week_range(year, period),
    }
}

pub fn budget_period_count(gran: BudgetGranularity, year: i32) -> i32 {
    match gran {
        BudgetGranularity::Yearly => 1,
        BudgetGranularity::Quarterly => 4,
        BudgetGranularity::Monthly => 12,
        BudgetGranularity::Weekly => budget_week_count(year),
    }
}

pub fn budget_period_start(gran: BudgetGranularity, year: i32, period: i32) -> NaiveDate {
    budget_period_date_range(gran, year, period).0
}

pub fn budget_period_for_date(gran: BudgetGranularity, year: i32, date: NaiveDate) -> Option<i32> {
    if date.year() != year {
        return None;
    }
    Some(match gran {
        BudgetGranularity::Yearly => 1,
        BudgetGranularity::Quarterly => ((date.month() - 1) / 3 + 1) as i32,
        BudgetGranularity::Monthly => date.month() as i32,
        BudgetGranularity::Weekly => budget_week_for_date(year, date),
    })
}

pub fn budget_period_code(gran: BudgetGranularity, period: i32) -> i32 {
    match gran {
        BudgetGranularity::Yearly => 0,
        BudgetGranularity::Quarterly => 20 + period,
        BudgetGranularity::Monthly => period,
        BudgetGranularity::Weekly => 1000 + period,
    }
}

pub fn budget_week_count(year: i32) -> i32 {
    let first = budget_week_start(year, 1);
    let last = year_end(year);
    (((last - first).num_days() / 7) + 1) as i32
}

pub fn budget_week_start(year: i32, period: i32) -> NaiveDate {
    let first = year_start(year);
    let offset = first.weekday().num_days_from_monday() as i64;
    first - Duration::days(offset) + Duration::days((period.max(1) - 1) as i64 * 7)
}

pub fn budget_week_for_date(year: i32, date: NaiveDate) -> i32 {
    let first = budget_week_start(year, 1);
    (((date - first).num_days() / 7) + 1) as i32
}

pub fn budget_week_range(year: i32, period: i32) -> (NaiveDate, NaiveDate) {
    let start = budget_week_start(year, period);
    let end = (start + Duration::days(6)).min(year_end(year));
    (start.max(year_start(year)), end)
}

pub fn normalize_budget_period_code(year: i32, code: i32) -> i32 {
    if !(100..=153).contains(&code) {
        return code;
    }
    let period = code - 100;
    let start = NaiveDate::from_isoywd_opt(year, period as u32, chrono::Weekday::Mon)
        .unwrap_or_else(|| year_start(year));
    let end = start + Duration::days(6);
    let clipped_start = start.max(year_start(year));
    let clipped_end = end.min(year_end(year));
    if clipped_start > clipped_end {
        code
    } else {
        budget_period_code(
            BudgetGranularity::Weekly,
            budget_week_for_date(year, clipped_start),
        )
    }
}

fn year_start(year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, 1, 1).unwrap()
}

fn year_end(year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, 12, 31).unwrap()
}

/// Sign for a stored amount given its category label: +1 (income credit),
/// −1 (debit). Excluded categories keep their real amount — the
/// flag only removes rows from aggregation math (budgets/analytics/
/// settlements), never from the grid or duplicate matching.
pub fn category_sign(categories: &[Category], label: &str) -> i64 {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return -1;
    }
    let parents = category_parent_map(categories);
    let Some(start) = categories
        .iter()
        .find(|c| c.full_label(&parents) == trimmed || c.name == trimmed)
    else {
        return -1;
    };
    let mut id = Some(start.id);
    while let Some(current_id) = id {
        let Some(current) = parents.get(&current_id).copied() else {
            break;
        };
        if current.name.eq_ignore_ascii_case(INCOME_PARENT) {
            return 1;
        }
        id = current.parent_id;
    }
    -1
}
