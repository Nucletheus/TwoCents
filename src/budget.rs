use crate::db::{
    copy_budget_year as copy_budget_year_db, load_budget_snapshots_for_year, replace_budget_plan,
};
use crate::models::{
    budget_period_code, budget_period_count, budget_period_date_range, budget_period_for_date,
    budget_period_start, excluded_category_labels, BudgetGranularity, BudgetSnapshot, Category,
    Expense,
};
use crate::TwoCentsApp;
use chrono::{Datelike, Duration, NaiveDate};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub enum BudgetEdit {
    Yearly {
        amount: i64,
    },
    Period {
        granularity: BudgetGranularity,
        period: i32,
        amount: i64,
    },
}

fn split_evenly(total: i64, count: usize) -> Vec<i64> {
    if count == 0 {
        return Vec::new();
    }
    let total = total.max(0);
    let base = total / count as i64;
    let remainder = total % count as i64;
    let extra_start = count - remainder as usize;
    (0..count)
        .map(|index| base + i64::from(index >= extra_start))
        .collect()
}

fn dates_between(start: NaiveDate, end: NaiveDate) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut date = start;
    while date <= end {
        dates.push(date);
        date += Duration::days(1);
    }
    dates
}

fn add_split_days(
    days: &mut HashMap<NaiveDate, i64>,
    start: NaiveDate,
    end: NaiveDate,
    amount: i64,
) {
    let dates = dates_between(start, end);
    let count = dates.len();
    for (date, share) in dates.into_iter().zip(split_evenly(amount, count)) {
        *days.entry(date).or_default() += share;
    }
}

fn add_month_days(days: &mut HashMap<NaiveDate, i64>, year: i32, month: i32, amount: i64) {
    let (start, end) = budget_period_date_range(BudgetGranularity::Monthly, year, month);
    add_split_days(days, start, end, amount);
}

fn add_quarter_days(days: &mut HashMap<NaiveDate, i64>, year: i32, quarter: i32, amount: i64) {
    let start_month = (quarter - 1) * 3 + 1;
    for (month, share) in (start_month..start_month + 3).zip(split_evenly(amount, 3)) {
        add_month_days(days, year, month, share);
    }
}

fn actual_spend_between(
    expenses: &[Expense],
    categories: &[Category],
    category: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> i64 {
    if end <= start {
        return 0;
    }
    let excluded = excluded_category_labels(categories);
    let mut total = 0i64;
    for expense in expenses {
        if expense.amount_cents >= 0
            || expense.category.trim() != category
            || excluded.contains(&expense.category)
        {
            continue;
        }
        let Ok(date) = NaiveDate::parse_from_str(&expense.date, "%Y-%m-%d") else {
            continue;
        };
        if date >= start && date < end {
            let amount = expense.amount_cents.checked_neg().unwrap_or(i64::MAX);
            total = total.saturating_add(amount);
        }
    }
    total
}

fn snapshot(
    values: &mut HashMap<i32, BudgetSnapshot>,
    category: &str,
    year: i32,
    code: i32,
    amount: i64,
    is_override: bool,
) {
    let id = values.get(&code).map(|value| value.id).unwrap_or(0);
    values.insert(
        code,
        BudgetSnapshot {
            id,
            category: category.to_string(),
            year,
            period_code: code,
            amount_cents: amount,
            is_override,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn period_amount(
    gran: BudgetGranularity,
    year: i32,
    period: i32,
    boundary: NaiveDate,
    day_amounts: &HashMap<NaiveDate, i64>,
    expenses: &[Expense],
    categories: &[Category],
    category: &str,
) -> i64 {
    let (start, end) = budget_period_date_range(gran, year, period);
    if end < boundary {
        return 0;
    }
    let mut amount = if start < boundary {
        actual_spend_between(expenses, categories, category, start, boundary)
    } else {
        0
    };
    let from = start.max(boundary);
    for date in dates_between(from, end) {
        amount = amount.saturating_add(day_amounts.get(&date).copied().unwrap_or(0));
    }
    amount
}

#[allow(clippy::too_many_arguments)]
pub fn build_budget_plan(
    year: i32,
    today: NaiveDate,
    category: &str,
    amount: i64,
    edit: BudgetEdit,
    expenses: &[Expense],
    categories: &[Category],
    existing: &[BudgetSnapshot],
) -> Option<Vec<BudgetSnapshot>> {
    if category.trim().is_empty() || amount < 0 {
        return None;
    }
    let category = category.trim();

    let (source_gran, source_period, explicit_source, boundary, day_amounts, cap) = match edit {
        BudgetEdit::Yearly { amount } => {
            let start_month = if year == today.year() {
                today.month() as i32
            } else {
                1
            };
            if !(1..=12).contains(&start_month) {
                return None;
            }
            let boundary = budget_period_start(BudgetGranularity::Monthly, year, start_month);
            let actual_before = actual_spend_between(
                expenses,
                categories,
                category,
                NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
                boundary,
            );
            let remaining = amount.saturating_sub(actual_before).max(0);
            let months = (13 - start_month) as usize;
            let mut days = HashMap::new();
            for (month, share) in (start_month..=12).zip(split_evenly(remaining, months)) {
                add_month_days(&mut days, year, month, share);
            }
            (
                BudgetGranularity::Monthly,
                start_month,
                false,
                boundary,
                days,
                actual_before.saturating_add(remaining),
            )
        }
        BudgetEdit::Period {
            granularity,
            period,
            amount,
        } => {
            let count = budget_period_count(granularity, year);
            if period < 1 || period > count {
                return None;
            }
            let boundary = budget_period_start(granularity, year, period);
            let actual_before = actual_spend_between(
                expenses,
                categories,
                category,
                NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),
                boundary,
            );
            let mut days = HashMap::new();
            for source_period in period..=count {
                match granularity {
                    BudgetGranularity::Monthly => {
                        add_month_days(&mut days, year, source_period, amount)
                    }
                    BudgetGranularity::Quarterly => {
                        add_quarter_days(&mut days, year, source_period, amount)
                    }
                    BudgetGranularity::Weekly => {
                        let (start, end) =
                            budget_period_date_range(granularity, year, source_period);
                        add_split_days(&mut days, start, end, amount);
                    }
                    BudgetGranularity::Yearly => {}
                }
            }
            (
                granularity,
                period,
                true,
                boundary,
                days,
                actual_before.saturating_add(amount.saturating_mul((count - period + 1) as i64)),
            )
        }
    };

    if boundary.year() != year {
        return None;
    }
    let mut values = HashMap::new();
    for value in existing {
        if value.category == category && value.year == year {
            let code = crate::models::normalize_budget_period_code(year, value.period_code);
            values.entry(code).or_insert_with(|| BudgetSnapshot {
                id: value.id,
                category: value.category.clone(),
                year: value.year,
                period_code: code,
                amount_cents: value.amount_cents,
                is_override: value.is_override,
            });
        }
    }

    snapshot(&mut values, category, year, 0, cap, true);

    for gran in [
        BudgetGranularity::Quarterly,
        BudgetGranularity::Monthly,
        BudgetGranularity::Weekly,
    ] {
        let count = budget_period_count(gran, year);
        for period in 1..=count {
            let code = budget_period_code(gran, period);
            let (start, end) = budget_period_date_range(gran, year, period);
            let source_period = explicit_source && gran == source_gran && period >= source_period;
            if source_period {
                snapshot(&mut values, category, year, code, amount, true);
            } else if end >= boundary {
                let value = period_amount(
                    gran,
                    year,
                    period,
                    boundary,
                    &day_amounts,
                    expenses,
                    categories,
                    category,
                );
                snapshot(&mut values, category, year, code, value, false);
            }
        }
    }

    let mut result = values.into_values().collect::<Vec<_>>();
    result.sort_by_key(|value| match value.period_code {
        0 => i32::MIN,
        1..=12 => value.period_code,
        21..=24 => value.period_code,
        1001..=1054 => value.period_code,
        _ => i32::MAX,
    });
    Some(result)
}

pub(crate) fn save_budget_plan(
    app: &mut TwoCentsApp,
    category: &str,
    amount: i64,
) -> Result<(), String> {
    app.flush_deferred_expense_commits();
    let today = chrono::Local::now().date_naive();
    if app.budget_year < today.year() {
        return Err("Past budget years are read-only.".to_string());
    }
    let edit = if app.budget_granularity == BudgetGranularity::Yearly {
        BudgetEdit::Yearly { amount }
    } else {
        let period = match app.budget_granularity {
            BudgetGranularity::Monthly => app.budget_month,
            BudgetGranularity::Quarterly => app.budget_quarter as i32,
            BudgetGranularity::Weekly => app.budget_week as i32,
            BudgetGranularity::Yearly => 1,
        };
        BudgetEdit::Period {
            granularity: app.budget_granularity,
            period,
            amount,
        }
    };
    let existing = load_budget_snapshots_for_year(&app.conn, app.household_id, app.budget_year)
        .map_err(|error| error.to_string())?;
    let expenses = app.expenses.clone();
    let categories = app.categories.clone();
    let plan = build_budget_plan(
        app.budget_year,
        today,
        category,
        amount,
        edit,
        &expenses,
        &categories,
        &existing,
    )
    .ok_or_else(|| "Invalid budget period.".to_string())?;
    replace_budget_plan(
        &mut app.conn,
        app.household_id,
        category,
        app.budget_year,
        &plan,
    )
    .map_err(|error| error.to_string())?;
    app.reload();
    Ok(())
}

pub(crate) fn copy_budget_year(app: &mut TwoCentsApp) -> Result<usize, String> {
    app.flush_deferred_expense_commits();
    let source_year = app.budget_year - 1;
    let copied = copy_budget_year_db(
        &mut app.conn,
        app.household_id,
        source_year,
        app.budget_year,
    )
    .map_err(|error| error.to_string())?;
    app.reload();
    Ok(copied)
}

pub(crate) fn budget_current_period(year: i32, gran: BudgetGranularity, today: NaiveDate) -> i32 {
    budget_period_for_date(gran, year, today).unwrap_or_else(|| match gran {
        BudgetGranularity::Yearly => 1,
        BudgetGranularity::Quarterly => ((today.month() - 1) / 3 + 1) as i32,
        BudgetGranularity::Monthly => today.month() as i32,
        BudgetGranularity::Weekly => 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Category;
    use eframe::egui::Color32;
    use rusqlite::Connection;

    fn expense(date: &str, amount: i64) -> Expense {
        Expense {
            id: 0,
            date: date.to_string(),
            amount_input: String::new(),
            amount_cents: amount,
            member: String::new(),
            category: "Food".to_string(),
            vendor: String::new(),
            description: String::new(),
            account_id: 1,
            account: String::new(),
        }
    }

    fn category() -> Category {
        Category {
            id: 1,
            name: "Food".to_string(),
            parent_id: None,
            color: Color32::RED,
            excluded: false,
        }
    }

    fn amount(plan: &[BudgetSnapshot], code: i32) -> i64 {
        plan.iter()
            .find(|snapshot| snapshot.period_code == code)
            .map(|snapshot| snapshot.amount_cents)
            .unwrap_or(0)
    }

    #[test]
    fn yearly_plan_subtracts_actual_spending_before_current_month() {
        let expenses = vec![expense("2026-01-15", -40_000)];
        let plan = build_budget_plan(
            2026,
            NaiveDate::from_ymd_opt(2026, 9, 24).unwrap(),
            "Food",
            120_000,
            BudgetEdit::Yearly { amount: 120_000 },
            &expenses,
            &[category()],
            &[],
        )
        .expect("plan");
        assert_eq!(amount(&plan, 0), 120_000);
        assert_eq!(amount(&plan, 9), 20_000);
        assert_eq!(amount(&plan, 10), 20_000);
        assert_eq!(amount(&plan, 11), 20_000);
        assert_eq!(amount(&plan, 12), 20_000);
    }

    #[test]
    fn period_edit_replaces_the_suffix_and_preserves_prefix() {
        let existing = vec![BudgetSnapshot {
            id: 1,
            category: "Food".to_string(),
            year: 2026,
            period_code: 8,
            amount_cents: 5_000,
            is_override: true,
        }];
        let expenses = vec![expense("2026-01-15", -40_000)];
        let plan = build_budget_plan(
            2026,
            NaiveDate::from_ymd_opt(2026, 9, 24).unwrap(),
            "Food",
            10_000,
            BudgetEdit::Period {
                granularity: BudgetGranularity::Monthly,
                period: 9,
                amount: 10_000,
            },
            &expenses,
            &[category()],
            &existing,
        )
        .expect("plan");
        assert_eq!(amount(&plan, 0), 80_000);
        assert_eq!(amount(&plan, 8), 5_000);
        assert_eq!(amount(&plan, 9), 10_000);
        assert_eq!(amount(&plan, 12), 10_000);
    }

    #[test]
    fn persisted_plans_are_year_scoped_and_copyable() {
        let mut conn = Connection::open_in_memory().expect("db");
        conn.execute_batch(
            "CREATE TABLE budget_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                household_id INTEGER NOT NULL,
                category TEXT NOT NULL,
                year INTEGER NOT NULL,
                period_code INTEGER NOT NULL,
                amount_cents INTEGER NOT NULL,
                is_override INTEGER NOT NULL,
                UNIQUE(household_id, category, year, period_code)
            );
            CREATE TABLE budgets (
                id INTEGER PRIMARY KEY,
                household_id INTEGER NOT NULL,
                category TEXT NOT NULL,
                amount_cents INTEGER NOT NULL,
                year INTEGER NOT NULL,
                month INTEGER NOT NULL
            );",
        )
        .expect("schema");
        let plan = build_budget_plan(
            2026,
            NaiveDate::from_ymd_opt(2026, 9, 24).unwrap(),
            "Food",
            120_000,
            BudgetEdit::Yearly { amount: 120_000 },
            &[],
            &[category()],
            &[],
        )
        .expect("plan");
        crate::db::replace_budget_plan(&mut conn, 1, "Food", 2026, &plan).expect("replace");
        assert!(crate::db::load_budget_snapshots_for_year(&conn, 1, 2025)
            .expect("load")
            .is_empty());
        let copied = crate::db::copy_budget_year(&mut conn, 1, 2026, 2027).expect("copy");
        assert!(copied > 0);
        assert_eq!(
            amount(
                &crate::db::load_budget_snapshots_for_year(&conn, 1, 2027).expect("load"),
                0
            ),
            120_000
        );
    }

    #[test]
    fn calendar_weeks_stay_inside_the_year() {
        let (start, end) = budget_period_date_range(BudgetGranularity::Weekly, 2021, 1);
        assert_eq!(start, NaiveDate::from_ymd_opt(2021, 1, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2021, 1, 3).unwrap());
        assert!(end.year() == 2021);
        assert_eq!(
            crate::models::normalize_budget_period_code(2021, 101),
            budget_period_code(BudgetGranularity::Weekly, 2)
        );
        assert_eq!(budget_period_count(BudgetGranularity::Weekly, 2023), 53);
    }
}
