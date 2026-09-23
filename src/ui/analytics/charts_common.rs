use chrono::{Datelike, NaiveDate};
use egui_plot::{GridInput, GridMark};

use super::state::BudgetViewPeriod;

/// shared helpers for the horizontal analytics charts
/// (Budget vs Actual, Period Comparison). Both plot ABS magnitudes on a
/// log-ish value axis: bars feed `ln1p(v)` (0 → 0, monotonic, baseline
/// intact — egui_plot has no native log transform) and the axis formatters
/// map back with `exp_m1` for $ labels.

/// One selectable analysis period: label + date range + the (year,
/// period) ints that budgets are priced with.
#[derive(Clone)]
pub struct ChartPeriod {
    pub label: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub year: i32,
    pub period: i32,
}

/// 24 same-granularity periods ending with the current one —
/// "September 2026", "Q3 2026", "2026", "W39 2026" — so A and B are always
/// comparable siblings instead of rolling DatePresets (a month could
/// previously be compared against a 3-month window). Ranges come from the
/// shared `models::budget_period_date_range`. Index 0 = current period.
/// Not persisted — same precedent as legend_sort/budget_view_period.
pub fn comparison_periods(gran: BudgetViewPeriod) -> Vec<ChartPeriod> {
    let now = chrono::Local::now().date_naive();
    let bgran = match gran {
        BudgetViewPeriod::Week => crate::models::BudgetGranularity::Weekly,
        BudgetViewPeriod::Month => crate::models::BudgetGranularity::Monthly,
        BudgetViewPeriod::Quarter => crate::models::BudgetGranularity::Quarterly,
        BudgetViewPeriod::Year => crate::models::BudgetGranularity::Yearly,
    };
    let (mut year, mut period) = match gran {
        BudgetViewPeriod::Week => {
            let iso = now.iso_week();
            (iso.year(), iso.week() as i32)
        }
        BudgetViewPeriod::Month => (now.year(), now.month() as i32),
        BudgetViewPeriod::Quarter => (now.year(), ((now.month() - 1) / 3 + 1) as i32),
        BudgetViewPeriod::Year => (now.year(), 0),
    };
    let mut out = Vec::with_capacity(24);
    for _ in 0..24 {
        let (start, end) = crate::models::budget_period_date_range(bgran, year, period);
        let label = match gran {
            BudgetViewPeriod::Week => format!("W{} {}", period, year),
            BudgetViewPeriod::Month => format!(
                "{} {}",
                chrono::Month::try_from(period as u8)
                    .map(|m| m.name())
                    .unwrap_or(""),
                year
            ),
            BudgetViewPeriod::Quarter => format!("Q{} {}", period, year),
            BudgetViewPeriod::Year => year.to_string(),
        };
        out.push(ChartPeriod {
            label,
            start,
            end,
            year,
            period,
        });
        match gran {
            BudgetViewPeriod::Week => {
                if period <= 1 {
                    year -= 1;
                    period = NaiveDate::from_ymd_opt(year, 12, 28)
                        .unwrap()
                        .iso_week()
                        .week() as i32;
                } else {
                    period -= 1;
                }
            }
            BudgetViewPeriod::Month => {
                if period == 1 {
                    period = 12;
                    year -= 1;
                } else {
                    period -= 1;
                }
            }
            BudgetViewPeriod::Quarter => {
                if period == 1 {
                    period = 4;
                    year -= 1;
                } else {
                    period -= 1;
                }
            }
            BudgetViewPeriod::Year => year -= 1,
        }
    }
    out
}

/// "Sep 01 – Sep 30" style combo text for one [`ChartPeriod`].
pub fn period_text(label: &str, start: NaiveDate, end: NaiveDate) -> String {
    format!(
        "{} ({} – {})",
        label,
        start.format("%b %d"),
        end.format("%b %d")
    )
}

/// Transform a raw dollar value into the ln-space the bars are fed.
pub fn ln1p(v: f64) -> f64 {
    v.max(0.0).ln_1p()
}

/// Compact $ label for a RAW dollar value (formatter side of [`ln1p`]).
pub fn money_label(v: f64) -> String {
    if v <= 0.0 {
        "$0".to_string()
    } else if v >= 1_000_000.0 {
        format!("${:.1}M", v / 1_000_000.0)
    } else if v >= 1000.0 {
        format!("${:.0}k", v / 1000.0)
    } else if v >= 1.0 {
        format!("${:.0}", v)
    } else {
        format!("${:.2}", v)
    }
}

/// 1-2-5 dollar ticks inside the visible ln-space range — feed to
/// `Plot::x_grid_spacer`. The default decimal spacer would place linear
/// ticks in ln space (unlabeled nonsense after the transform).
pub fn money_grid_spacer(input: GridInput) -> Vec<GridMark> {
    let (a, b) = input.bounds;
    let (lo, hi) = (a.min(b), a.max(b));
    let (rlo, rhi) = (lo.exp_m1(), hi.exp_m1());

    let mut raw: Vec<f64> = Vec::new();
    if lo <= 0.0 && hi >= 0.0 {
        raw.push(0.0); // $0 sits at ln1p(0) = 0
    }
    for exp in -2i32..=7 {
        let base = 10f64.powi(exp);
        for m in [1.0, 2.0, 5.0] {
            let v = m * base;
            if v >= rlo && v <= rhi {
                raw.push(v);
            }
        }
    }
    raw.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));

    let xs: Vec<f64> = raw.iter().map(|v| v.ln_1p()).collect();
    xs.iter()
        .enumerate()
        .map(|(i, &x)| {
            // step_size = gap to the neighbouring mark (grid-line thickness
            // tier); first mark borrows the gap to the next.
            let step = if i + 1 < xs.len() {
                xs[i + 1] - x
            } else if i > 0 {
                x - xs[i - 1]
            } else {
                0.1
            };
            GridMark {
                value: x,
                step_size: step.max(f64::EPSILON),
            }
        })
        .collect()
}

/// Integer ticks for the category axis — feed to `Plot::y_grid_spacer`.
/// The default decimal spacer skipped rows at coarse zooms; every whole
/// category row gets a tick + label.
pub fn category_grid_spacer(input: GridInput) -> Vec<GridMark> {
    let (a, b) = input.bounds;
    let (lo, hi) = (a.min(b), a.max(b));
    let mut out = Vec::new();
    let mut i = lo.floor();
    while i <= hi.ceil() {
        if i >= lo && i <= hi {
            out.push(GridMark {
                value: i,
                step_size: 1.0,
            });
        }
        i += 1.0;
    }
    out
}

/// one-shot reseed decision for `Plot::reset()`. egui_plot
/// freezes auto-bounds on first pan/zoom (and `set_plot_bounds` never
/// re-enables them) — dropping PlotMemory re-fits the new data; manual
/// zoom persists until the data/filters change or Reset View fires.
pub fn take_reseed(prev: &mut u64, new_sig: u64, reset_view: bool) -> bool {
    let changed = *prev != new_sig;
    *prev = new_sig;
    changed || reset_view
}

/// x/y-axis label showing only the subcategory name (after the
/// " › " separator) so dense category lists don't overflow the axis. The
/// full "Parent › Sub" name still shows in tooltips and the picker.
pub fn subcategory_label(full: &str) -> String {
    match full.rfind(crate::models::CATEGORY_LABEL_SEP) {
        Some(i) => full[i + crate::models::CATEGORY_LABEL_SEP.len()..].to_string(),
        None => full.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_spacer_ticks_inside_ln_bounds() {
        // $50..$500 visible → ln-space bounds.
        let (lo, hi) = (ln1p(50.0), ln1p(500.0));
        let marks = money_grid_spacer(GridInput {
            bounds: (lo, hi),
            base_step_size: 0.1,
        });
        assert!(!marks.is_empty());
        for m in &marks {
            assert!(m.value >= lo && m.value <= hi, "mark outside bounds");
            assert!(m.step_size > 0.0);
            let raw = m.value.exp_m1();
            // every mark must land on a 1-2-5 dollar value (or $0)
            let ok = (0.0..=0.001).contains(&raw)
                || [1.0, 2.0, 5.0].iter().any(|mant| {
                    let dec = raw.log10().floor();
                    (raw / 10f64.powf(dec) - mant).abs() < 1e-6
                });
            assert!(ok, "mark {raw} not a 1-2-5 value");
        }
    }

    #[test]
    fn take_reseed_only_fires_on_change_or_reset() {
        let mut sig = 0;
        assert!(take_reseed(&mut sig, 7, false)); // first signature
        assert!(!take_reseed(&mut sig, 7, false)); // unchanged
        assert!(take_reseed(&mut sig, 7, true)); // reset forces reseed
        assert!(take_reseed(&mut sig, 8, false)); // data changed
    }
}
