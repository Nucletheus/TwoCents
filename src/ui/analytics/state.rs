use chrono::{Datelike, Duration, NaiveDate};
use std::collections::HashSet;

use crate::db::AnalyticsFilterRow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DatePreset {
    Custom,
    ThisMonth,
    LastMonth,
    Last3Months,
    Last6Months,
    YTD,
    LastYear,
    AllTime,
}

impl DatePreset {
    pub fn label(&self) -> &'static str {
        match self {
            DatePreset::Custom => "Custom",
            DatePreset::ThisMonth => "This Month",
            DatePreset::LastMonth => "Last Month",
            DatePreset::Last3Months => "Last 3 Months",
            DatePreset::Last6Months => "Last 6 Months",
            DatePreset::YTD => "YTD",
            DatePreset::LastYear => "Last Year",
            DatePreset::AllTime => "All Time",
        }
    }

    pub fn date_range(&self) -> (Option<NaiveDate>, Option<NaiveDate>) {
        let now = chrono::Local::now().date_naive();
        match self {
            DatePreset::Custom => (None, None),
            DatePreset::ThisMonth => {
                let start = now.with_day(1).unwrap();
                (Some(start), Some(now))
            }
            DatePreset::LastMonth => {
                let first_of_this_month = now.with_day(1).unwrap();
                let end = first_of_this_month - Duration::days(1);
                let start = end.with_day(1).unwrap();
                (Some(start), Some(end))
            }
            DatePreset::Last3Months => {
                let start = now - Duration::days(90);
                (Some(start), Some(now))
            }
            DatePreset::Last6Months => {
                let start = now - Duration::days(180);
                (Some(start), Some(now))
            }
            DatePreset::YTD => {
                let start = now.with_ordinal(1).unwrap();
                (Some(start), Some(now))
            }
            DatePreset::LastYear => {
                let last_year = now.year() - 1;
                let start = NaiveDate::from_ymd_opt(last_year, 1, 1).unwrap();
                let end = NaiveDate::from_ymd_opt(last_year, 12, 31).unwrap();
                (Some(start), Some(end))
            }
            DatePreset::AllTime => (None, None),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterMode {
    Include,
    Exclude,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnalyticsChart {
    CategoryBreakdown,
    BudgetVsActual,
    PeriodComparison,
}

impl AnalyticsChart {
    pub fn label(&self) -> &'static str {
        match self {
            AnalyticsChart::CategoryBreakdown => "Category Breakdown",
            AnalyticsChart::BudgetVsActual => "Budget vs Actual",
            AnalyticsChart::PeriodComparison => "Period Comparison",
        }
    }
}

/// shared Category/Cost row ordering for ALL three analytics
/// charts (breakdown legend, BvA picker/chart, PC picker/chart) — one
/// selection so switching tabs keeps your sort. Not persisted — no
/// schema migration; defaults to Category ascending each launch.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LegendSort {
    #[default]
    CategoryAsc,
    CategoryDesc,
    CostAsc,
    CostDesc,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BudgetViewPeriod {
    Week,
    #[default]
    Month,
    Quarter,
    Year,
}

impl BudgetViewPeriod {
    pub fn label(&self) -> &'static str {
        match self {
            BudgetViewPeriod::Week => "Week",
            BudgetViewPeriod::Month => "Month",
            BudgetViewPeriod::Quarter => "Quarter",
            BudgetViewPeriod::Year => "Year",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnalyticsState {
    pub date_start: Option<NaiveDate>,
    pub date_end: Option<NaiveDate>,
    pub date_preset: DatePreset,
    pub selected_categories: HashSet<String>,
    pub category_filter_mode: FilterMode,
    pub selected_members: HashSet<String>,
    pub selected_vendors: HashSet<String>,
    pub active_chart: AnalyticsChart,
    /// Period Comparison includes income (positive) rows; the
    /// other charts stay spending-only. Set on PC's scope clone only.
    pub include_income: bool,
    /// Period Comparison granularity + selected indices into the
    /// generated period list (see `period_comparison.rs`). Not persisted —
    /// same precedent as legend_sort/budget_view_period; defaults to
    /// Month, A = previous month, B = current month each launch.
    pub comparison_granularity: BudgetViewPeriod,
    pub comparison_period_a: usize,
    pub comparison_period_b: usize,
    /// one-shot flag. When set, every analytics chart resets
    /// its zoom/pan to the default bounds on its next draw, then clears
    /// the flag. Driven by the "Reset View" button so it works across
    /// all charts (the button lives outside the plot closures).
    pub reset_view: bool,
    /// which budget period Budget vs Actual compares against —
    /// granularity tab + index into the 24-entry `comparison_periods`
    /// list (0 = current). Not persisted — same precedent as
    /// comparison_period_a/b; defaults to Month, current period, each
    /// launch.
    pub budget_view_period: BudgetViewPeriod,
    pub budget_period_index: usize,
    /// Category Breakdown legend ordering.
    pub legend_sort: LegendSort,
    /// plot reseed signatures (NOT persisted). egui_plot freezes
    /// auto-bounds on first pan/zoom; when the data behind a chart changes
    /// we call Plot::reset() so bounds auto-fit the new data again. Manual
    /// zoom persists until the next change or the Reset View flag.
    pub bva_sig: u64,
    pub pc_sig: u64,
}

impl Default for AnalyticsState {
    fn default() -> Self {
        let (start, end) = DatePreset::Last3Months.date_range();
        Self {
            date_start: start,
            date_end: end,
            date_preset: DatePreset::Last3Months,
            selected_categories: HashSet::new(),
            category_filter_mode: FilterMode::Include,
            selected_members: HashSet::new(),
            selected_vendors: HashSet::new(),
            active_chart: AnalyticsChart::CategoryBreakdown,
            include_income: false,
            comparison_granularity: BudgetViewPeriod::Month,
            comparison_period_a: 1,
            comparison_period_b: 0,
            reset_view: false,
            budget_view_period: BudgetViewPeriod::Month,
            budget_period_index: 0,
            legend_sort: LegendSort::CategoryAsc,
            bva_sig: 0,
            pc_sig: 0,
        }
    }
}

impl AnalyticsState {
    pub fn to_row(&self) -> AnalyticsFilterRow {
        AnalyticsFilterRow {
            date_preset: date_preset_to_str(self.date_preset).to_string(),
            date_start: self.date_start.map(|d| d.format("%Y-%m-%d").to_string()),
            date_end: self.date_end.map(|d| d.format("%Y-%m-%d").to_string()),
            active_chart: chart_to_str(self.active_chart).to_string(),
            category_filter_mode: filter_mode_to_str(self.category_filter_mode).to_string(),
            selected_categories: string_set_to_json(&self.selected_categories),
            selected_members: string_set_to_json(&self.selected_members),
            selected_vendors: string_set_to_json(&self.selected_vendors),
            // The comparison_mode column stays NOT NULL in the schema but
            // is a legacy constant now — the view-mode UI was removed in
            // favor of the single paired chart.
            comparison_mode: "overlay".to_string(),
            // The old A-preset column stays NOT NULL in the schema but is
            // no longer read — granularity/A/B indices aren't persisted.
            comparison_date_preset: "last_month".to_string(),
        }
    }

    pub fn from_row(row: &AnalyticsFilterRow) -> Self {
        let date_preset = str_to_date_preset(&row.date_preset);

        // rolling presets must be recomputed at load, not
        // restored — a saved "This Month" from Aug 15 reopened in Sep
        // still showing August dates. Saved strings are honored only
        // for Custom, where the user picked the exact bounds.
        let (date_start, date_end) = if date_preset == DatePreset::Custom {
            let parse = |s: &Option<String>| {
                s.as_ref()
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            };
            (parse(&row.date_start), parse(&row.date_end))
        } else {
            date_preset.date_range()
        };

        Self {
            date_start: if date_preset == DatePreset::Custom {
                date_start.or_else(|| chrono::Local::now().date_naive().pred_opt())
            } else {
                date_start
            },
            date_end: if date_preset == DatePreset::Custom {
                date_end.or_else(|| Some(chrono::Local::now().date_naive()))
            } else {
                date_end
            },
            date_preset,
            selected_categories: json_to_string_set(&row.selected_categories),
            category_filter_mode: str_to_filter_mode(&row.category_filter_mode),
            selected_members: json_to_string_set(&row.selected_members),
            selected_vendors: json_to_string_set(&row.selected_vendors),
            active_chart: str_to_chart(&row.active_chart),
            include_income: false,
            comparison_granularity: BudgetViewPeriod::Month,
            comparison_period_a: 1,
            comparison_period_b: 0,
            reset_view: false,
            budget_view_period: BudgetViewPeriod::Month,
            budget_period_index: 0,
            legend_sort: LegendSort::CategoryAsc,
            bva_sig: 0,
            pc_sig: 0,
        }
    }
}

fn date_preset_to_str(p: DatePreset) -> &'static str {
    match p {
        DatePreset::Custom => "custom",
        DatePreset::ThisMonth => "this_month",
        DatePreset::LastMonth => "last_month",
        DatePreset::Last3Months => "last_3_months",
        DatePreset::Last6Months => "last_6_months",
        DatePreset::YTD => "ytd",
        DatePreset::LastYear => "last_year",
        DatePreset::AllTime => "all_time",
    }
}

fn str_to_date_preset(s: &str) -> DatePreset {
    match s {
        "custom" => DatePreset::Custom,
        "this_month" => DatePreset::ThisMonth,
        "last_month" => DatePreset::LastMonth,
        "last_3_months" => DatePreset::Last3Months,
        "last_6_months" => DatePreset::Last6Months,
        "ytd" => DatePreset::YTD,
        "last_year" => DatePreset::LastYear,
        "all_time" => DatePreset::AllTime,
        _ => DatePreset::Last3Months,
    }
}

fn chart_to_str(c: AnalyticsChart) -> &'static str {
    match c {
        AnalyticsChart::CategoryBreakdown => "category_breakdown",
        AnalyticsChart::BudgetVsActual => "budget_vs_actual",
        AnalyticsChart::PeriodComparison => "period_comparison",
    }
}

fn str_to_chart(s: &str) -> AnalyticsChart {
    match s {
        "budget_vs_actual" => AnalyticsChart::BudgetVsActual,
        "period_comparison" => AnalyticsChart::PeriodComparison,
        _ => AnalyticsChart::CategoryBreakdown,
    }
}

fn filter_mode_to_str(f: FilterMode) -> &'static str {
    match f {
        FilterMode::Include => "include",
        FilterMode::Exclude => "exclude",
    }
}

fn str_to_filter_mode(s: &str) -> FilterMode {
    match s {
        "exclude" => FilterMode::Exclude,
        _ => FilterMode::Include,
    }
}

fn string_set_to_json(set: &HashSet<String>) -> String {
    let items: Vec<String> = set
        .iter()
        .map(|s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect();
    format!("[{}]", items.join(","))
}

fn json_to_string_set(json: &str) -> HashSet<String> {
    let trimmed = json.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return HashSet::new();
    }
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() {
        return HashSet::new();
    }

    let mut result = HashSet::new();
    let mut chars = inner.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace and commas
        while let Some(&c) = chars.peek() {
            if c == ' ' || c == ',' {
                chars.next();
            } else {
                break;
            }
        }

        if chars.peek().is_none() {
            break;
        }

        // Parse string
        if chars.peek() == Some(&'"') {
            chars.next(); // consume opening quote
            let mut value = String::new();
            let mut escaped = false;

            while let Some(c) = chars.next() {
                if escaped {
                    value.push(c);
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    break;
                } else {
                    value.push(c);
                }
            }

            if !value.is_empty() {
                result.insert(value);
            }
        }
    }

    result
}
