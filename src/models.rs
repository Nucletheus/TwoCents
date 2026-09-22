use eframe::egui::Color32;
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
  fn row_date(&self) -> &str { &self.date }
  fn row_date_mut(&mut self) -> &mut String { &mut self.date }
  fn row_amount_input(&self) -> &str { &self.amount_input }
  fn row_amount_input_mut(&mut self) -> &mut String { &mut self.amount_input }
  fn row_amount_cents(&self) -> i64 { self.amount_cents }
  fn set_row_amount_cents(&mut self, cents: i64) { self.amount_cents = cents; }
  fn row_member(&self) -> &str { &self.member }
  fn row_member_mut(&mut self) -> &mut String { &mut self.member }
  fn row_category(&self) -> &str { &self.category }
  fn row_category_mut(&mut self) -> &mut String { &mut self.category }
  fn row_vendor(&self) -> &str { &self.vendor }
  fn row_vendor_mut(&mut self) -> &mut String { &mut self.vendor }
  fn row_description(&self) -> &str { &self.description }
  fn row_description_mut(&mut self) -> &mut String { &mut self.description }
  fn row_account(&self) -> &str { &self.account }
  fn row_account_mut(&mut self) -> &mut String { &mut self.account }
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
  fn row_date(&self) -> &str { &self.date }
  fn row_date_mut(&mut self) -> &mut String { &mut self.date }
  fn row_amount_input(&self) -> &str { &self.amount_input }
  fn row_amount_input_mut(&mut self) -> &mut String { &mut self.amount_input }
  fn row_amount_cents(&self) -> i64 { self.amount_cents }
  fn set_row_amount_cents(&mut self, cents: i64) { self.amount_cents = cents; }
  fn row_member(&self) -> &str { &self.member }
  fn row_member_mut(&mut self) -> &mut String { &mut self.member }
  fn row_category(&self) -> &str { &self.category }
  fn row_category_mut(&mut self) -> &mut String { &mut self.category }
  fn row_vendor(&self) -> &str { &self.vendor }
  fn row_vendor_mut(&mut self) -> &mut String { &mut self.vendor }
  fn row_description(&self) -> &str { &self.description }
  fn row_description_mut(&mut self) -> &mut String { &mut self.description }
  fn row_account(&self) -> &str { &self.account }
  fn row_account_mut(&mut self) -> &mut String { &mut self.account }
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

// ---- Excluded categories ----------------------------------------------------
//
// ponytail: any category can be flagged "excluded" in Settings (children
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
      let Some(current) = parents.get(&id).copied() else { break };
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

/// Sign for a stored amount given its category label: +1 (income credit),
/// −1 (debit). ponytail: excluded categories keep their real amount — the
/// flag only removes rows from aggregation math (budgets/analytics/
/// settlements), never from the grid or duplicate matching.
pub fn category_sign(categories: &[Category], label: &str) -> i64 {
  let trimmed = label.trim();
  if trimmed.is_empty() {
    return -1;
  }
  let parents = category_parent_map(categories);
  let Some(start) = categories.iter().find(|c| c.full_label(&parents) == trimmed || c.name == trimmed) else {
    return -1;
  };
  let mut id = Some(start.id);
  while let Some(current_id) = id {
    let Some(current) = parents.get(&current_id).copied() else { break };
    if current.name.eq_ignore_ascii_case(INCOME_PARENT) {
      return 1;
    }
    id = current.parent_id;
  }
  -1
}


