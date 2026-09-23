use eframe::egui::{self, RichText};

use super::aggregation::truncate_label;
use super::state::*;
use crate::models::{category_parent_map, Category};

/// One category row in the shared picker: swatch + clickable name + an
/// optional right-aligned value (amount, variance, difference…).
pub struct PickerRow {
    pub label: String,
    pub color: eframe::egui::Color32,
    pub value: Option<String>,
    pub value_color: Option<eframe::egui::Color32>,
}

pub fn toggle_category_filter(state: &mut AnalyticsState, category: &str) {
    if state.selected_categories.contains(category) {
        state.selected_categories.remove(category);
    } else {
        state.selected_categories.insert(category.to_string());
    }
}

/// THE Category/Cost sort row — shared by all three analytics
/// tabs so one selection orders every chart (and follows you when you
/// switch tabs). Two buttons, each click toggles its direction.
pub fn render_legend_sort(ui: &mut egui::Ui, state: &mut AnalyticsState) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        crate::ui::components::label_muted(ui, "Sort:");
        let cat_active = matches!(
            state.legend_sort,
            LegendSort::CategoryAsc | LegendSort::CategoryDesc
        );
        let cost_active = matches!(
            state.legend_sort,
            LegendSort::CostAsc | LegendSort::CostDesc
        );
        let cat_label = match state.legend_sort {
            LegendSort::CategoryAsc => "Category ↑",
            LegendSort::CategoryDesc => "Category ↓",
            _ => "Category",
        };
        let cost_label = match state.legend_sort {
            LegendSort::CostAsc => "Cost ↑",
            LegendSort::CostDesc => "Cost ↓",
            _ => "Cost",
        };
        if crate::ui::components::tab_label_button(ui, cat_active, cat_label).clicked() {
            state.legend_sort = if state.legend_sort == LegendSort::CategoryAsc {
                LegendSort::CategoryDesc
            } else {
                LegendSort::CategoryAsc
            };
            changed = true;
        }
        if crate::ui::components::tab_label_button(ui, cost_active, cost_label).clicked() {
            state.legend_sort = if state.legend_sort == LegendSort::CostAsc {
                LegendSort::CostDesc
            } else {
                LegendSort::CostAsc
            };
            changed = true;
        }
    });
    changed
}

/// shared row ordering for the analytics category lists (chart rows and
/// picker rows both ride this). Category = the parent→sub tree walk
/// (orphans appended by cost desc, whole vec reversed on Desc — the
/// original breakdown behavior); Cost = |cost| descending/ascending with
/// a name tie-break. Costs are magnitudes, otherwise the breakdown's
/// signed-negative amounts would float the smallest spends to the top on
/// Cost ↓.
pub fn order_rows_by_legend_sort<T>(
    categories: &[Category],
    rows: &mut Vec<T>,
    sort: LegendSort,
    label_of: impl Fn(&T) -> &str,
    cost_of: impl Fn(&T) -> f64,
) {
    match sort {
        LegendSort::CategoryAsc | LegendSort::CategoryDesc => {
            let parents = category_parent_map(categories);
            let mut used = vec![false; rows.len()];
            let mut order: Vec<usize> = Vec::with_capacity(rows.len());
            // Tree walk: rows present in `categories` come first, in the
            // categories list's own parent→sub order.
            for cat in categories {
                let label = cat.full_label(&parents);
                if let Some(i) = rows
                    .iter()
                    .enumerate()
                    .find(|(i, r)| !used[*i] && label_of(r) == label)
                    .map(|(i, _)| i)
                {
                    used[i] = true;
                    order.push(i);
                }
            }
            // Orphans (categories deleted after spend) append by cost desc.
            let mut orphans: Vec<usize> = used
                .iter()
                .enumerate()
                .filter(|(_, &u)| !u)
                .map(|(i, _)| i)
                .collect();
            orphans.sort_by(|&a, &b| {
                cost_of(&rows[b])
                    .abs()
                    .partial_cmp(&cost_of(&rows[a]).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| label_of(&rows[a]).cmp(label_of(&rows[b])))
            });
            order.extend(orphans);
            if sort == LegendSort::CategoryDesc {
                order.reverse();
            }
            // Rebuild via Option slots — rows is borrowed, can't move out.
            let mut slots: Vec<Option<T>> = std::mem::take(rows).into_iter().map(Some).collect();
            *rows = order
                .into_iter()
                .map(|i| slots[i].take().unwrap())
                .collect();
        }
        LegendSort::CostAsc | LegendSort::CostDesc => {
            let desc = sort == LegendSort::CostDesc;
            rows.sort_by(|a, b| {
                let ord = cost_of(a)
                    .abs()
                    .partial_cmp(&cost_of(b).abs())
                    .unwrap_or(std::cmp::Ordering::Equal);
                let ord = if desc { ord.reverse() } else { ord };
                ord.then_with(|| label_of(a).cmp(label_of(b)))
            });
        }
    }
}

/// the one category selector for every analytics tab, rendered
/// in a right-hand side column (Category Breakdown next to the donut,
/// Budget vs Actual under its summary card, Period Comparison next to the
/// chart). The caller passes the column's fill height; rows are
/// pre-ordered by the caller; returns true if anything toggled.
pub fn render_category_picker(
    ui: &mut egui::Ui,
    state: &mut AnalyticsState,
    rows: &[PickerRow],
    max_height: f32,
) -> bool {
    let mut changed = false;
    egui::ScrollArea::vertical()
        .id_salt("category_picker_scroll")
        .max_height(max_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for row in rows {
                let is_filtered = state.selected_categories.contains(&row.label);
                let is_include_mode = state.category_filter_mode == FilterMode::Include;
                // In include mode, active = in the filter set; in exclude
                // mode, active = NOT in the set.
                let is_active = if is_include_mode {
                    is_filtered
                } else {
                    !is_filtered
                };

                ui.horizontal(|ui| {
                    let (swatch_rect, swatch_response) =
                        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
                    let swatch_color = if is_active {
                        row.color
                    } else {
                        row.color.gamma_multiply(0.3)
                    };
                    ui.painter().rect_filled(swatch_rect, 2.0, swatch_color);
                    if swatch_response.clicked() {
                        changed = true;
                        toggle_category_filter(state, &row.label);
                    }

                    ui.add_space(4.0);

                    // Truncated so long names can't overlap the right value.
                    let display_name = truncate_label(&row.label, 24);
                    let name_response =
                        ui.add(egui::Label::new(&display_name).sense(egui::Sense::click()));
                    let name_clicked = name_response.clicked();
                    if row.label.chars().count() > 24 {
                        name_response.on_hover_text(&row.label);
                    }
                    if name_clicked {
                        changed = true;
                        toggle_category_filter(state, &row.label);
                    }

                    if let Some(value) = &row.value {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            match row.value_color {
                                Some(color) => {
                                    ui.label(RichText::new(value).color(color));
                                }
                                None => {
                                    ui.label(value);
                                }
                            }
                        });
                    }
                });
                ui.add_space(4.0);
            }
        });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Color32;

    fn cat(id: i64, name: &str) -> Category {
        Category {
            id,
            name: name.to_string(),
            parent_id: None,
            color: Color32::GRAY,
            excluded: false,
        }
    }

    struct Row {
        label: String,
        cost: f64,
    }

    fn rows() -> Vec<Row> {
        vec![
            Row {
                label: "Fun".into(),
                cost: 5.0,
            },
            Row {
                label: "Rent".into(),
                cost: 50.0,
            },
            Row {
                label: "Food".into(),
                cost: 20.0,
            },
            Row {
                label: "Orphan".into(),
                cost: 7.0,
            },
        ]
    }

    fn labels(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.label.as_str()).collect()
    }

    #[test]
    fn legend_sort_orders_tree_and_cost() {
        // categories list order = tree order; "Orphan" is in no list.
        let cats = vec![cat(1, "Food"), cat(2, "Rent"), cat(3, "Fun")];

        let mut r = rows();
        order_rows_by_legend_sort(
            &cats,
            &mut r,
            LegendSort::CategoryAsc,
            |x| &x.label,
            |x| x.cost,
        );
        assert_eq!(labels(&r), vec!["Food", "Rent", "Fun", "Orphan"]);

        let mut r = rows();
        order_rows_by_legend_sort(
            &cats,
            &mut r,
            LegendSort::CategoryDesc,
            |x| &x.label,
            |x| x.cost,
        );
        assert_eq!(labels(&r), vec!["Orphan", "Fun", "Rent", "Food"]);

        // Cost ↓ = biggest magnitude first (the old signed sort put the
        // smallest spends on top).
        let mut r = rows();
        order_rows_by_legend_sort(
            &cats,
            &mut r,
            LegendSort::CostDesc,
            |x| &x.label,
            |x| x.cost,
        );
        assert_eq!(labels(&r), vec!["Rent", "Food", "Orphan", "Fun"]);

        let mut r = rows();
        order_rows_by_legend_sort(&cats, &mut r, LegendSort::CostAsc, |x| &x.label, |x| x.cost);
        assert_eq!(labels(&r), vec!["Fun", "Orphan", "Food", "Rent"]);

        // Ties fall back to name.
        let mut r = vec![
            Row {
                label: "B".into(),
                cost: 10.0,
            },
            Row {
                label: "A".into(),
                cost: 10.0,
            },
        ];
        order_rows_by_legend_sort(&[], &mut r, LegendSort::CostDesc, |x| &x.label, |x| x.cost);
        assert_eq!(labels(&r), vec!["A", "B"]);
    }
}
