use eframe::egui::{Color32, Id};
use std::collections::{HashMap, HashSet};

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
  pub name: String,
  pub kind: String,
  pub balance_cents: i64,
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
    .filter(|category| category.parent_id.is_none())
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

pub fn settings_new_parent_name_id() -> Id {
  Id::new("settings_new_parent_category_name")
}

pub fn settings_new_sub_name_id() -> Id {
  Id::new("settings_new_subcategory_name")
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
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExpenseSortColumn {
  Date,
  Amount,
  Member,
  Category,
  Vendor,
  Description,
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
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
