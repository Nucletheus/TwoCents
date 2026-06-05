use chrono::{Datelike, Duration, NaiveDate};
use std::collections::{HashMap, HashSet};

use crate::models::*;
use super::state::*;

#[derive(Clone)]
pub struct TimeSeriesPoint {
    pub date: NaiveDate,
    pub amount: f64,
    pub cumulative: f64,
}

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
pub struct CandlestickData {
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub label: String,
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

pub fn parse_expense_date(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

pub fn filter_expenses(
    expenses: &[Expense],
    state: &AnalyticsState,
) -> Vec<Expense> {
    let (start, end) = (state.date_start, state.date_end);
    
    expenses.iter()
        .filter(|exp| {
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
            if !state.selected_categories.is_empty() {
                let is_selected = state.selected_categories.contains(&exp.category);
                match state.category_filter_mode {
                    FilterMode::Include => {
                        if !is_selected {
                            return false;
                        }
                    }
                    FilterMode::Exclude => {
                        if is_selected {
                            return false;
                        }
                    }
                }
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

pub fn aggregate_time_series(
    expenses: &[Expense],
    granularity: AnalyticsGranularity,
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<TimeSeriesPoint> {
    // Group expenses by period
    let mut period_totals: HashMap<NaiveDate, f64> = HashMap::new();
    
    for exp in expenses {
        if let Some(exp_date) = parse_expense_date(&exp.date) {
            let period_start = match granularity {
                AnalyticsGranularity::Daily => exp_date,
                AnalyticsGranularity::Weekly => {
                    let weekday = exp_date.weekday().num_days_from_monday();
                    exp_date - Duration::days(weekday as i64)
                }
                AnalyticsGranularity::Monthly => {
                    exp_date.with_day(1).unwrap()
                }
                AnalyticsGranularity::Quarterly => {
                    let quarter_start_month = ((exp_date.month() - 1) / 3) * 3 + 1;
                    exp_date.with_day(1).unwrap().with_month(quarter_start_month).unwrap()
                }
                AnalyticsGranularity::Yearly => {
                    exp_date.with_ordinal(1).unwrap()
                }
            };
            
            *period_totals.entry(period_start).or_insert(0.0) += exp.amount_cents as f64 / 100.0;
        }
    }
    
    // Generate all periods in range
    let mut periods: Vec<NaiveDate> = Vec::new();
    let mut current = match granularity {
        AnalyticsGranularity::Daily => start,
        AnalyticsGranularity::Weekly => {
            let weekday = start.weekday().num_days_from_monday();
            start - Duration::days(weekday as i64)
        }
        AnalyticsGranularity::Monthly => start.with_day(1).unwrap(),
        AnalyticsGranularity::Quarterly => {
            let quarter_start_month = ((start.month() - 1) / 3) * 3 + 1;
            start.with_day(1).unwrap().with_month(quarter_start_month).unwrap()
        }
        AnalyticsGranularity::Yearly => start.with_ordinal(1).unwrap(),
    };
    
    while current <= end {
        periods.push(current);
        current = match granularity {
            AnalyticsGranularity::Daily => current + Duration::days(1),
            AnalyticsGranularity::Weekly => current + Duration::days(7),
            AnalyticsGranularity::Monthly => {
                if current.month() == 12 {
                    current.with_year(current.year() + 1).unwrap().with_month(1).unwrap()
                } else {
                    current.with_month(current.month() + 1).unwrap()
                }
            }
            AnalyticsGranularity::Quarterly => {
                let next_month = current.month() + 3;
                if next_month > 12 {
                    current.with_year(current.year() + 1).unwrap().with_month(next_month - 12).unwrap()
                } else {
                    current.with_month(next_month).unwrap()
                }
            }
            AnalyticsGranularity::Yearly => {
                current.with_year(current.year() + 1).unwrap()
            }
        };
    }
    
    // Build time series with cumulative totals
    let mut cumulative = 0.0;
    periods.iter()
        .map(|&period| {
            let amount = period_totals.get(&period).copied().unwrap_or(0.0);
            cumulative += amount;
            TimeSeriesPoint {
                date: period,
                amount,
                cumulative,
            }
        })
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
    
    // Calculate total for percentages
    let total: f64 = category_totals.values().sum();
    
    // Build result with colors and percentages
    let mut result: Vec<CategoryTotal> = category_totals
        .into_iter()
        .map(|(category, amount)| {
            let color = category_colors.get(&category).copied()
                .unwrap_or(eframe::egui::Color32::from_rgb(128, 128, 128));
            let percentage = if total > 0.0 {
                (amount / total) * 100.0
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
    
    // Sort by amount descending
    result.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal));
    
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
            let actual = actual_totals.get(&category).copied().unwrap_or(0.0);
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
    
    // Sort by absolute variance descending (biggest differences first)
    result.sort_by(|a, b| b.variance.abs().partial_cmp(&a.variance.abs()).unwrap_or(std::cmp::Ordering::Equal));
    
    result
}

pub fn aggregate_candlestick(
    expenses: &[Expense],
    granularity: AnalyticsGranularity,
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<CandlestickData> {
    // Group expenses by period and calculate daily cumulative spending within each period
    let mut period_expenses: HashMap<NaiveDate, Vec<(NaiveDate, f64)>> = HashMap::new();
    
    for exp in expenses {
        if let Some(exp_date) = parse_expense_date(&exp.date) {
            let period_start = match granularity {
                AnalyticsGranularity::Daily => exp_date,
                AnalyticsGranularity::Weekly => {
                    let weekday = exp_date.weekday().num_days_from_monday();
                    exp_date - Duration::days(weekday as i64)
                }
                AnalyticsGranularity::Monthly => {
                    exp_date.with_day(1).unwrap()
                }
                AnalyticsGranularity::Quarterly => {
                    let quarter_start_month = ((exp_date.month() - 1) / 3) * 3 + 1;
                    exp_date.with_day(1).unwrap().with_month(quarter_start_month).unwrap()
                }
                AnalyticsGranularity::Yearly => {
                    exp_date.with_ordinal(1).unwrap()
                }
            };
            
            let amount = exp.amount_cents as f64 / 100.0;
            period_expenses.entry(period_start).or_default().push((exp_date, amount));
        }
    }
    
    // Generate all periods in range
    let mut periods: Vec<NaiveDate> = Vec::new();
    let mut current = match granularity {
        AnalyticsGranularity::Daily => start,
        AnalyticsGranularity::Weekly => {
            let weekday = start.weekday().num_days_from_monday();
            start - Duration::days(weekday as i64)
        }
        AnalyticsGranularity::Monthly => start.with_day(1).unwrap(),
        AnalyticsGranularity::Quarterly => {
            let quarter_start_month = ((start.month() - 1) / 3) * 3 + 1;
            start.with_day(1).unwrap().with_month(quarter_start_month).unwrap()
        }
        AnalyticsGranularity::Yearly => start.with_ordinal(1).unwrap(),
    };
    
    while current <= end {
        periods.push(current);
        current = match granularity {
            AnalyticsGranularity::Daily => current + Duration::days(1),
            AnalyticsGranularity::Weekly => current + Duration::days(7),
            AnalyticsGranularity::Monthly => {
                if current.month() == 12 {
                    current.with_year(current.year() + 1).unwrap().with_month(1).unwrap()
                } else {
                    current.with_month(current.month() + 1).unwrap()
                }
            }
            AnalyticsGranularity::Quarterly => {
                let next_month = current.month() + 3;
                if next_month > 12 {
                    current.with_year(current.year() + 1).unwrap().with_month(next_month - 12).unwrap()
                } else {
                    current.with_month(next_month).unwrap()
                }
            }
            AnalyticsGranularity::Yearly => {
                current.with_year(current.year() + 1).unwrap()
            }
        };
    }
    
    // Build candlestick data for each period
    let mut result = Vec::new();
    
    for (i, &period_start) in periods.iter().enumerate() {
        let period_end = if i + 1 < periods.len() {
            periods[i + 1] - Duration::days(1)
        } else {
            end
        };
        
        let mut daily_expenses = period_expenses.get(&period_start).cloned().unwrap_or_default();
        daily_expenses.sort_by_key(|(date, _)| *date);
        
        // Calculate cumulative spending within the period
        let mut cumulative = 0.0;
        let mut daily_cumulative: Vec<f64> = Vec::new();
        
        for (_, amount) in &daily_expenses {
            cumulative += amount;
            daily_cumulative.push(cumulative);
        }
        
        // OHLC values
        let open = if !daily_cumulative.is_empty() {
            daily_cumulative[0]
        } else {
            0.0
        };
        
        let close = cumulative;
        
        let high = daily_cumulative.iter().cloned().fold(0.0_f64, f64::max);
        let low = daily_cumulative.iter().cloned().fold(f64::MAX, f64::min);
        let low = if low == f64::MAX { 0.0 } else { low };
        
        // Generate label
        let label = match granularity {
            AnalyticsGranularity::Daily => period_start.format("%b %d").to_string(),
            AnalyticsGranularity::Weekly => format!("W{}", period_start.iso_week().week()),
            AnalyticsGranularity::Monthly => period_start.format("%b %Y").to_string(),
            AnalyticsGranularity::Quarterly => {
                let quarter = (period_start.month() - 1) / 3 + 1;
                format!("Q{} {}", quarter, period_start.year())
            }
            AnalyticsGranularity::Yearly => period_start.format("%Y").to_string(),
        };
        
        result.push(CandlestickData {
            period_start,
            period_end,
            open,
            high,
            low,
            close,
            label,
        });
    }
    
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
    
    let mut totals_a: HashMap<String, f64> = HashMap::new();
    for exp in expenses_a {
        *totals_a.entry(exp.category.clone()).or_insert(0.0) += exp.amount_cents as f64 / 100.0;
    }
    
    let mut totals_b: HashMap<String, f64> = HashMap::new();
    for exp in expenses_b {
        *totals_b.entry(exp.category.clone()).or_insert(0.0) += exp.amount_cents as f64 / 100.0;
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
    
    category_comparisons.sort_by(|a, b| b.difference.abs().partial_cmp(&a.difference.abs()).unwrap_or(std::cmp::Ordering::Equal));
    
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
