use crate::db::*;
use crate::models::*;
use crate::ui::analytics::aggregation::parse_expense_date;
use eframe::egui::{self, Color32, RichText, Stroke};
use rusqlite::OptionalExtension;
use std::collections::{HashMap, HashSet};

/// One category's contribution to the settlement math, for the selected range.
pub struct CategorySettlement {
    pub category: String,
    pub spend_cents: i64,
    pub paid_by: HashMap<String, i64>,
    pub allocated: HashMap<String, i64>,
}

#[derive(Default)]
pub struct SettlementTotals {
    pub paid: HashMap<String, i64>,
    pub owed: HashMap<String, i64>,
    pub categories: Vec<CategorySettlement>,
}

/// Does this row fall inside the selected range? An unbounded range (All
/// Time) keeps every row, including one whose date never parsed.
fn in_date_range(
    raw: &str,
    start: Option<chrono::NaiveDate>,
    end: Option<chrono::NaiveDate>,
) -> bool {
    if start.is_none() && end.is_none() {
        return true;
    }
    let Some(date) = parse_expense_date(raw) else {
        return false;
    };
    if start.is_some_and(|start| date < start) || end.is_some_and(|end| date > end) {
        return false;
    }
    true
}

/// Who-owes-whom and the category breakdown, in one pass. Counts only
/// split-covered debits inside the range: credits and user-excluded transfer
/// categories are not household payments, and a category with no non-zero
/// allocation is skipped entirely.
pub fn settlement_totals(
    expenses: &[Expense],
    split_shares: &HashMap<String, Vec<(String, f64)>>,
    excluded: &HashSet<String>,
    start: Option<chrono::NaiveDate>,
    end: Option<chrono::NaiveDate>,
) -> SettlementTotals {
    let mut totals = SettlementTotals::default();
    let mut by_category: HashMap<String, CategorySettlement> = HashMap::new();

    for exp in expenses {
        if !in_date_range(&exp.date, start, end) {
            continue;
        }
        if exp.amount_cents >= 0 || excluded.contains(&exp.category) {
            continue;
        }
        let Some(allocations) = split_shares.get(&exp.category) else {
            continue;
        };
        let total = -exp.amount_cents;
        *totals.paid.entry(exp.member.clone()).or_insert(0) += total;
        let entry = by_category
            .entry(exp.category.clone())
            .or_insert_with(|| CategorySettlement {
                category: exp.category.clone(),
                spend_cents: 0,
                paid_by: HashMap::new(),
                allocated: HashMap::new(),
            });
        entry.spend_cents += total;
        *entry.paid_by.entry(exp.member.clone()).or_insert(0) += total;
        for (member_name, percentage) in allocations {
            let share = (total as f64 * percentage / 100.0).round() as i64;
            *totals.owed.entry(member_name.clone()).or_insert(0) += share;
            *entry.allocated.entry(member_name.clone()).or_insert(0) += share;
        }
    }

    totals.categories = by_category.into_values().collect();
    totals.categories.sort_by(|a, b| {
        b.spend_cents
            .cmp(&a.spend_cents)
            .then(a.category.cmp(&b.category))
    });
    totals
}

const NAME_COL_W: f32 = 150.0;
const BTN_COL_W: f32 = 46.0;
const ROW_H: f32 = 24.0;
const SWATCH_R: f32 = 5.0;
const INDENT: f32 = 16.0;
const MIN_MEMBER_COL_W: f32 = 52.0;
/// Right padding between the =/× buttons and the block border, so the
/// clear button sits inset like the rest of the app's controls.
const BTN_PAD: f32 = 6.0;
const BLOCK_GAP: f32 = 6.0;
/// Fixed block height: pinned header + 8 visible data rows + border allowance.
const BLOCK_H: f32 = 9.0 * ROW_H + 4.0;

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

fn format_cents(cents: i64) -> String {
    format!("{:.2}", cents.unsigned_abs() as f64 / 100.0)
}

#[derive(Clone)]
struct BlockRow<'a> {
    category_label: String,
    display_name: &'a str,
    color: Color32,
    is_parent: bool,
}

impl super::super::TwoCentsApp {
    pub fn ui_settlements(&mut self, ui: &mut egui::Ui) {
        let members = self.members.clone();
        let categories = self.categories.clone();
        let expenses = self.expenses.clone();
        let splits = self.category_splits.clone();
        let parents = category_parent_map(&categories);

        crate::ui::components::heading_lg(ui, "Settlements");
        ui.add_space(crate::ui::theme_tokens::SPACE_1);
        crate::ui::components::label_muted(
            ui,
            "Configure how expenses are split across household members.",
        );
        ui.add_space(crate::ui::theme_tokens::SPACE_2);

        if members.is_empty() {
            crate::ui::components::label_muted(ui, "Add household members first.");
            return;
        }

        // Own date range: the analytics preset strip, stored separately so the
        // two tabs never move each other. Rolling presets are recomputed every
        // frame so YTD and friends stay current.
        let mut range_changed = crate::ui::analytics::filters::date_range_row(
            ui,
            &mut self.settlements_date_preset,
            &mut self.settlements_date_start,
            &mut self.settlements_date_end,
        );
        if crate::ui::popups::styled_button(ui, "Reset", false).clicked() {
            self.settlements_date_preset = crate::ui::analytics::state::DatePreset::AllTime;
            self.settlements_date_start = None;
            self.settlements_date_end = None;
            range_changed = true;
        }
        if range_changed {
            self.persist_settlements_date_range();
        }
        let (range_start, range_end) =
            if self.settlements_date_preset == crate::ui::analytics::state::DatePreset::Custom {
                (self.settlements_date_start, self.settlements_date_end)
            } else {
                self.settlements_date_preset.date_range()
            };
        ui.add_space(crate::ui::theme_tokens::SPACE_2);

        // === Who Owes Whom (balances + suggested payments side by side) ===
        crate::ui::components::heading_lg(ui, "Who Owes Whom");
        ui.add_space(crate::ui::theme_tokens::SPACE_2);

        // settlements are about shared spending — debit rows only,
        // magnitude; income (credits) and excluded transfer/payment rows are
        // not household payments and never enter the math.
        let excluded = excluded_category_labels(&categories);

        // Who Owes Whom counts ONLY split-covered categories — any
        // category with at least one non-zero allocation. No equal-split
        // fallback: it dragged every unassigned expense into the math (±$36k
        // with one split category configured). Categories whose splits are all
        // zero stay out too. A 100%-one-member split still counts when the
        // OTHER member paid the row (personal spend on the other's card).
        let mut split_shares: std::collections::HashMap<String, Vec<(String, f64)>> =
            std::collections::HashMap::new();
        for split in &splits {
            if split.percentage != 0.0 {
                split_shares
                    .entry(split.category.clone())
                    .or_default()
                    .push((split.member_name.clone(), split.percentage));
            }
        }

        let totals = settlement_totals(&expenses, &split_shares, &excluded, range_start, range_end);

        if totals.paid.values().all(|&value| value == 0) {
            crate::ui::components::label_muted(
                ui,
                "Balances only count categories with split percentages assigned below.",
            );
            ui.add_space(crate::ui::theme_tokens::SPACE_2);
        }

        let mut net: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for m in &members {
            let p = totals.paid.get(&m.name).copied().unwrap_or(0);
            let o = totals.owed.get(&m.name).copied().unwrap_or(0);
            net.insert(m.name.clone(), p - o);
        }

        let mut debtors: Vec<(String, i64)> = net
            .iter()
            .filter(|(_, &v)| v < 0)
            .map(|(k, &v)| (k.clone(), -v))
            .collect();
        let mut creditors: Vec<(String, i64)> = net
            .iter()
            .filter(|(_, &v)| v > 0)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        debtors.sort_by(|a, b| b.1.cmp(&a.1));
        creditors.sort_by(|a, b| b.1.cmp(&a.1));

        let two_or_more = members.len() >= 2;
        // single column — balances, then suggested payments directly
        // under them (was a side-by-side columns(2) split).
        crate::ui::components::label_muted(ui, "Balances");
        if !two_or_more {
            crate::ui::components::label_muted(
                ui,
                "Add at least 2 household members to see settlements.",
            );
        } else {
            for m in &members {
                let balance = net.get(&m.name).copied().unwrap_or(0);
                let color = if balance > 0 {
                    crate::ui::theme::accent()
                } else if balance < 0 {
                    crate::ui::theme::error()
                } else {
                    crate::ui::theme::fg_secondary()
                };
                let sign = if balance >= 0 { "+" } else { "-" };
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(SWATCH_R * 2.0, SWATCH_R * 2.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().circle_filled(rect.center(), SWATCH_R, m.color);
                    // egui hardwires strong() text to widgets.active's
                    // fg_stroke (contrast-on-accent = near-black here), so strong
                    // labels must carry an explicit palette color.
                    ui.label(
                        RichText::new(&m.name)
                            .strong()
                            .color(crate::ui::theme::fg_primary()),
                    );
                    ui.label(
                        RichText::new(format!("{}${}", sign, format_cents(balance))).color(color),
                    );
                });
            }

            ui.add_space(crate::ui::theme_tokens::SPACE_1);
            let mut di = 0;
            let mut ci = 0;
            let mut emitted = 0;
            while di < debtors.len() && ci < creditors.len() {
                let amount = debtors[di].1.min(creditors[ci].1);
                if amount > 0 {
                    // explicit fg_default — this line previously used egui's
                    // default text stroke, which is also what muted backgrounds show;
                    // in light variants it rendered washed-out (light-on-light).
                    ui.label(
                        RichText::new(format!(
                            "{} pays {} ${}",
                            debtors[di].0,
                            creditors[ci].0,
                            format_cents(amount)
                        ))
                        .small()
                        .color(crate::ui::theme::fg_primary()),
                    );
                    emitted += 1;
                }
                debtors[di].1 -= amount;
                creditors[ci].1 -= amount;
                if debtors[di].1 == 0 {
                    di += 1;
                }
                if creditors[ci].1 == 0 {
                    ci += 1;
                }
            }
            if emitted == 0 {
                crate::ui::components::label_muted(ui, "All settled — no payments needed.");
            }
        }

        ui.add_space(crate::ui::theme_tokens::SPACE_2);

        // === Category breakdown for the selected range ===
        crate::ui::components::heading_lg(ui, "Category Breakdown");
        ui.add_space(crate::ui::theme_tokens::SPACE_1);
        crate::ui::components::label_muted(
            ui,
            "Shared spend inside the selected range, and each member's net on it.",
        );
        ui.add_space(crate::ui::theme_tokens::SPACE_2);
        if totals.categories.is_empty() {
            crate::ui::components::label_muted(
                ui,
                "No split-covered spending in this range. Widen the date range or assign split percentages below.",
            );
        } else {
            egui::ScrollArea::vertical()
                .id_salt("settlements_breakdown")
                .auto_shrink([false, false])
                .max_height(200.0)
                .show(ui, |ui| {
                    for category in &totals.categories {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(SWATCH_R * 2.0, SWATCH_R * 2.0),
                                egui::Sense::hover(),
                            );
                            let color = categories
                                .iter()
                                .find(|c| c.full_label(&parents) == category.category)
                                .map_or(crate::ui::theme::fg_secondary(), |c| c.color);
                            ui.painter().circle_filled(rect.center(), SWATCH_R, color);
                            ui.label(
                                RichText::new(&category.category)
                                    .color(crate::ui::theme::fg_primary()),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    for m in members.iter().rev() {
                                        let paid =
                                            category.paid_by.get(&m.name).copied().unwrap_or(0);
                                        let share =
                                            category.allocated.get(&m.name).copied().unwrap_or(0);
                                        if paid == 0 && share == 0 {
                                            continue;
                                        }
                                        let net = paid - share;
                                        let (sign, color) = if net >= 0 {
                                            ("+", crate::ui::theme::accent())
                                        } else {
                                            ("-", crate::ui::theme::error())
                                        };
                                        ui.label(
                                            RichText::new(format!(
                                                "{} {sign}${}",
                                                m.name,
                                                format_cents(net)
                                            ))
                                            .small()
                                            .color(color),
                                        );
                                        ui.add_space(crate::ui::theme_tokens::SPACE_2);
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "${}",
                                            format_cents(category.spend_cents)
                                        ))
                                        .strong()
                                        .color(crate::ui::theme::fg_primary()),
                                    );
                                },
                            );
                        });
                    }
                });
        }

        ui.add_space(crate::ui::theme_tokens::SPACE_2);

        let parent_ids = sorted_parent_category_ids(&categories);
        let num_members = members.len();

        // Collect deferred actions
        let mut pending_edits = std::mem::take(&mut self.settlement_pending_edits);
        let mut commit_edit = false;
        let mut clear_target: Option<(String, bool)> = None;
        let mut apply_parent_to_subs: Option<String> = None;
        let mut apply_equal_split: Option<String> = None;

        // === Category blocks with wrapping layout (member headers render inside each block) ===
        egui::ScrollArea::vertical()
            .id_salt("settlements_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let w = ui.available_width();
                // Planning minimum: name col + right-anchored btn col + one
                // min-width member col per member. Decides how many blocks fit per
                // wrap row; actual member cols just divide the leftover space.
                let min_block_w =
                    NAME_COL_W + BTN_COL_W + BTN_PAD + num_members as f32 * MIN_MEMBER_COL_W;
                let cols = ((w + BLOCK_GAP) / (min_block_w + BLOCK_GAP))
                    .floor()
                    .max(1.0) as usize;
                let block_w = (w - BLOCK_GAP * (cols - 1) as f32) / cols as f32;

                // Build blocks: header (parent) + data rows (subs; the parent itself when childless)
                // excluded categories (user-flagged) get no block at all —
                // their rows never enter the math either (settlement_totals filters them).
                let mut blocks: Vec<(BlockRow, Vec<BlockRow>)> = Vec::new();
                for parent_id in &parent_ids {
                    let Some(parent) = categories.iter().find(|c| c.id == *parent_id) else {
                        continue;
                    };
                    if excluded.contains(&parent.name) {
                        continue;
                    }
                    let header = BlockRow {
                        category_label: parent.name.clone(),
                        display_name: &parent.name,
                        color: parent.color,
                        is_parent: true,
                    };

                    let mut subs: Vec<BlockRow> = Vec::new();
                    let mut sub_ids: Vec<i64> = categories
                        .iter()
                        .filter(|c| c.parent_id == Some(parent.id))
                        .map(|c| c.id)
                        .collect();
                    sub_ids.sort_by_key(|id| {
                        categories
                            .iter()
                            .find(|c| c.id == *id)
                            .map(|c| c.name.to_lowercase())
                            .unwrap_or_default()
                    });
                    for sub_id in &sub_ids {
                        if let Some(sub) = categories.iter().find(|c| c.id == *sub_id) {
                            subs.push(BlockRow {
                                category_label: sub.full_label(&parents),
                                display_name: &sub.name,
                                color: sub.color,
                                is_parent: false,
                            });
                        }
                    }
                    // Childless parents are assignable — their splits live on a data row.
                    let data_rows = if subs.is_empty() {
                        vec![header.clone()]
                    } else {
                        subs
                    };
                    blocks.push((header, data_rows));
                }

                // Render blocks in wrapping grid — every block a fixed height
                let mut row_start = 0;
                while row_start < blocks.len() {
                    let row_end = (row_start + cols).min(blocks.len());
                    let row_blocks = &blocks[row_start..row_end];

                    // Render each block side by side
                    ui.allocate_ui_with_layout(
                        egui::vec2(w, BLOCK_H),
                        egui::Layout::left_to_right(egui::Align::TOP),
                        |ui| {
                            // Zero x-spacing: the cols/block_w math above budgets only
                            // BLOCK_GAP between blocks, but the default item_spacing.x is
                            // also applied between every LTR child — overflowing the row
                            // and pushing the last block past the clip edge on wide screens.
                            ui.spacing_mut().item_spacing.x = 0.0;
                            for (i, (parent, rows)) in row_blocks.iter().enumerate() {
                                if i > 0 {
                                    ui.add_space(BLOCK_GAP);
                                }
                                let has_subs = rows.iter().any(|r| !r.is_parent);
                                render_category_block(
                                    ui,
                                    block_w,
                                    parent,
                                    rows,
                                    has_subs,
                                    &members,
                                    &splits,
                                    &mut pending_edits,
                                    &mut commit_edit,
                                    &mut clear_target,
                                    &mut apply_parent_to_subs,
                                    &mut apply_equal_split,
                                );
                            }
                        },
                    );
                    // Padding between block rows
                    ui.add_space(BLOCK_GAP);

                    row_start = row_end;
                }
            });

        self.settlement_pending_edits = pending_edits;
        if commit_edit {
            self.commit_pending_settlement_edits();
        }
        if !self.settlement_pending_edits.is_empty() {
            return;
        }
        if let Some((label, include_subs)) = clear_target {
            let descendant_count = if include_subs {
                Self::settlement_equal_labels(&categories, &label, true)
                    .len()
                    .saturating_sub(1)
            } else {
                0
            };
            self.open_destructive_confirmation(DestructiveConfirm::Settlement {
                label,
                include_subcategories: include_subs,
                descendant_count,
            });
        }
        if let Some(parent_label) = apply_parent_to_subs {
            self.apply_settlement_equal_split(parent_label, true);
        }
        if let Some(cat_label) = apply_equal_split {
            self.apply_settlement_equal_split(cat_label, false);
        }
    }

    fn settlement_equal_labels(
        categories: &[Category],
        label: &str,
        include_subcategories: bool,
    ) -> Vec<String> {
        let parents = category_parent_map(categories);
        let Some(category) = find_category_by_label(categories, label) else {
            return vec![label.to_string()];
        };
        let mut labels = vec![category.full_label(&parents)];
        if include_subcategories {
            let mut ids = vec![category.id];
            let mut index = 0;
            while index < ids.len() {
                let parent_id = ids[index];
                for child in categories
                    .iter()
                    .filter(|child| child.parent_id == Some(parent_id))
                {
                    if !ids.contains(&child.id) {
                        labels.push(child.full_label(&parents));
                        ids.push(child.id);
                    }
                }
                index += 1;
            }
        }
        labels
    }

    fn apply_settlement_equal_split(&mut self, label: String, include_subcategories: bool) {
        let categories = self.categories.clone();
        let members = self.members.clone();
        let labels = Self::settlement_equal_labels(&categories, &label, include_subcategories);
        let each = 100.0 / members.len().max(1) as f64;
        let household_id = self.household_id;
        let result = self.run_settlement_mutation(
            |tx| {
                for category in &labels {
                    for member in &members {
                        save_category_split(tx, household_id, category, &member.name, each)?;
                    }
                }
                Ok(())
            },
            |before, after| UndoAction::SettlementEqualSplit { before, after },
        );
        match result {
            Ok(true) => self.set_transient_status("Equal split saved."),
            Ok(false) => {}
            Err(error) => self.set_transient_status(error),
        }
    }

    pub(crate) fn clear_settlement_splits_with_undo(
        &mut self,
        label: &str,
        include_subcategories: bool,
    ) -> Result<(), String> {
        let categories = self.categories.clone();
        let labels = Self::settlement_equal_labels(&categories, label, include_subcategories);
        let household_id = self.household_id;
        let result = self.run_settlement_mutation(
            |tx| {
                for category in &labels {
                    delete_category_splits_for(tx, household_id, category)?;
                }
                Ok(())
            },
            |before, after| UndoAction::SettlementClear { before, after },
        )?;
        if result {
            self.set_transient_status("Settlement splits cleared.");
        }
        Ok(())
    }

    pub(crate) fn commit_pending_settlement_edits(&mut self) {
        let pending = std::mem::take(&mut self.settlement_pending_edits);
        let mut failed = Vec::new();
        let mut changed = false;
        let mut error = None;
        for edit in pending {
            let household_id = self.household_id;
            let category = edit.category.clone();
            let member = edit.member_name.clone();
            let before = edit.before;
            let value = edit.value;
            let result = self.run_settlement_mutation(
                |tx| {
                    let current = tx
                        .query_row(
                            "SELECT percentage FROM category_splits WHERE household_id = ?1 AND category = ?2 AND member_name = ?3",
                            rusqlite::params![household_id, category, member],
                            |row| row.get::<_, f64>(0),
                        )
                        .optional()?;
                    if current != before {
                        return Err(rusqlite::Error::InvalidParameterName(
                            "settlement edit changed before commit".into(),
                        ));
                    }
                    save_category_split(tx, household_id, &category, &member, value)
                },
                |before, after| UndoAction::SettlementPercentage { before, after },
            );
            match result {
                Ok(changed_now) => changed |= changed_now,
                Err(message) => {
                    error.get_or_insert(message);
                    failed.push(edit);
                }
            }
        }
        self.settlement_pending_edits = failed;
        if let Some(message) = error {
            self.set_transient_status(message);
        } else if changed {
            self.set_transient_status("Split updated.");
        }
    }
}

/// Center a compact widget (auto-sized) both axes inside a fixed cell rect.
fn centered_cell<R>(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
        |ui| {
            let widget_h = ui.spacing().interact_size.y;
            ui.add_space(((rect.height() - widget_h) / 2.0).max(0.0));
            add(ui)
        },
    )
    .inner
}

#[allow(clippy::too_many_arguments)]
fn render_category_block(
    ui: &mut egui::Ui,
    block_w: f32,
    parent: &BlockRow,
    rows: &[BlockRow],
    has_subs: bool,
    members: &[HouseholdMember],
    splits: &[CategorySplit],
    pending_edits: &mut Vec<PendingSettlementEdit>,
    commit_edit: &mut bool,
    clear_target: &mut Option<(String, bool)>,
    apply_parent_to_subs: &mut Option<String>,
    apply_equal_split: &mut Option<String>,
) {
    let sep_color = crate::ui::theme::border();
    let faint_bg = crate::ui::theme::bg_secondary();
    let extreme_bg = crate::ui::theme::bg_primary();
    let num_members = members.len();
    // Row content spans block_w - 2 (1px border each side). The button column
    // is anchored to the right edge with BTN_PAD clearance, so the clear button
    // never sits on the border; member columns take the remainder (no floor —
    // a floor here is what used to push the buttons past the border).
    let row_w = block_w - 2.0;
    let member_col_w = ((row_w - NAME_COL_W - BTN_COL_W - BTN_PAD) / num_members as f32).max(1.0);

    // Header row lives inside each block so member columns stay aligned per block.
    // Fixed block height — short blocks leave empty space, tall ones scroll inside.
    ui.allocate_ui_with_layout(
        egui::vec2(block_w, BLOCK_H),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            let block_rect = ui.max_rect();

            let left0 = block_rect.left() + 1.0;
            let top0 = block_rect.top() + 1.0;

            // --- Header row IS the parent category row: swatch + name + member
            // names + parent-level =/× buttons (only when the block has subs). ---
            let hdr_y = top0;
            let hdr_rect =
                egui::Rect::from_min_size(egui::pos2(left0, hdr_y), egui::vec2(row_w, ROW_H));
            let tint = Color32::from_rgba_unmultiplied(
                parent.color.r(),
                parent.color.g(),
                parent.color.b(),
                35,
            );
            ui.painter().rect_filled(hdr_rect, 0.0, tint);
            // Underline is painted AFTER the data ScrollArea below: scrolled row
            // backgrounds bleed a few px above the scroll clip (clip_rect_margin)
            // and were covering this line whenever the block had a scrollbar.

            let swatch_center = egui::pos2(left0 + 8.0 + SWATCH_R, hdr_y + ROW_H / 2.0);
            ui.painter()
                .circle_filled(swatch_center, SWATCH_R, parent.color);
            ui.painter()
                .circle_stroke(swatch_center, SWATCH_R, Stroke::new(1.0_f32, sep_color));

            let name_x = left0 + 8.0 + SWATCH_R * 2.0 + 4.0;
            clip_text(
                ui,
                hdr_rect,
                egui::pos2(name_x, hdr_y + ROW_H / 2.0),
                egui::Align2::LEFT_CENTER,
                parent.display_name,
                egui::FontId::proportional(13.0),
                crate::ui::theme::fg_primary(),
            );

            for (mi, m) in members.iter().enumerate() {
                let x = left0 + NAME_COL_W + mi as f32 * member_col_w + member_col_w / 2.0;
                clip_text(
                    ui,
                    hdr_rect,
                    egui::pos2(x, hdr_y + ROW_H / 2.0),
                    egui::Align2::CENTER_CENTER,
                    &m.name,
                    egui::FontId::proportional(11.0),
                    m.color,
                );
            }

            if has_subs {
                let btn_x = left0 + row_w - BTN_PAD - BTN_COL_W;
                let half_w = (BTN_COL_W - 4.0) / 2.0;
                let btn1_rect =
                    egui::Rect::from_min_size(egui::pos2(btn_x, hdr_y), egui::vec2(half_w, ROW_H));
                let btn2_rect = egui::Rect::from_min_size(
                    egui::pos2(btn_x + half_w + 4.0, hdr_y),
                    egui::vec2(half_w, ROW_H),
                );
                let parent_label = parent.category_label.clone();
                centered_cell(ui, btn1_rect, |ui| {
                    if ui
                        .small_button("=")
                        .on_hover_text("Equal split all subcategories")
                        .clicked()
                    {
                        *apply_parent_to_subs = Some(parent_label.clone());
                    }
                });
                centered_cell(ui, btn2_rect, |ui| {
                    if ui
                        .small_button("×")
                        .on_hover_text("Clear parent + all subcategories")
                        .clicked()
                    {
                        *clear_target = Some((parent_label, true));
                    }
                });
            }

            // Advance the cursor past the pinned header so the scroll area starts below it.
            ui.allocate_exact_size(egui::vec2(block_w, ROW_H + 2.0), egui::Sense::hover());

            // Data rows in an internal scroll: overflow scrolls, short blocks leave empty space.
            let inner_h = BLOCK_H - ROW_H - 3.0;
            egui::ScrollArea::vertical()
                .id_salt(("settlements_block_scroll", parent.category_label.as_str()))
                .auto_shrink([false, false])
                .max_height(inner_h)
                .show(ui, |ui| {
                    // Content origin — moves with scroll offset, so all rows stay in sync.
                    let origin = ui.cursor().left_top();
                    let left0 = origin.x + 1.0;

                    for (row_idx, row) in rows.iter().enumerate() {
                        let y0 = origin.y + row_idx as f32 * ROW_H;
                        let row_rect = egui::Rect::from_min_size(
                            egui::pos2(left0, y0),
                            egui::vec2(row_w, ROW_H),
                        );

                        // Row background
                        if row.is_parent {
                            let tint = Color32::from_rgba_unmultiplied(
                                row.color.r(),
                                row.color.g(),
                                row.color.b(),
                                35,
                            );
                            ui.painter().rect_filled(row_rect, 0.0, tint);
                            // Bottom separator for parent
                            ui.painter().line_segment(
                                [
                                    egui::pos2(left0, y0 + ROW_H),
                                    egui::pos2(left0 + row_w, y0 + ROW_H),
                                ],
                                Stroke::new(1.0_f32, sep_color),
                            );
                        } else {
                            let bg = if row_idx % 2 == 0 {
                                extreme_bg
                            } else {
                                faint_bg
                            };
                            ui.painter().rect_filled(row_rect, 0.0, bg);
                        }

                        // Color swatch
                        let swatch_x = left0 + 8.0 + if row.is_parent { 0.0 } else { INDENT };
                        let swatch_center = egui::pos2(swatch_x + SWATCH_R, y0 + ROW_H / 2.0);
                        ui.painter()
                            .circle_filled(swatch_center, SWATCH_R, row.color);
                        ui.painter().circle_stroke(
                            swatch_center,
                            SWATCH_R,
                            Stroke::new(1.0_f32, sep_color),
                        );

                        // Category name
                        let name_x = swatch_x + SWATCH_R * 2.0 + 4.0;
                        let font_size = if row.is_parent { 13.0 } else { 12.0 };
                        let name_color = if row.is_parent {
                            crate::ui::theme::fg_primary()
                        } else {
                            crate::ui::theme::fg_secondary()
                        };
                        let font_id = egui::FontId::proportional(font_size);

                        let name_rect = egui::Rect::from_min_size(
                            egui::pos2(name_x, y0),
                            egui::vec2(NAME_COL_W - INDENT - SWATCH_R * 2.0 - 8.0, ROW_H),
                        );
                        clip_text(
                            ui,
                            name_rect,
                            egui::pos2(name_x, y0 + ROW_H / 2.0),
                            egui::Align2::LEFT_CENTER,
                            row.display_name,
                            font_id,
                            name_color,
                        );

                        // Member DragValue cells — compact, centered both axes in the column.
                        let cat_label = &row.category_label;
                        let cat_splits: Vec<&CategorySplit> =
                            splits.iter().filter(|s| s.category == *cat_label).collect();

                        for (mi, m) in members.iter().enumerate() {
                            let cell_x = left0 + NAME_COL_W + mi as f32 * member_col_w;
                            let cell_rect = egui::Rect::from_min_size(
                                egui::pos2(cell_x, y0),
                                egui::vec2(member_col_w, ROW_H),
                            );

                            let current = cat_splits
                                .iter()
                                .find(|s| s.member_name == m.name)
                                .map(|s| s.percentage);
                            let mut pct = pending_edits
                                .iter()
                                .find(|pending| pending.matches(cat_label, &m.name))
                                .map(|pending| pending.value)
                                .or(current)
                                .unwrap_or(0.0);

                            centered_cell(ui, cell_rect, |ui| {
                                let response = ui.add(
                                    egui::DragValue::new(&mut pct)
                                        .speed(1.0)
                                        .range(0.0..=100.0)
                                        .suffix("%")
                                        .max_decimals(0),
                                );
                                if response.changed() {
                                    if let Some(pending) = pending_edits
                                        .iter_mut()
                                        .find(|pending| pending.matches(cat_label, &m.name))
                                    {
                                        pending.value = pct;
                                    } else {
                                        pending_edits.push(PendingSettlementEdit {
                                            category: cat_label.to_string(),
                                            member_name: m.name.clone(),
                                            before: current,
                                            value: pct,
                                        });
                                    }
                                }
                                if response.drag_stopped()
                                    || response.lost_focus()
                                    || (response.has_focus()
                                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                                {
                                    if pending_edits
                                        .iter()
                                        .any(|pending| pending.matches(cat_label, &m.name))
                                    {
                                        *commit_edit = true;
                                    }
                                }
                                if (response.has_focus() || response.dragged())
                                    && ui.input(|input| input.key_pressed(egui::Key::Escape))
                                {
                                    pending_edits
                                        .retain(|pending| !pending.matches(cat_label, &m.name));
                                }
                            });
                        }

                        // Buttons — every data row gets "=" (equal split) and "×" (clear),
                        // compact and centered, anchored right with BTN_PAD clearance.
                        let btn_x = left0 + row_w - BTN_PAD - BTN_COL_W;
                        let half_w = (BTN_COL_W - 4.0) / 2.0;
                        let btn1_rect = egui::Rect::from_min_size(
                            egui::pos2(btn_x, y0),
                            egui::vec2(half_w, ROW_H),
                        );
                        let btn2_rect = egui::Rect::from_min_size(
                            egui::pos2(btn_x + half_w + 4.0, y0),
                            egui::vec2(half_w, ROW_H),
                        );
                        centered_cell(ui, btn1_rect, |ui| {
                            if ui.small_button("=").on_hover_text("Equal split").clicked() {
                                if row.is_parent {
                                    *apply_parent_to_subs = Some(cat_label.clone());
                                } else {
                                    *apply_equal_split = Some(cat_label.clone());
                                }
                            }
                        });
                        centered_cell(ui, btn2_rect, |ui| {
                            if ui.small_button("×").on_hover_text("Clear splits").clicked() {
                                *clear_target = Some((cat_label.clone(), row.is_parent));
                            }
                        });
                    }
                });

            // Block border — painted after the data ScrollArea so scrolled row
            // backgrounds (which bleed up to clip_rect_margin past the scroll
            // clip) can't cover the bottom edge when the block has a scrollbar.
            ui.painter().rect_stroke(
                block_rect,
                4.0,
                Stroke::new(1.0_f32, sep_color),
                egui::StrokeKind::Inside,
            );

            // Parent header underline — painted after the ScrollArea so scrolled
            // row backgrounds (which can bleed up to clip_rect_margin above the
            // scroll clip) can't cover it when the block has a scrollbar.
            ui.painter().line_segment(
                [
                    egui::pos2(left0, hdr_y + ROW_H),
                    egui::pos2(left0 + row_w, hdr_y + ROW_H),
                ],
                Stroke::new(1.0_f32, sep_color),
            );
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expense(date: &str, amount_cents: i64, member: &str, category: &str) -> Expense {
        Expense {
            id: 0,
            date: date.into(),
            amount_input: String::new(),
            amount_cents,
            member: member.into(),
            category: category.into(),
            vendor: String::new(),
            description: String::new(),
            account_id: 0,
            account: String::new(),
        }
    }

    fn shares() -> HashMap<String, Vec<(String, f64)>> {
        HashMap::from([(
            "Food > Groceries".to_string(),
            vec![("Me".to_string(), 50.0), ("Alex".to_string(), 50.0)],
        )])
    }

    #[test]
    fn settlement_totals_respect_the_date_range_and_exclusions() {
        let expenses = vec![
            expense("2026-09-10", -1000, "Me", "Food > Groceries"),
            expense("2026-09-20", -2000, "Alex", "Food > Groceries"),
            // out of range
            expense("2026-01-05", -9999, "Me", "Food > Groceries"),
            // income: never shared spend
            expense("2026-09-11", 5000, "Me", "Income > Salary"),
            // user-excluded transfer category
            expense(
                "2026-09-12",
                -3000,
                "Me",
                "Financial > Debt Repayment (Credit Cards)",
            ),
        ];
        let excluded: HashSet<String> = ["Financial > Debt Repayment (Credit Cards)".to_string()]
            .into_iter()
            .collect();
        let start = chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap();

        let totals = settlement_totals(&expenses, &shares(), &excluded, Some(start), None);

        assert_eq!(totals.paid.get("Me"), Some(&1000));
        assert_eq!(totals.paid.get("Alex"), Some(&2000));
        assert_eq!(totals.owed.get("Me"), Some(&1500));
        assert_eq!(totals.owed.get("Alex"), Some(&1500));
        assert_eq!(totals.categories.len(), 1);
        let category = &totals.categories[0];
        assert_eq!(category.category, "Food > Groceries");
        assert_eq!(category.spend_cents, 3000);
        assert_eq!(category.paid_by.get("Alex"), Some(&2000));
        assert_eq!(category.allocated.get("Me"), Some(&1500));

        // All Time keeps the same rows plus the older one.
        let all_time = settlement_totals(&expenses, &shares(), &excluded, None, None);
        assert_eq!(all_time.categories[0].spend_cents, 12999);
    }

    #[test]
    fn a_row_outside_the_range_leaves_every_total_at_zero() {
        let expenses = vec![expense("2026-09-10", -1000, "Me", "Food > Groceries")];
        let start = chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let end = chrono::NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();

        let totals = settlement_totals(
            &expenses,
            &shares(),
            &HashSet::new(),
            Some(start),
            Some(end),
        );

        assert!(totals.paid.is_empty());
        assert!(totals.categories.is_empty());
    }
}
