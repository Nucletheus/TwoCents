use crate::db::*;
use crate::models::*;
use chrono::Datelike;
use eframe::egui::{self, Color32, RichText, Stroke};
fn clip_text(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    p: egui::Pos2,
    a: egui::Align2,
    t: &str,
    f: egui::FontId,
    c: Color32,
) {
    let old = ui.clip_rect();
    ui.set_clip_rect(cell.intersect(old));
    ui.painter().text(p, a, t, f, c);
    ui.set_clip_rect(old);
}
use crate::TwoCentsApp;

fn week_range(year: i32, week: u32) -> (chrono::NaiveDate, chrono::NaiveDate) {
    budget_period_date_range(BudgetGranularity::Weekly, year, week as i32)
}

const MAX_BUDGET_CENTS: i64 = i64::MAX / 1_000_000;

fn parse_budget_cents(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    let value = trimmed.strip_prefix('$').unwrap_or(trimmed);
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));

    if whole.contains(',') {
        let mut groups = whole.split(',');
        let first = groups.next().unwrap_or_default();
        if first.is_empty()
            || first.len() > 3
            || !first.bytes().all(|byte| byte.is_ascii_digit())
            || groups
                .any(|group| group.len() != 3 || !group.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return None;
        }
    }
    let whole_digits = whole.replace(',', "");
    if whole_digits.is_empty() || !whole_digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if fraction.len() > 2
        || (value.contains('.') && fraction.is_empty())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }

    let dollars = whole_digits.parse::<i64>().ok()?;
    let cents = match fraction.len() {
        0 => 0,
        1 => i64::from(fraction.as_bytes()[0] - b'0') * 10,
        _ => fraction.parse::<i64>().ok()?,
    };
    let value = dollars.checked_mul(100)?.checked_add(cents)?;
    (value <= MAX_BUDGET_CENTS).then_some(value)
}

fn initialize_budget_text_edit(
    ui: &egui::Ui,
    output: &mut egui::widgets::text_edit::TextEditOutput,
    value_char_count: usize,
    text_edit_id: egui::Id,
    init_id: egui::Id,
) {
    if ui
        .ctx()
        .data(|data| data.get_temp::<bool>(init_id).unwrap_or(false))
    {
        return;
    }

    output.response.request_focus();
    let mut state =
        egui::widgets::text_edit::TextEditState::load(ui.ctx(), text_edit_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::default(),
            egui::text::CCursor::new(value_char_count),
        )));
    state.store(ui.ctx(), text_edit_id);
    ui.ctx()
        .data_mut(|data| data.insert_temp::<bool>(init_id, true));
    ui.ctx().request_repaint();
}

impl TwoCentsApp {
    pub fn ui_budgets(&mut self, ui: &mut egui::Ui) {
        crate::ui::components::heading_lg(ui, "Budgets");
        ui.add_space(8.0);

        // 1. Timeframe / Granularity Toggles at the Top
        let granularities = [
            (BudgetGranularity::Weekly, "Weekly"),
            (BudgetGranularity::Monthly, "Monthly"),
            (BudgetGranularity::Quarterly, "Quarterly"),
            (BudgetGranularity::Yearly, "Yearly"),
        ];
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Timeframe:"));
            for (gran, label) in granularities {
                let selected = self.budget_granularity == gran;
                let sel = crate::ui::theme::sel_text(ui, selected, label);
                if ui.selectable_label(selected, sel).clicked() {
                    self.budget_granularity = gran;
                    self.budget_editing_category = None;
                    changed = true;
                }
            }
        });
        if changed {
            self.reload();
        }
        let desc = match self.budget_granularity {
            BudgetGranularity::Weekly => "Tracking weekly cash flow envelopes.",
            BudgetGranularity::Monthly => "Standard monthly envelope budget.",
            BudgetGranularity::Quarterly => "Quarterly high-level tracking.",
            BudgetGranularity::Yearly => "Annual fiscal allocations.",
        };
        ui.label(RichText::new(desc).weak().size(12.0));

        ui.separator();
        ui.add_space(6.0);

        // 2. Dynamic Period Selector Control Bar
        ui.horizontal_wrapped(|ui| {
            ui.style_mut().spacing.button_padding = egui::vec2(10.0, 6.0);

            let prev_clicked = crate::ui::popups::styled_button(ui, "‹", false).clicked();

            // Build title text for the fixed-width area
            let title_text = match self.budget_granularity {
                BudgetGranularity::Weekly => {
                    let (start, end) = week_range(self.budget_year, self.budget_week);
                    let start_str = start.format("%b %d").to_string();
                    let end_str = end.format("%b %d").to_string();
                    format!("Week {} ({} - {})", self.budget_week, start_str, end_str)
                }
                BudgetGranularity::Monthly => {
                    let month_names = [
                        "January",
                        "February",
                        "March",
                        "April",
                        "May",
                        "June",
                        "July",
                        "August",
                        "September",
                        "October",
                        "November",
                        "December",
                    ];
                    let month_name = month_names[(self.budget_month as usize - 1).min(11)];
                    format!("{} {}", month_name, self.budget_year)
                }
                BudgetGranularity::Quarterly => {
                    format!("Q{} {}", self.budget_quarter, self.budget_year)
                }
                BudgetGranularity::Yearly => {
                    format!("{}", self.budget_year)
                }
            };
            // Fixed-width title area so arrow buttons stay in the same place
            let title_w = 195.0;
            let (title_rect, _) = ui.allocate_exact_size(
                egui::vec2(title_w, ui.style().spacing.interact_size.y),
                egui::Sense::hover(),
            );
            ui.painter().text(
                egui::pos2(title_rect.left() + title_w / 2.0, title_rect.center().y),
                egui::Align2::CENTER_CENTER,
                &title_text,
                egui::FontId::proportional(16.0),
                crate::ui::theme::fg_primary(),
            );

            let next_clicked = crate::ui::popups::styled_button(ui, "›", false).clicked();
            let snap_clicked = crate::ui::popups::styled_button(ui, "⟲", false)
                .on_hover_text("Snap to current period")
                .clicked();

            if prev_clicked {
                match self.budget_granularity {
                    BudgetGranularity::Weekly => {
                        if self.budget_week <= 1 {
                            self.budget_year -= 1;
                            self.budget_week =
                                budget_period_count(BudgetGranularity::Weekly, self.budget_year)
                                    as u32;
                        } else {
                            self.budget_week -= 1;
                        }
                    }
                    BudgetGranularity::Monthly => {
                        if self.budget_month == 1 {
                            self.budget_month = 12;
                            self.budget_year -= 1;
                        } else {
                            self.budget_month -= 1;
                        }
                    }
                    BudgetGranularity::Quarterly => {
                        if self.budget_quarter == 1 {
                            self.budget_quarter = 4;
                            self.budget_year -= 1;
                        } else {
                            self.budget_quarter -= 1;
                        }
                    }
                    BudgetGranularity::Yearly => {
                        self.budget_year -= 1;
                    }
                }
                self.budget_editing_category = None;
                self.reload();
            }

            if next_clicked {
                match self.budget_granularity {
                    BudgetGranularity::Weekly => {
                        let count =
                            budget_period_count(BudgetGranularity::Weekly, self.budget_year) as u32;
                        if self.budget_week >= count {
                            self.budget_week = 1;
                            self.budget_year += 1;
                        } else {
                            self.budget_week += 1;
                        }
                    }
                    BudgetGranularity::Monthly => {
                        if self.budget_month == 12 {
                            self.budget_month = 1;
                            self.budget_year += 1;
                        } else {
                            self.budget_month += 1;
                        }
                    }
                    BudgetGranularity::Quarterly => {
                        if self.budget_quarter == 4 {
                            self.budget_quarter = 1;
                            self.budget_year += 1;
                        } else {
                            self.budget_quarter += 1;
                        }
                    }
                    BudgetGranularity::Yearly => {
                        self.budget_year += 1;
                    }
                }
                self.budget_editing_category = None;
                self.reload();
            }

            if snap_clicked {
                let now = chrono::Local::now();
                self.budget_year = now.year();
                self.budget_month = now.month() as i32;
                self.budget_quarter = ((now.month() - 1) / 3 + 1) as u32;
                self.budget_week = budget_period_for_date(
                    BudgetGranularity::Weekly,
                    self.budget_year,
                    now.date_naive(),
                )
                .unwrap_or(1) as u32;
                self.budget_editing_category = None;
                self.reload();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let source_year = self.budget_year - 1;
                let copy_label = format!("Copy Limits from {source_year}");
                if crate::ui::popups::styled_button(ui, &copy_label, false)
                    .on_hover_text("Copy the complete previous-year budget plan to this year.")
                    .clicked()
                {
                    match self.copy_budget_year() {
                        Ok(0) => {
                            self.set_transient_status(format!(
                                "No budget limits found in {source_year}."
                            ));
                        }
                        Ok(count) => {
                            self.set_transient_status(format!(
                                "Copied {count} budget limits from {source_year}."
                            ));
                        }
                        Err(error) => self.set_transient_status(error),
                    }
                }
            });
        });

        ui.add_space(10.0);

        // 3. Compute Actual Spent per category dynamically based on active timeframe
        // spending = debit rows only, magnitude — income (positive) and
        // excluded transfer/payment categories never enter budget math.
        let excluded_categories = excluded_category_labels(&self.categories);
        let assignable_categories = category_assignable_labels(&self.categories);
        let mut spent_map = std::collections::HashMap::new();

        let accumulate =
            |exp: &crate::models::Expense,
             spent_map: &mut std::collections::HashMap<String, i64>| {
                if exp.amount_cents >= 0 || excluded_categories.contains(&exp.category) {
                    return;
                }
                *spent_map.entry(exp.category.clone()).or_insert(0) += -exp.amount_cents;
            };

        match self.budget_granularity {
            BudgetGranularity::Weekly => {
                let (start_date, end_date) = week_range(self.budget_year, self.budget_week);
                for exp in &self.expenses {
                    if let Ok(exp_date) = chrono::NaiveDate::parse_from_str(&exp.date, "%Y-%m-%d") {
                        if exp_date >= start_date && exp_date <= end_date {
                            accumulate(exp, &mut spent_map);
                        }
                    }
                }
            }
            BudgetGranularity::Monthly => {
                let prefix = format!("{}-{:02}", self.budget_year, self.budget_month);
                for exp in &self.expenses {
                    if exp.date.starts_with(&prefix) {
                        accumulate(exp, &mut spent_map);
                    }
                }
            }
            BudgetGranularity::Quarterly => {
                let prefix1 = format!(
                    "{}-{:02}",
                    self.budget_year,
                    (self.budget_quarter - 1) * 3 + 1
                );
                let prefix2 = format!(
                    "{}-{:02}",
                    self.budget_year,
                    (self.budget_quarter - 1) * 3 + 2
                );
                let prefix3 = format!(
                    "{}-{:02}",
                    self.budget_year,
                    (self.budget_quarter - 1) * 3 + 3
                );
                for exp in &self.expenses {
                    if exp.date.starts_with(&prefix1)
                        || exp.date.starts_with(&prefix2)
                        || exp.date.starts_with(&prefix3)
                    {
                        accumulate(exp, &mut spent_map);
                    }
                }
            }
            BudgetGranularity::Yearly => {
                let prefix = format!("{}-", self.budget_year);
                for exp in &self.expenses {
                    if exp.date.starts_with(&prefix) {
                        accumulate(exp, &mut spent_map);
                    }
                }
            }
        }

        let mut total_budgeted = 0;
        let mut total_spent = 0;
        for cat in &assignable_categories {
            let limit = self.budgets.get(cat).copied().unwrap_or(0);
            let spent = spent_map.get(cat).copied().unwrap_or(0);
            total_budgeted += limit;
            total_spent += spent;
        }
        let remaining_overall = total_budgeted - total_spent;

        // extracted summary-card helper. Three near-identical Frame::default
        // blocks reduced to one helper called three times. Caller passes the
        // themed label + amount + optional color override for the amount.
        // Success/error overrides route through components::success_color/error_color
        // so the "remaining overall" green/red matches the rest of the app.
        let total_w = ui.available_width();
        let card_content_w = ((total_w - 72.0) / 3.0).max(20.0); // 72 = 3 frames × 24px inner margin
        let render_card =
            |ui: &mut egui::Ui, label: &str, amount: i64, color: Option<egui::Color32>| {
                crate::ui::components::card(ui).show(ui, |ui| {
                    ui.set_width(card_content_w);
                    ui.vertical(|ui| {
                        crate::ui::components::label_faint(ui, label);
                        ui.add_space(4.0);
                        let amount_text = money(amount);
                        let rt = match color {
                            Some(c) => egui::RichText::new(amount_text)
                                .size(20.0)
                                .strong()
                                .color(c),
                            None => egui::RichText::new(amount_text)
                                .size(20.0)
                                .strong()
                                .color(crate::ui::theme::fg_primary()),
                        };
                        ui.heading(rt);
                    });
                });
            };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            render_card(ui, "TOTAL ALLOCATED", total_budgeted, None);
            render_card(ui, "TOTAL SPENT", total_spent, None);
            let color = if remaining_overall >= 0 {
                crate::ui::theme::success()
            } else {
                crate::ui::theme::error()
            };
            render_card(ui, "REMAINING OVERALL", remaining_overall, Some(color));
        });

        ui.add_space(14.0);

        // Pill Selector Filters
        ui.horizontal_wrapped(|ui| {
            ui.label("Filter:");

            let filter_options = [
                (BudgetFilter::All, "All"),
                (BudgetFilter::OverBudget, "Over Budget"),
                (BudgetFilter::UnderBudget, "Under Budget"),
                (BudgetFilter::Unbudgeted, "Unbudgeted"),
            ];

            for (filter, label) in filter_options {
                let selected = self.budget_filter == filter;
                let button =
                    ui.selectable_label(selected, crate::ui::theme::sel_text(ui, selected, label));
                if button.clicked() {
                    self.budget_filter = filter;
                    self.budget_editing_category = None;
                }
            }
        });

        ui.add_space(10.0);

        // Build hierarchical category tree
        let cat_parents = category_parent_map(&self.categories);
        let parent_ids = sorted_parent_category_ids(&self.categories);
        let mut has_children = std::collections::HashSet::new();
        for cat in &self.categories {
            if let Some(pid) = cat.parent_id {
                has_children.insert(pid);
            }
        }

        // Collect all leaf child categories (those with a parent_id) and childless parents
        // Each item: (is_child, parent_id_or_self, display_name, full_label_for_lookup, color)
        struct CatEntry {
            is_child: bool,
            parent_name: String,
            full_label: String,
            display_name: String,
            color: Color32,
        }

        let mut entries: Vec<CatEntry> = Vec::new();
        // excluded categories (user-flagged) are skipped from the
        // budget list entirely — spent stays 0 via the aggregation filter too.
        let excluded = excluded_category_labels(&self.categories);
        for &pid in &parent_ids {
            let Some(parent) = self.categories.iter().find(|c| c.id == pid) else {
                continue;
            };
            if excluded.contains(&parent.name) {
                continue;
            }
            if !has_children.contains(&pid) {
                entries.push(CatEntry {
                    is_child: false,
                    parent_name: parent.name.clone(),
                    full_label: parent.name.clone(),
                    display_name: parent.name.clone(),
                    color: parent.color,
                });
            } else {
                // Push the parent as a header entry first
                entries.push(CatEntry {
                    is_child: false,
                    parent_name: parent.name.clone(),
                    full_label: parent.name.clone(),
                    display_name: parent.name.clone(),
                    color: parent.color,
                });
                let mut child_ids: Vec<i64> = self
                    .categories
                    .iter()
                    .filter(|c| c.parent_id == Some(pid))
                    .map(|c| c.id)
                    .collect();
                child_ids.sort_by(|a, b| {
                    let na = self
                        .categories
                        .iter()
                        .find(|c| c.id == *a)
                        .map(|c| c.name.as_str())
                        .unwrap_or("");
                    let nb = self
                        .categories
                        .iter()
                        .find(|c| c.id == *b)
                        .map(|c| c.name.as_str())
                        .unwrap_or("");
                    na.to_lowercase().cmp(&nb.to_lowercase())
                });
                for &cid in &child_ids {
                    let Some(child) = self.categories.iter().find(|c| c.id == cid) else {
                        continue;
                    };
                    let full_label = child.full_label(&cat_parents);
                    if excluded.contains(&full_label) {
                        continue;
                    }
                    entries.push(CatEntry {
                        is_child: true,
                        parent_name: parent.name.clone(),
                        full_label,
                        display_name: child.name.clone(),
                        color: parent.color,
                    });
                }
            }
        }

        // Filter
        struct CatRow {
            is_child: bool,
            parent_name: String,
            full_label: String,
            display_name: String,
            color: Color32,
            limit: i64,
            spent: i64,
        }

        let mut rows: Vec<CatRow> = Vec::new();
        for e in &entries {
            let limit = self.budgets.get(&e.full_label).copied().unwrap_or(0);
            let spent = spent_map.get(&e.full_label).copied().unwrap_or(0);
            let matches = match self.budget_filter {
                BudgetFilter::All => true,
                BudgetFilter::OverBudget => spent > limit,
                BudgetFilter::UnderBudget => spent <= limit && limit > 0,
                BudgetFilter::Unbudgeted => spent > 0 && limit == 0,
            };
            if matches {
                rows.push(CatRow {
                    is_child: e.is_child,
                    parent_name: e.parent_name.clone(),
                    full_label: e.full_label.clone(),
                    display_name: e.display_name.clone(),
                    color: e.color,
                    limit,
                    spent,
                });
            }
        }

        if rows.is_empty() {
            // real empty state, matches the rest of the app's empty
            // placeholders (icon + headline + subtext).
            crate::ui::components::empty_state(
                ui,
                "No categories match the active filter",
                "Switch the filter to All to see every category, or clear filters above.",
            );
            return;
        }

        // Table
        let sep_color = crate::ui::theme::border();
        let row_h = 22.0;
        let avail = ui.available_width();
        let cw = (avail - 4.0).max(10.0); // content width inside the 1px stroke inset

        const MIN_COL_PCT: f32 = 0.06;
        let d = self.budget_col_dividers;
        let col_w: Vec<f32> = vec![
            d[0] * cw,
            (d[1] - d[0]) * cw,
            (d[2] - d[1]) * cw,
            (d[3] - d[2]) * cw,
            (1.0 - d[3]) * cw,
        ];

        // Paint a background rect for the entire table area; the outer stroke is
        // painted after the data ScrollArea (rows bleeding past the scroll clip
        // would cover its bottom edge otherwise).
        let table_tl = ui.cursor().left_top();
        let table_avail_h = ui.available_height().max(120.0);
        let table_rect = egui::Rect::from_min_size(table_tl, egui::vec2(avail, table_avail_h));
        ui.painter()
            .rect_filled(table_rect, 8.0, crate::ui::theme::bg_primary());

        let left0 = table_rect.left() + 2.0;

        let col_x = |i: usize| -> f32 { left0 + col_w[..i].iter().sum::<f32>() };

        let divider_xs: Vec<f32> = col_w
            .iter()
            .scan(left0, |acc, &w| {
                *acc += w;
                Some(*acc)
            })
            .collect();

        // Column divider drag handles
        for i in 0..4 {
            let handle_rect = egui::Rect::from_min_size(
                egui::pos2(divider_xs[i] - 3.0, table_rect.top()),
                egui::vec2(6.0, table_rect.height()),
            );
            let id = egui::Id::new(("budget_col_divider", i));
            let response = ui.interact(handle_rect, id, egui::Sense::drag());
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            }
            if response.dragged() {
                let delta = response.drag_delta().x;
                let new_frac = self.budget_col_dividers[i] + delta / cw;
                let prev = if i > 0 {
                    self.budget_col_dividers[i - 1]
                } else {
                    0.0
                };
                let next = if i < 3 {
                    self.budget_col_dividers[i + 1]
                } else {
                    1.0
                };
                self.budget_col_dividers[i] =
                    new_frac.clamp(prev + MIN_COL_PCT, next - MIN_COL_PCT);
                ui.ctx().request_repaint();
            }
        }

        let paint_vseps = |ui: &egui::Ui, y0: f32, y1: f32| {
            for &x in &divider_xs[..divider_xs.len() - 1] {
                ui.painter().line_segment(
                    [egui::pos2(x, y0), egui::pos2(x, y1)],
                    Stroke::new(1.0_f32, sep_color),
                );
            }
        };

        // ---- Header row (fixed) ----
        // bg_subtle matches the expense / import / duplicates grid
        // headers so all four tables in the app have the same header band.
        // 1px bottom stroke (was 1.5) to match the other borders in the table
        // and the design system.
        // Painted AFTER the data ScrollArea below: scrolled rows bleed a few px
        // above the scroll clip (clip_rect_margin) and were covering the header
        // bottom border whenever the table had a scrollbar.
        let hdr_top = table_rect.top() + 1.0;
        let hdr_rect = egui::Rect::from_min_size(egui::pos2(left0, hdr_top), egui::vec2(cw, row_h));

        // Advance cursor past the header so the scroll area starts below it
        ui.allocate_space(egui::vec2(avail, row_h));

        // ---- Scrollable data rows (remaining height) ----
        let data_h = ui.available_height().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(avail, data_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("budget_data_rows")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut row_idx = 0usize;
                        let mut i = 0;
                        let past_period = self.budget_year < chrono::Local::now().year();
                        while i < rows.len() {
                            let cat = &rows[i];
                            let y0 = ui.cursor().top();

                            if !cat.is_child
                                && i + 1 < rows.len()
                                && rows[i + 1].is_child
                                && rows[i + 1].parent_name == cat.parent_name
                            {
                                // ===== Parent header row (with summed children, tinted bg) =====
                                let children: Vec<&CatRow> = rows[i + 1..]
                                    .iter()
                                    .take_while(|r| r.is_child && r.parent_name == cat.parent_name)
                                    .collect();
                                let child_count = children.len();
                                let limit_sum: i64 = children.iter().map(|c| c.limit).sum();
                                let spent_sum: i64 = children.iter().map(|c| c.spent).sum();
                                let rem_sum = limit_sum - spent_sum;
                                let pct = if limit_sum > 0 {
                                    (spent_sum as f32 / limit_sum as f32).clamp(0.0, 2.0)
                                } else if spent_sum > 0 {
                                    1.5
                                } else {
                                    0.0
                                };

                                let cat_bg = Color32::from_rgba_unmultiplied(
                                    cat.color.r(),
                                    cat.color.g(),
                                    cat.color.b(),
                                    35,
                                );
                                let row_rect = egui::Rect::from_min_size(
                                    egui::pos2(left0, y0),
                                    egui::vec2(cw, row_h),
                                );
                                ui.painter().rect_filled(row_rect, 0.0, cat_bg);
                                ui.painter().line_segment(
                                    [
                                        egui::pos2(left0, y0 + row_h),
                                        egui::pos2(left0 + cw, y0 + row_h),
                                    ],
                                    Stroke::new(1.5_f32, sep_color),
                                );

                                let c0 = egui::Rect::from_min_size(
                                    egui::pos2(col_x(0), y0),
                                    egui::vec2(col_w[0], row_h),
                                );
                                let c1 = egui::Rect::from_min_size(
                                    egui::pos2(col_x(1), y0),
                                    egui::vec2(col_w[1], row_h),
                                );
                                let c2 = egui::Rect::from_min_size(
                                    egui::pos2(col_x(2), y0),
                                    egui::vec2(col_w[2], row_h),
                                );
                                let c3 = egui::Rect::from_min_size(
                                    egui::pos2(col_x(3), y0),
                                    egui::vec2(col_w[3], row_h),
                                );

                                let swatch_rect = egui::Rect::from_min_size(
                                    egui::pos2(left0 + 6.0, y0 + row_h / 2.0 - 5.0),
                                    egui::vec2(10.0, 10.0),
                                );
                                // Themed swatch with a 1px border so it reads against the tinted row background.
                                crate::ui::components::color_swatch_decorated(
                                    ui.painter(),
                                    swatch_rect,
                                    cat.color,
                                    crate::ui::theme::border(),
                                );
                                clip_text(
                                    ui,
                                    c0,
                                    egui::pos2(left0 + 22.0, y0 + row_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    &cat.parent_name,
                                    egui::FontId::proportional(13.0),
                                    crate::ui::theme::fg_primary(),
                                );

                                clip_text(
                                    ui,
                                    c1,
                                    egui::pos2(col_x(1) + 8.0, y0 + row_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    &money(limit_sum),
                                    egui::FontId::monospace(13.0),
                                    crate::ui::theme::fg_primary(),
                                );
                                clip_text(
                                    ui,
                                    c2,
                                    egui::pos2(col_x(2) + 8.0, y0 + row_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    &money(spent_sum),
                                    egui::FontId::monospace(13.0),
                                    crate::ui::theme::fg_primary(),
                                );
                                let rem_col = if rem_sum >= 0 {
                                    crate::ui::theme::fg_primary()
                                } else {
                                    crate::ui::theme::error()
                                };
                                clip_text(
                                    ui,
                                    c3,
                                    egui::pos2(col_x(3) + 8.0, y0 + row_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    &money(rem_sum),
                                    egui::FontId::monospace(13.0),
                                    rem_col,
                                );
                                let bar_cell_w = col_w[4];
                                let bar_rect = egui::Rect::from_min_size(
                                    egui::pos2(col_x(4), y0),
                                    egui::vec2(bar_cell_w, row_h),
                                );
                                // progress_bar now lives in components. Same
                                // semantic (≤80% success, ≤100% warning, >100% error) but
                                // routes through the components palette.
                                crate::ui::components::progress_bar(ui, bar_rect, pct);

                                paint_vseps(ui, y0, y0 + row_h);
                                ui.allocate_space(egui::vec2(0.0, row_h));
                                row_idx += 1;

                                // ===== Child rows (indented, editable) =====
                                for child in &rows[i + 1..i + 1 + child_count] {
                                    let cy0 = ui.cursor().top();
                                    let c0c = egui::Rect::from_min_size(
                                        egui::pos2(col_x(0), cy0),
                                        egui::vec2(col_w[0], row_h),
                                    );
                                    let c2c = egui::Rect::from_min_size(
                                        egui::pos2(col_x(2), cy0),
                                        egui::vec2(col_w[2], row_h),
                                    );
                                    let c3c = egui::Rect::from_min_size(
                                        egui::pos2(col_x(3), cy0),
                                        egui::vec2(col_w[3], row_h),
                                    );
                                    let row_bg = if row_idx % 2 == 0 {
                                        crate::ui::theme::bg_secondary()
                                    } else {
                                        crate::ui::theme::bg_primary()
                                    };
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(
                                            egui::pos2(left0, cy0),
                                            egui::vec2(cw, row_h),
                                        ),
                                        0.0,
                                        row_bg,
                                    );

                                    clip_text(
                                        ui,
                                        c0c,
                                        egui::pos2(left0 + 24.0, cy0 + row_h / 2.0),
                                        egui::Align2::LEFT_CENTER,
                                        &child.display_name,
                                        egui::FontId::proportional(13.0),
                                        crate::ui::theme::fg_primary(),
                                    );
                                    let bgt_rect = egui::Rect::from_min_size(
                                        egui::pos2(col_x(1), cy0),
                                        egui::vec2(col_w[1], row_h),
                                    );
                                    self.render_budget_limit_cell(
                                        ui,
                                        &child.full_label,
                                        child.limit,
                                        bgt_rect,
                                        past_period,
                                    );
                                    clip_text(
                                        ui,
                                        c2c,
                                        egui::pos2(col_x(2) + 8.0, cy0 + row_h / 2.0),
                                        egui::Align2::LEFT_CENTER,
                                        &money(child.spent),
                                        egui::FontId::monospace(13.0),
                                        crate::ui::theme::fg_primary(),
                                    );
                                    let rem = child.limit - child.spent;
                                    let rem_col = if rem >= 0 {
                                        crate::ui::theme::fg_primary()
                                    } else {
                                        crate::ui::theme::error()
                                    };
                                    clip_text(
                                        ui,
                                        c3c,
                                        egui::pos2(col_x(3) + 8.0, cy0 + row_h / 2.0),
                                        egui::Align2::LEFT_CENTER,
                                        &money(rem),
                                        egui::FontId::monospace(13.0),
                                        rem_col,
                                    );
                                    let pct = if child.limit > 0 {
                                        (child.spent as f32 / child.limit as f32).clamp(0.0, 2.0)
                                    } else if child.spent > 0 {
                                        1.5
                                    } else {
                                        0.0
                                    };
                                    let bar_cell_w = col_w[4];
                                    let bar_rect = egui::Rect::from_min_size(
                                        egui::pos2(col_x(4), cy0),
                                        egui::vec2(bar_cell_w, row_h),
                                    );
                                    crate::ui::components::progress_bar(ui, bar_rect, pct);

                                    paint_vseps(ui, cy0, cy0 + row_h);
                                    ui.allocate_space(egui::vec2(0.0, row_h));
                                    row_idx += 1;
                                }
                                i += 1 + child_count;
                            } else {
                                // ===== Leaf row (no children) =====
                                let c0 = egui::Rect::from_min_size(
                                    egui::pos2(col_x(0), y0),
                                    egui::vec2(col_w[0], row_h),
                                );
                                let c2 = egui::Rect::from_min_size(
                                    egui::pos2(col_x(2), y0),
                                    egui::vec2(col_w[2], row_h),
                                );
                                let c3 = egui::Rect::from_min_size(
                                    egui::pos2(col_x(3), y0),
                                    egui::vec2(col_w[3], row_h),
                                );
                                let row_bg = if row_idx % 2 == 0 {
                                    crate::ui::theme::bg_secondary()
                                } else {
                                    crate::ui::theme::bg_primary()
                                };
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        egui::pos2(left0, y0),
                                        egui::vec2(cw, row_h),
                                    ),
                                    0.0,
                                    row_bg,
                                );

                                let swatch_rect = egui::Rect::from_min_size(
                                    egui::pos2(left0 + 6.0, y0 + row_h / 2.0 - 5.0),
                                    egui::vec2(10.0, 10.0),
                                );
                                // Themed swatch with a 1px border so it reads against the tinted row background.
                                crate::ui::components::color_swatch_decorated(
                                    ui.painter(),
                                    swatch_rect,
                                    cat.color,
                                    crate::ui::theme::border(),
                                );
                                clip_text(
                                    ui,
                                    c0,
                                    egui::pos2(left0 + 22.0, y0 + row_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    &cat.display_name,
                                    egui::FontId::proportional(13.0),
                                    crate::ui::theme::fg_primary(),
                                );
                                let bgt_rect = egui::Rect::from_min_size(
                                    egui::pos2(col_x(1), y0),
                                    egui::vec2(col_w[1], row_h),
                                );
                                self.render_budget_limit_cell(
                                    ui,
                                    &cat.full_label,
                                    cat.limit,
                                    bgt_rect,
                                    past_period,
                                );
                                clip_text(
                                    ui,
                                    c2,
                                    egui::pos2(col_x(2) + 8.0, y0 + row_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    &money(cat.spent),
                                    egui::FontId::monospace(13.0),
                                    crate::ui::theme::fg_primary(),
                                );
                                let rem = cat.limit - cat.spent;
                                // live palette (theme::error) like the
                                // parent/child rows, never a stale static.
                                let rem_col = if rem >= 0 {
                                    crate::ui::theme::fg_primary()
                                } else {
                                    crate::ui::theme::error()
                                };
                                clip_text(
                                    ui,
                                    c3,
                                    egui::pos2(col_x(3) + 8.0, y0 + row_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    &money(rem),
                                    egui::FontId::monospace(13.0),
                                    rem_col,
                                );
                                let pct = if cat.limit > 0 {
                                    (cat.spent as f32 / cat.limit as f32).clamp(0.0, 2.0)
                                } else if cat.spent > 0 {
                                    1.5
                                } else {
                                    0.0
                                };
                                let bar_cell_w = col_w[4];
                                let bar_rect = egui::Rect::from_min_size(
                                    egui::pos2(col_x(4), y0),
                                    egui::vec2(bar_cell_w, row_h),
                                );
                                // same progress_bar as parent/child rows so all three
                                // row shapes render the bar identically (track + fill + label).
                                crate::ui::components::progress_bar(ui, bar_rect, pct);

                                paint_vseps(ui, y0, y0 + row_h);
                                ui.allocate_space(egui::vec2(0.0, row_h));
                                row_idx += 1;
                                i += 1;
                            }
                        }
                    });

                // Table outer stroke + header row (fixed) — painted after the data
                // ScrollArea so scrolled row backgrounds (which can bleed up to
                // clip_rect_margin above the scroll clip) can't cover them.
                ui.painter().rect_stroke(
                    table_rect,
                    8.0,
                    Stroke::new(1.0_f32, sep_color),
                    egui::StrokeKind::Inside,
                );
                ui.painter()
                    .rect_filled(hdr_rect, 0.0, crate::ui::theme::bg_secondary());
                ui.painter().line_segment(
                    [
                        egui::pos2(left0, hdr_top + row_h),
                        egui::pos2(left0 + cw, hdr_top + row_h),
                    ],
                    Stroke::new(1.0_f32, crate::ui::theme::border()),
                );
                let hdr_labels = [
                    "Category",
                    "Allocated Limit",
                    "Actual Spending",
                    "Remaining",
                    "Progress",
                ];
                // small uppercase muted labels, same style as the other
                // table headers in the app.
                let hdr_text = crate::ui::theme::fg_secondary();
                for (i, label) in hdr_labels.iter().enumerate() {
                    let cx = col_x(i) + 8.0;
                    ui.painter().text(
                        egui::pos2(cx, hdr_top + row_h / 2.0),
                        egui::Align2::LEFT_CENTER,
                        &label.to_uppercase(),
                        egui::FontId::proportional(11.0),
                        hdr_text,
                    );
                }
                paint_vseps(ui, hdr_top, hdr_top + row_h);
            },
        );
    }

    fn render_budget_limit_cell(
        &mut self,
        ui: &mut egui::Ui,
        full_label: &str,
        limit_cents: i64,
        cell_rect: egui::Rect,
        past_period: bool,
    ) {
        let original_cents = limit_cents.max(0);
        let limit_text = money(original_cents);
        let limit_input = limit_text
            .strip_prefix('$')
            .unwrap_or(&limit_text)
            .to_string();
        let text_edit_id = egui::Id::new(("budget_limit_edit", full_label));
        let init_id = text_edit_id.with("initialize");
        let input_char_count = self.budget_editing_input.chars().count();

        if !past_period && self.budget_editing_category.as_deref() == Some(full_label) {
            // Use an Area overlay so the TextEdit doesn't participate in layout
            // (no cursor advancement, no ghost rows).
            let area_id = text_edit_id.with("area");
            let (ready, resp) = egui::Area::new(area_id)
                .fixed_pos(cell_rect.left_top())
                .show(ui.ctx(), |ui| {
                    ui.set_width(cell_rect.width());
                    let mut output = egui::TextEdit::singleline(&mut self.budget_editing_input)
                        .id(text_edit_id)
                        .font(egui::FontId::monospace(13.0))
                        .desired_width(cell_rect.width() - 8.0)
                        .show(ui);
                    let ready = !ui.is_sizing_pass();
                    if ready {
                        initialize_budget_text_edit(
                            ui,
                            &mut output,
                            input_char_count,
                            text_edit_id,
                            init_id,
                        );
                    }
                    (ready, output.response)
                })
                .inner;
            let plain_enter =
                ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            let pressed_escape =
                ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            if pressed_escape {
                self.budget_editing_input = limit_input;
                self.budget_editing_category = None;
            } else if ready && plain_enter {
                if let Some(value_cents) = parse_budget_cents(&self.budget_editing_input) {
                    if value_cents != original_cents {
                        self.save_budget_with_forward_propagation(full_label, value_cents);
                    }
                }
                self.budget_editing_input = limit_input;
                self.budget_editing_category = None;
            } else if ready && resp.lost_focus() {
                self.budget_editing_input = limit_input;
                self.budget_editing_category = None;
            }
        } else {
            if past_period {
                // Dimmed display, no interaction for past periods
                ui.painter().text(
                    egui::pos2(cell_rect.left() + 8.0, cell_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &limit_text,
                    egui::FontId::monospace(13.0),
                    crate::ui::theme::fg_secondary(),
                );
            } else {
                ui.painter().text(
                    egui::pos2(cell_rect.left() + 8.0, cell_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &limit_text,
                    egui::FontId::monospace(13.0),
                    if limit_cents > 0 {
                        crate::ui::theme::fg_primary()
                    } else {
                        crate::ui::theme::fg_secondary()
                    },
                );
                let id = egui::Id::new(("budget_limit_click", full_label));
                let resp = ui.interact(cell_rect, id, egui::Sense::click());
                if resp.clicked() {
                    self.budget_editing_category = Some(full_label.to_string());
                    self.budget_editing_input = limit_input;
                    ui.ctx()
                        .data_mut(|data| data.insert_temp::<bool>(init_id, false));
                }
            }
        }
    }

    fn save_budget_with_forward_propagation(&mut self, full_label: &str, val_cents: i64) {
        if let Err(error) = self.save_budget_plan(full_label, val_cents) {
            self.set_transient_status(error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{egui, initialize_budget_text_edit, parse_budget_cents, MAX_BUDGET_CENTS};

    #[test]
    fn budget_parser_accepts_exact_currency() {
        for (input, expected) in [
            ("0", 0),
            (" $0.00 ", 0),
            ("7", 700),
            ("7.1", 710),
            ("$7.09", 709),
            ("1,234", 123_400),
            ("$1,234,567.8", 123_456_780),
            ("0007", 700),
        ] {
            assert_eq!(parse_budget_cents(input), Some(expected), "{input:?}");
        }
    }

    #[test]
    fn budget_parser_rejects_invalid_currency() {
        for input in [
            "",
            " ",
            "$",
            ".5",
            "-1",
            "+1",
            "1e3",
            "NaN",
            "inf",
            "1.",
            "1.234",
            ",123",
            "1,",
            "1,00",
            "1000,000",
            "1,000,00",
            "1,,000",
            "1,000,",
            "1 000",
            "$ 1",
            "1.$",
            "USD1",
            "1,000.00,",
            "1,000.000",
        ] {
            assert_eq!(parse_budget_cents(input), None, "{input:?}");
        }
    }

    #[test]
    fn budget_parser_rejects_unsafe_values() {
        assert_eq!(parse_budget_cents("92233720368.54"), Some(MAX_BUDGET_CENTS));
        assert_eq!(parse_budget_cents("92233720368.55"), None);
        assert_eq!(parse_budget_cents("92233720368547758.07"), None);
    }

    #[test]
    fn budget_text_edit_initializes_once_per_open() {
        egui::__run_test_ui(|ui| {
            let text_edit_id = egui::Id::new("budget_limit_edit_test");
            let init_id = text_edit_id.with("initialize");
            ui.ctx()
                .data_mut(|data| data.insert_temp::<bool>(init_id, false));
            let mut input = String::from("123.45");
            let input_char_count = input.chars().count();
            let mut output = egui::TextEdit::singleline(&mut input)
                .id(text_edit_id)
                .show(ui);

            initialize_budget_text_edit(ui, &mut output, input_char_count, text_edit_id, init_id);

            assert!(ui.ctx().memory(|memory| memory.has_focus(text_edit_id)));
            let mut state = egui::widgets::text_edit::TextEditState::load(ui.ctx(), text_edit_id)
                .expect("text edit state");
            assert_eq!(
                state.cursor.char_range().unwrap().as_sorted_char_range(),
                egui::text::CharIndex(0)..egui::text::CharIndex(6)
            );
            state.cursor.set_char_range(None);
            state.store(ui.ctx(), text_edit_id);

            initialize_budget_text_edit(ui, &mut output, input_char_count, text_edit_id, init_id);

            let state = egui::widgets::text_edit::TextEditState::load(ui.ctx(), text_edit_id)
                .expect("text edit state");
            assert!(state.cursor.is_empty());

            ui.ctx()
                .data_mut(|data| data.insert_temp::<bool>(init_id, false));
            initialize_budget_text_edit(ui, &mut output, input_char_count, text_edit_id, init_id);

            let state = egui::widgets::text_edit::TextEditState::load(ui.ctx(), text_edit_id)
                .expect("text edit state");
            assert_eq!(
                state.cursor.char_range().unwrap().as_sorted_char_range(),
                egui::text::CharIndex(0)..egui::text::CharIndex(6)
            );
        });
    }
}
