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
pub enum AnalyticsGranularity {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl AnalyticsGranularity {
    pub fn label(&self) -> &'static str {
        match self {
            AnalyticsGranularity::Daily => "Daily",
            AnalyticsGranularity::Weekly => "Weekly",
            AnalyticsGranularity::Monthly => "Monthly",
            AnalyticsGranularity::Quarterly => "Quarterly",
            AnalyticsGranularity::Yearly => "Yearly",
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
    TimeSeries,
    CategoryBreakdown,
    BudgetVsActual,
    Candlestick,
    PeriodComparison,
}

impl AnalyticsChart {
    pub fn label(&self) -> &'static str {
        match self {
            AnalyticsChart::TimeSeries => "Time Series",
            AnalyticsChart::CategoryBreakdown => "Category Breakdown",
            AnalyticsChart::BudgetVsActual => "Budget vs Actual",
            AnalyticsChart::Candlestick => "Candlestick",
            AnalyticsChart::PeriodComparison => "Period Comparison",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CandlestickPeriod {
    Weekly,
    Monthly,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComparisonMode {
    SideBySide,
    Overlay,
    Delta,
}

impl ComparisonMode {
    pub fn label(&self) -> &'static str {
        match self {
            ComparisonMode::SideBySide => "Side-by-Side",
            ComparisonMode::Overlay => "Overlay",
            ComparisonMode::Delta => "Delta",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnalyticsState {
    pub date_start: Option<NaiveDate>,
    pub date_end: Option<NaiveDate>,
    pub date_preset: DatePreset,
    pub granularity: AnalyticsGranularity,
    pub selected_categories: HashSet<String>,
    pub category_filter_mode: FilterMode,
    pub selected_members: HashSet<String>,
    pub selected_vendors: HashSet<String>,
    pub active_chart: AnalyticsChart,
    pub candlestick_year: i32,
    pub candlestick_period: CandlestickPeriod,
    pub comparison_mode: ComparisonMode,
    pub comparison_period_a_preset: DatePreset,
    pub comparison_period_b_preset: DatePreset,
    /// ponytail: one-shot flag. When set, every analytics chart resets
    /// its zoom/pan to the default bounds on its next draw, then clears
    /// the flag. Driven by the "Reset View" button so it works across
    /// all charts (the button lives outside the plot closures).
    pub reset_view: bool,
}

impl Default for AnalyticsState {
    fn default() -> Self {
        let now = chrono::Local::now().date_naive();
        let (start, end) = DatePreset::Last3Months.date_range();
        Self {
            date_start: start,
            date_end: end,
            date_preset: DatePreset::Last3Months,
            granularity: AnalyticsGranularity::Monthly,
            selected_categories: HashSet::new(),
            category_filter_mode: FilterMode::Include,
            selected_members: HashSet::new(),
            selected_vendors: HashSet::new(),
            active_chart: AnalyticsChart::CategoryBreakdown,
            candlestick_year: now.year(),
            candlestick_period: CandlestickPeriod::Monthly,
            comparison_mode: ComparisonMode::Overlay,
            comparison_period_a_preset: DatePreset::LastMonth,
            comparison_period_b_preset: DatePreset::ThisMonth,
            reset_view: false,
        }
    }
}

impl AnalyticsState {
    pub fn to_row(&self) -> AnalyticsFilterRow {
        AnalyticsFilterRow {
            date_preset: date_preset_to_str(self.date_preset).to_string(),
            date_start: self.date_start.map(|d| d.format("%Y-%m-%d").to_string()),
            date_end: self.date_end.map(|d| d.format("%Y-%m-%d").to_string()),
            granularity: granularity_to_str(self.granularity).to_string(),
            active_chart: chart_to_str(self.active_chart).to_string(),
            category_filter_mode: filter_mode_to_str(self.category_filter_mode).to_string(),
            selected_categories: string_set_to_json(&self.selected_categories),
            selected_members: string_set_to_json(&self.selected_members),
            selected_vendors: string_set_to_json(&self.selected_vendors),
            candlestick_year: self.candlestick_year,
            candlestick_period: candlestick_period_to_str(self.candlestick_period).to_string(),
            comparison_mode: comparison_mode_to_str(self.comparison_mode).to_string(),
            comparison_date_preset: date_preset_to_str(self.comparison_period_a_preset).to_string(),
        }
    }

    pub fn from_row(row: &AnalyticsFilterRow) -> Self {
        let date_preset = str_to_date_preset(&row.date_preset);
        let (default_start, default_end) = date_preset.date_range();
        
        let date_start = row.date_start.as_ref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .or(default_start);
        let date_end = row.date_end.as_ref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .or(default_end);
        
        Self {
            date_start,
            date_end,
            date_preset,
            granularity: str_to_granularity(&row.granularity),
            selected_categories: json_to_string_set(&row.selected_categories),
            category_filter_mode: str_to_filter_mode(&row.category_filter_mode),
            selected_members: json_to_string_set(&row.selected_members),
            selected_vendors: json_to_string_set(&row.selected_vendors),
            active_chart: str_to_chart(&row.active_chart),
            candlestick_year: row.candlestick_year,
            candlestick_period: str_to_candlestick_period(&row.candlestick_period),
            comparison_mode: str_to_comparison_mode(&row.comparison_mode),
            comparison_period_a_preset: str_to_date_preset(&row.comparison_date_preset),
            comparison_period_b_preset: DatePreset::ThisMonth,
            reset_view: false,
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

fn granularity_to_str(g: AnalyticsGranularity) -> &'static str {
    match g {
        AnalyticsGranularity::Daily => "daily",
        AnalyticsGranularity::Weekly => "weekly",
        AnalyticsGranularity::Monthly => "monthly",
        AnalyticsGranularity::Quarterly => "quarterly",
        AnalyticsGranularity::Yearly => "yearly",
    }
}

fn str_to_granularity(s: &str) -> AnalyticsGranularity {
    match s {
        "daily" => AnalyticsGranularity::Daily,
        "weekly" => AnalyticsGranularity::Weekly,
        "monthly" => AnalyticsGranularity::Monthly,
        "quarterly" => AnalyticsGranularity::Quarterly,
        "yearly" => AnalyticsGranularity::Yearly,
        _ => AnalyticsGranularity::Monthly,
    }
}

fn chart_to_str(c: AnalyticsChart) -> &'static str {
    match c {
        AnalyticsChart::TimeSeries => "time_series",
        AnalyticsChart::CategoryBreakdown => "category_breakdown",
        AnalyticsChart::BudgetVsActual => "budget_vs_actual",
        AnalyticsChart::Candlestick => "candlestick",
        AnalyticsChart::PeriodComparison => "period_comparison",
    }
}

fn str_to_chart(s: &str) -> AnalyticsChart {
    match s {
        "time_series" => AnalyticsChart::TimeSeries,
        "category_breakdown" => AnalyticsChart::CategoryBreakdown,
        "budget_vs_actual" => AnalyticsChart::BudgetVsActual,
        "candlestick" => AnalyticsChart::Candlestick,
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

fn candlestick_period_to_str(p: CandlestickPeriod) -> &'static str {
    match p {
        CandlestickPeriod::Weekly => "weekly",
        CandlestickPeriod::Monthly => "monthly",
    }
}

fn str_to_candlestick_period(s: &str) -> CandlestickPeriod {
    match s {
        "weekly" => CandlestickPeriod::Weekly,
        _ => CandlestickPeriod::Monthly,
    }
}

fn comparison_mode_to_str(m: ComparisonMode) -> &'static str {
    match m {
        ComparisonMode::SideBySide => "side_by_side",
        ComparisonMode::Overlay => "overlay",
        ComparisonMode::Delta => "delta",
    }
}

fn str_to_comparison_mode(s: &str) -> ComparisonMode {
    match s {
        "side_by_side" => ComparisonMode::SideBySide,
        "overlay" => ComparisonMode::Overlay,
        "delta" => ComparisonMode::Delta,
        _ => ComparisonMode::Overlay,
    }
}

fn string_set_to_json(set: &HashSet<String>) -> String {
    let items: Vec<String> = set.iter()
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
