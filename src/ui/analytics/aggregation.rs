use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};

use crate::models::*;
use super::state::*;

#[derive(Clone)]
pub struct CategoryTotal {
    pub category: String,
    pub amount: f64,
    pub percentage: f64,
    pub color: eframe::egui::Color32,
}

#[derive(Clone)]
pub struct BudgetComparison {
    pub category: String,
    pub budgeted: f64,
    pub actual: f64,
    pub variance: f64,
    pub variance_pct: f64,
    pub color: eframe::egui::Color32,
}

#[derive(Clone)]
pub struct PeriodComparisonData {
    pub period_a_label: String,
    pub period_b_label: String,
    pub period_a_total: f64,
    pub period_b_total: f64,
    pub difference: f64,
    pub difference_pct: f64,
    pub category_comparisons: Vec<CategoryComparison>,
}

#[derive(Clone)]
pub struct CategoryComparison {
    pub category: String,
    pub period_a_amount: f64,
    pub period_b_amount: f64,
    pub difference: f64,
    pub difference_pct: f64,
    pub color: eframe::egui::Color32,
}

/// char-boundary-safe truncation. `&s[..17]` panicked on
/// "Parent › Sub" labels (`›` is 3 bytes UTF-8); slice by chars instead.
pub fn truncate_label(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let kept: String = s.chars().take(max_chars).collect();
        format!("{}…", kept.trim_end())
    }
}

pub fn parse_expense_date(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

/// the include/exclude category predicate, shared by
/// `filter_expenses` and the Budget-vs-Actual budget-side filter — the
/// budget bars never used to honor the selection, so clicking a category
/// in that tab looked like a no-op. Empty selection passes everything.
pub fn passes_category_filter(state: &AnalyticsState, label: &str) -> bool {
    if state.selected_categories.is_empty() {
        return true;
    }
    let is_selected = state.selected_categories.contains(label);
    match state.category_filter_mode {
        FilterMode::Include => is_selected,
        FilterMode::Exclude => !is_selected,
    }
}

pub fn filter_expenses(
    expenses: &[Expense],
    state: &AnalyticsState,
    excluded_categories: &std::collections::HashSet<String>,
) -> Vec<Expense> {
    let (start, end) = (state.date_start, state.date_end);

    expenses.iter()
        .filter(|exp| {
            // analytics default to spending-only — income
            // (positive rows) chart only where the caller opts in via
            // `state.include_income` (Period Comparison). Excluded
            // transfer/payment categories never chart.
            if (!state.include_income && exp.amount_cents >= 0)
                || excluded_categories.contains(&exp.category)
            {
                return false;
            }

            // Date filter
            if let Some(exp_date) = parse_expense_date(&exp.date) {
                if let Some(start_date) = start {
                    if exp_date < start_date {
                        return false;
                    }
                }
                if let Some(end_date) = end {
                    if exp_date > end_date {
                        return false;
                    }
                }
            } else {
                return false;
            }
            
            // Category filter
            if !passes_category_filter(state, &exp.category) {
                return false;
            }

            // Member filter
            if !state.selected_members.is_empty() {
                if !state.selected_members.contains(&exp.member) {
                    return false;
                }
            }
            
            // Vendor filter
            if !state.selected_vendors.is_empty() {
                if !state.selected_vendors.contains(&exp.vendor) {
                    return false;
                }
            }
            
            true
        })
        .cloned()
        .collect()
}

pub fn aggregate_by_category(
    expenses: &[Expense],
    categories: &[Category],
) -> Vec<CategoryTotal> {
    // Build category color map
    let cat_parents = category_parent_map(categories);
    let mut category_colors: HashMap<String, eframe::egui::Color32> = HashMap::new();
    
    for category in categories {
        let label = category.full_label(&cat_parents);
        category_colors.insert(label, category.color);
    }
    
    // Group expenses by category
    let mut category_totals: HashMap<String, f64> = HashMap::new();
    
    for exp in expenses {
        *category_totals.entry(exp.category.clone()).or_insert(0.0) += exp.amount_cents as f64 / 100.0;
    }
    
    // Calculate total for percentages — abs, since expense amounts are
    // negative (spend) and a signed total <= 0 forced every share to 0.0%.
    let total: f64 = category_totals.values().map(|v| v.abs()).sum();
    
    // Build result with colors and percentages
    let mut result: Vec<CategoryTotal> = category_totals
        .into_iter()
        .map(|(category, amount)| {
            let color = category_colors.get(&category).copied()
                .unwrap_or(eframe::egui::Color32::from_rgb(128, 128, 128));
            let percentage = if total > 0.0 {
                (amount.abs() / total) * 100.0
            } else {
                0.0
            };
            CategoryTotal {
                category,
                amount,
                percentage,
                color,
            }
        })
        .collect();
    
    // Sort by amount descending, name ascending on ties — the HashMap
    // iteration order behind `category_totals` is not stable across
    // frames, so equal amounts used to reshuffle every repaint.
    result.sort_by(|a, b| {
        b.amount
            .partial_cmp(&a.amount)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.category.cmp(&b.category))
    });
    
    result
}

pub fn compare_budget_vs_actual(
    expenses: &[Expense],
    budgets: &std::collections::HashMap<String, i64>,
    categories: &[Category],
) -> Vec<BudgetComparison> {
    // Build category color map
    let cat_parents = category_parent_map(categories);
    let mut category_colors: std::collections::HashMap<String, eframe::egui::Color32> = std::collections::HashMap::new();
    
    for category in categories {
        let label = category.full_label(&cat_parents);
        category_colors.insert(label, category.color);
    }
    
    // Calculate actual spending per category
    let mut actual_totals: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for exp in expenses {
        *actual_totals.entry(exp.category.clone()).or_insert(0.0) += exp.amount_cents as f64 / 100.0;
    }
    
    // Get all categories that have either a budget or actual spending
    let mut all_categories: std::collections::HashSet<String> = std::collections::HashSet::new();
    for category in budgets.keys() {
        all_categories.insert(category.clone());
    }
    for category in actual_totals.keys() {
        all_categories.insert(category.clone());
    }
    
    // Build comparison data
    let mut result: Vec<BudgetComparison> = all_categories
        .into_iter()
        .map(|category| {
            let budgeted = budgets.get(&category).copied().unwrap_or(0) as f64 / 100.0;
            // abs: filter_expenses already drops income rows, but when this
            // runs on raw expenses (legacy callers) netting is wrong either
            // way — "actual spend" is a magnitude, and every bar/variance in
            // Budget vs Actual now points the same direction.
            let actual = actual_totals.get(&category).copied().unwrap_or(0.0).abs();
            let variance = budgeted - actual;
            let variance_pct = if budgeted > 0.0 {
                (variance / budgeted) * 100.0
            } else if actual > 0.0 {
                -100.0 // Over budget by 100% when no budget was set
            } else {
                0.0
            };
            
            let color = category_colors.get(&category).copied()
                .unwrap_or(eframe::egui::Color32::from_rgb(128, 128, 128));
            
            BudgetComparison {
                category,
                budgeted,
                actual,
                variance,
                variance_pct,
                color,
            }
        })
        .collect();
    
    // Sort by absolute variance descending (biggest differences first),
    // name ascending on ties — equal |variance| rows reshuffled every
    // frame otherwise (HashSet union order is not stable).
    result.sort_by(|a, b| {
        b.variance
            .abs()
            .partial_cmp(&a.variance.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.category.cmp(&b.category))
    });
    
    result
}

pub fn aggregate_period_comparison(
    expenses_a: &[Expense],
    expenses_b: &[Expense],
    period_a_label: &str,
    period_b_label: &str,
    categories: &[Category],
) -> PeriodComparisonData {
    let cat_parents = category_parent_map(categories);
    let mut category_colors: HashMap<String, eframe::egui::Color32> = HashMap::new();
    
    for category in categories {
        let label = category.full_label(&cat_parents);
        category_colors.insert(label, category.color);
    }
    
    // gross magnitude: each expense contributes its absolute amount (income
    // reads positive), so inflows never net against outflows in a category.
    let mut totals_a: HashMap<String, f64> = HashMap::new();
    for exp in expenses_a {
        *totals_a.entry(exp.category.clone()).or_insert(0.0) += exp.amount_cents.abs() as f64 / 100.0;
    }
    
    let mut totals_b: HashMap<String, f64> = HashMap::new();
    for exp in expenses_b {
        *totals_b.entry(exp.category.clone()).or_insert(0.0) += exp.amount_cents.abs() as f64 / 100.0;
    }
    
    let period_a_total: f64 = totals_a.values().sum();
    let period_b_total: f64 = totals_b.values().sum();
    let difference = period_b_total - period_a_total;
    let difference_pct = if period_a_total > 0.0 {
        (difference / period_a_total) * 100.0
    } else if period_b_total > 0.0 {
        100.0
    } else {
        0.0
    };
    
    let mut all_categories: HashSet<String> = HashSet::new();
    for cat in totals_a.keys() {
        all_categories.insert(cat.clone());
    }
    for cat in totals_b.keys() {
        all_categories.insert(cat.clone());
    }
    
    let mut category_comparisons: Vec<CategoryComparison> = all_categories
        .into_iter()
        .map(|category| {
            let a_amount = totals_a.get(&category).copied().unwrap_or(0.0);
            let b_amount = totals_b.get(&category).copied().unwrap_or(0.0);
            let diff = b_amount - a_amount;
            let diff_pct = if a_amount > 0.0 {
                (diff / a_amount) * 100.0
            } else if b_amount > 0.0 {
                100.0
            } else {
                0.0
            };
            
            let color = category_colors.get(&category).copied()
                .unwrap_or(eframe::egui::Color32::from_rgb(128, 128, 128));
            
            CategoryComparison {
                category,
                period_a_amount: a_amount,
                period_b_amount: b_amount,
                difference: diff,
                difference_pct: diff_pct,
                color,
            }
        })
        .collect();
    
    // |difference| desc, name asc on ties — zero-difference rows (and any
    // other ties) reshuffled frame-to-frame off the HashSet union order.
    category_comparisons.sort_by(|a, b| {
        b.difference
            .abs()
            .partial_cmp(&a.difference.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.category.cmp(&b.category))
    });
    
    PeriodComparisonData {
        period_a_label: period_a_label.to_string(),
        period_b_label: period_b_label.to_string(),
        period_a_total,
        period_b_total,
        difference,
        difference_pct,
        category_comparisons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(selected: &[&str], mode: FilterMode) -> AnalyticsState {
        AnalyticsState {
            category_filter_mode: mode,
            selected_categories: selected.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn passes_category_filter_modes() {
        // Empty selection passes everything.
        let state = state_with(&[], FilterMode::Include);
        assert!(passes_category_filter(&state, "Food"));
        assert!(passes_category_filter(&state, "Rent"));

        // Include: only selected labels pass.
        let state = state_with(&["Food"], FilterMode::Include);
        assert!(passes_category_filter(&state, "Food"));
        assert!(!passes_category_filter(&state, "Rent"));

        // Exclude: selected labels are dropped, everything else passes.
        let state = state_with(&["Food"], FilterMode::Exclude);
        assert!(!passes_category_filter(&state, "Food"));
        assert!(passes_category_filter(&state, "Rent"));
    }

    fn expense(category: &str, cents: i64) -> Expense {
        Expense {
            id: 0,
            date: "2026-09-01".to_string(),
            amount_input: String::new(),
            amount_cents: cents,
            member: "Me".to_string(),
            category: category.to_string(),
            vendor: String::new(),
            description: String::new(),
            account_id: 0,
            account: String::new(),
        }
    }

    fn cat(id: i64, name: &str) -> Category {
        Category {
            id,
            name: name.to_string(),
            parent_id: None,
            color: eframe::egui::Color32::GRAY,
            excluded: false,
        }
    }

    #[test]
    fn period_comparison_abs_and_stable_tie_order() {
        let categories = vec![cat(1, "Alpha"), cat(2, "Beta")];
        // Equal |difference| (both zero): name tie-break must give a
        // deterministic order regardless of HashMap union order.
        let a = vec![expense("Alpha", -500), expense("Beta", -500)];
        let b = vec![expense("Alpha", -500), expense("Beta", -500)];
        let data = aggregate_period_comparison(&a, &b, "A", "B", &categories);
        let names: Vec<&str> = data
            .category_comparisons
            .iter()
            .map(|c| c.category.as_str())
            .collect();
        assert_eq!(names, vec!["Alpha", "Beta"]);

        // Per-category and total sums are magnitudes: spending −$10 and
        // income +$30 in period B both read positive, totals = Σ|cat|.
        let a = vec![expense("Alpha", -1000)];
        let b = vec![expense("Alpha", -1000), expense("Alpha", 3000)];
        let data = aggregate_period_comparison(&a, &b, "A", "B", &categories);
        let alpha = &data.category_comparisons[0];
        assert_eq!(alpha.period_a_amount, 10.0);
        assert_eq!(alpha.period_b_amount, 40.0);
        assert_eq!(data.period_a_total, 10.0);
        assert_eq!(data.period_b_total, 40.0);
        assert_eq!(data.difference, 30.0);
    }
}
