use eframe::egui::{self, Color32, RichText, Stroke};
use crate::models::*;
use crate::db::*;

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

fn clip_text(ui: &mut egui::Ui, cell: egui::Rect, p: egui::Pos2, a: egui::Align2, t: &str, f: egui::FontId, c: Color32) {
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

    // === Who Owes Whom (balances + suggested payments side by side) ===
    crate::ui::components::heading_lg(ui, "Who Owes Whom");
    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    // ponytail: settlements are about shared spending — debit rows only,
    // magnitude; income (credits) and excluded transfer/payment rows are
    // not household payments and never enter the math.
    let excluded = excluded_category_labels(&categories);
    let shared_spend = |exp: &Expense| -> i64 {
      if exp.amount_cents < 0 && !excluded.contains(&exp.category) { -exp.amount_cents } else { 0 }
    };

    // ponytail: Who Owes Whom counts ONLY split-covered categories — any
    // category with at least one non-zero allocation. No equal-split
    // fallback: it dragged every unassigned expense into the math (±$36k
    // with one split category configured). Categories whose splits are all
    // zero stay out too. A 100%-one-member split still counts when the
    // OTHER member paid the row (personal spend on the other's card).
    let mut split_shares: std::collections::HashMap<String, Vec<(&str, f64)>> = std::collections::HashMap::new();
    for split in &splits {
      if split.percentage != 0.0 {
        split_shares
          .entry(split.category.clone())
          .or_default()
          .push((split.member_name.as_str(), split.percentage));
      }
    }

    let mut paid: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut owed: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for exp in &expenses {
      let total = shared_spend(exp);
      if total == 0 {
        continue;
      }
      let Some(allocations) = split_shares.get(&exp.category) else {
        continue;
      };
      *paid.entry(exp.member.clone()).or_insert(0) += total;
      for (member_name, percentage) in allocations {
        let share = (total as f64 * percentage / 100.0).round() as i64;
        *owed.entry((*member_name).to_string()).or_insert(0) += share;
      }
    }

    if paid.values().all(|&value| value == 0) {
      crate::ui::components::label_muted(
        ui,
        "Balances only count categories with split percentages assigned below.",
      );
      ui.add_space(crate::ui::theme_tokens::SPACE_2);
    }

    let mut net: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for m in &members {
      let p = paid.get(&m.name).copied().unwrap_or(0);
      let o = owed.get(&m.name).copied().unwrap_or(0);
      net.insert(m.name.clone(), p - o);
    }

    let mut debtors: Vec<(String, i64)> = net.iter()
      .filter(|(_, &v)| v < 0)
      .map(|(k, &v)| (k.clone(), -v))
      .collect();
    let mut creditors: Vec<(String, i64)> = net.iter()
      .filter(|(_, &v)| v > 0)
      .map(|(k, v)| (k.clone(), *v))
      .collect();
    debtors.sort_by(|a, b| b.1.cmp(&a.1));
    creditors.sort_by(|a, b| b.1.cmp(&a.1));

    let two_or_more = members.len() >= 2;
    // ponytail: single column — balances, then suggested payments directly
    // under them (was a side-by-side columns(2) split).
    crate::ui::components::label_muted(ui, "Balances");
    if !two_or_more {
      crate::ui::components::label_muted(ui, "Add at least 2 household members to see settlements.");
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
          let (rect, _) = ui.allocate_exact_size(egui::vec2(SWATCH_R * 2.0, SWATCH_R * 2.0), egui::Sense::hover());
          ui.painter().circle_filled(rect.center(), SWATCH_R, m.color);
          // ponytail: egui hardwires strong() text to widgets.active's
          // fg_stroke (contrast-on-accent = near-black here), so strong
          // labels must carry an explicit palette color.
          ui.label(RichText::new(&m.name).strong().color(crate::ui::theme::fg_primary()));
          ui.label(
            RichText::new(format!("{}${}", sign, format_cents(balance)))
              .color(color),
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
          // ponytail: explicit fg_default — this line previously used egui's
          // default text stroke, which is also what muted backgrounds show;
          // in light variants it rendered washed-out (light-on-light).
          ui.label(
            RichText::new(format!("{} pays {} ${}", debtors[di].0, creditors[ci].0, format_cents(amount)))
              .small()
              .color(crate::ui::theme::fg_primary()),
          );
          emitted += 1;
        }
        debtors[di].1 -= amount;
        creditors[ci].1 -= amount;
        if debtors[di].1 == 0 { di += 1; }
        if creditors[ci].1 == 0 { ci += 1; }
      }
      if emitted == 0 {
        crate::ui::components::label_muted(ui, "All settled — no payments needed.");
      }
    }

    ui.add_space(crate::ui::theme_tokens::SPACE_2);

    let parent_ids = sorted_parent_category_ids(&categories);
    let num_members = members.len();

    // Collect deferred actions
    let mut save_target: Option<(String, String, f64)> = None;
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
        let min_block_w = NAME_COL_W + BTN_COL_W + BTN_PAD + num_members as f32 * MIN_MEMBER_COL_W;
        let cols = ((w + BLOCK_GAP) / (min_block_w + BLOCK_GAP)).floor().max(1.0) as usize;
        let block_w = (w - BLOCK_GAP * (cols - 1) as f32) / cols as f32;

// Build blocks: header (parent) + data rows (subs; the parent itself when childless)
        // ponytail: excluded categories (user-flagged) get no block at all —
        // their rows never enter the math either (shared_spend filters them).
        let excluded = excluded_category_labels(&categories);
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
            categories.iter().find(|c| c.id == *id).map(|c| c.name.to_lowercase()).unwrap_or_default()
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
                  &mut save_target,
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

    // Apply deferred actions
    if let Some((cat, member, pct)) = save_target {
      let _ = save_category_split(&self.conn, self.household_id, &cat, &member, pct);
      self.category_splits = load_category_splits(&self.conn, self.household_id).unwrap_or_default();
    }
    if let Some((label, include_subs)) = clear_target {
      // Parent "0" clears the parent and all its subcategory splits;
      // sub "0" clears just that subcategory.
      let _ = delete_category_splits_for(&self.conn, self.household_id, &label);
      if include_subs {
        if let Some(pid) = categories.iter().find(|c| c.name == label).map(|c| c.id) {
          for sub in categories.iter().filter(|c| c.parent_id == Some(pid)) {
            let sub_label = sub.full_label(&parents);
            let _ = delete_category_splits_for(&self.conn, self.household_id, &sub_label);
          }
        }
      }
      self.category_splits = load_category_splits(&self.conn, self.household_id).unwrap_or_default();
    }
    if let Some(parent_label) = apply_parent_to_subs {
      let n = members.len().max(1) as f64;
      let each = 100.0 / n;
      for m in &members {
        let _ = save_category_split(&self.conn, self.household_id, &parent_label, &m.name, each);
      }
      let parent_id = categories.iter().find(|c| c.name == parent_label).map(|c| c.id);
      if let Some(pid) = parent_id {
        for sub in categories.iter().filter(|c| c.parent_id == Some(pid)) {
          let sub_label = sub.full_label(&parents);
          for m in &members {
            let _ = save_category_split(&self.conn, self.household_id, &sub_label, &m.name, each);
          }
        }
      }
      self.category_splits = load_category_splits(&self.conn, self.household_id).unwrap_or_default();
    }
    if let Some(cat_label) = apply_equal_split {
      let n = members.len().max(1) as f64;
      let each = 100.0 / n;
      for m in &members {
        let _ = save_category_split(&self.conn, self.household_id, &cat_label, &m.name, each);
      }
      self.category_splits = load_category_splits(&self.conn, self.household_id).unwrap_or_default();
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
  ).inner
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
  save_target: &mut Option<(String, String, f64)>,
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
      let hdr_rect = egui::Rect::from_min_size(egui::pos2(left0, hdr_y), egui::vec2(row_w, ROW_H));
      let tint = Color32::from_rgba_unmultiplied(parent.color.r(), parent.color.g(), parent.color.b(), 35);
      ui.painter().rect_filled(hdr_rect, 0.0, tint);
      // Underline is painted AFTER the data ScrollArea below: scrolled row
      // backgrounds bleed a few px above the scroll clip (clip_rect_margin)
      // and were covering this line whenever the block had a scrollbar.

      let swatch_center = egui::pos2(left0 + 8.0 + SWATCH_R, hdr_y + ROW_H / 2.0);
      ui.painter().circle_filled(swatch_center, SWATCH_R, parent.color);
      ui.painter().circle_stroke(swatch_center, SWATCH_R, Stroke::new(1.0_f32, sep_color));

      let name_x = left0 + 8.0 + SWATCH_R * 2.0 + 4.0;
      clip_text(
        ui, hdr_rect,
        egui::pos2(name_x, hdr_y + ROW_H / 2.0),
        egui::Align2::LEFT_CENTER,
        parent.display_name,
        egui::FontId::proportional(13.0),
        crate::ui::theme::fg_primary(),
      );

      for (mi, m) in members.iter().enumerate() {
        let x = left0 + NAME_COL_W + mi as f32 * member_col_w + member_col_w / 2.0;
        clip_text(
          ui, hdr_rect,
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
        let btn1_rect = egui::Rect::from_min_size(egui::pos2(btn_x, hdr_y), egui::vec2(half_w, ROW_H));
        let btn2_rect = egui::Rect::from_min_size(egui::pos2(btn_x + half_w + 4.0, hdr_y), egui::vec2(half_w, ROW_H));
        let parent_label = parent.category_label.clone();
        centered_cell(ui, btn1_rect, |ui| {
          if ui.small_button("=").on_hover_text("Equal split all subcategories").clicked() {
            *apply_parent_to_subs = Some(parent_label.clone());
          }
        });
        centered_cell(ui, btn2_rect, |ui| {
          if ui.small_button("×").on_hover_text("Clear parent + all subcategories").clicked() {
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
            let row_rect = egui::Rect::from_min_size(egui::pos2(left0, y0), egui::vec2(row_w, ROW_H));

            // Row background
            if row.is_parent {
              let tint = Color32::from_rgba_unmultiplied(row.color.r(), row.color.g(), row.color.b(), 35);
              ui.painter().rect_filled(row_rect, 0.0, tint);
              // Bottom separator for parent
              ui.painter().line_segment(
                [egui::pos2(left0, y0 + ROW_H), egui::pos2(left0 + row_w, y0 + ROW_H)],
                Stroke::new(1.0_f32, sep_color),
              );
            } else {
              let bg = if row_idx % 2 == 0 { extreme_bg } else { faint_bg };
              ui.painter().rect_filled(row_rect, 0.0, bg);
            }

            // Color swatch
            let swatch_x = left0 + 8.0 + if row.is_parent { 0.0 } else { INDENT };
            let swatch_center = egui::pos2(swatch_x + SWATCH_R, y0 + ROW_H / 2.0);
            ui.painter().circle_filled(swatch_center, SWATCH_R, row.color);
            ui.painter().circle_stroke(swatch_center, SWATCH_R, Stroke::new(1.0_f32, sep_color));

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
              ui, name_rect,
              egui::pos2(name_x, y0 + ROW_H / 2.0),
              egui::Align2::LEFT_CENTER,
              row.display_name,
              font_id,
              name_color,
            );

            // Member DragValue cells — compact, centered both axes in the column.
            let cat_label = &row.category_label;
            let cat_splits: Vec<&CategorySplit> = splits.iter().filter(|s| s.category == *cat_label).collect();

            for (mi, m) in members.iter().enumerate() {
              let cell_x = left0 + NAME_COL_W + mi as f32 * member_col_w;
              let cell_rect = egui::Rect::from_min_size(egui::pos2(cell_x, y0), egui::vec2(member_col_w, ROW_H));

              let current = cat_splits.iter()
                .find(|s| s.member_name == m.name)
                .map(|s| s.percentage)
                .unwrap_or(0.0);
              let mut pct = current;

              centered_cell(ui, cell_rect, |ui| {
                let response = ui.add(
                  egui::DragValue::new(&mut pct)
                    .speed(1.0)
                    .range(0.0..=100.0)
                    .suffix("%")
                    .max_decimals(0),
                );
                if response.changed() {
                  *save_target = Some((cat_label.to_string(), m.name.clone(), pct));
                }
              });
            }

            // Buttons — every data row gets "=" (equal split) and "×" (clear),
            // compact and centered, anchored right with BTN_PAD clearance.
            let btn_x = left0 + row_w - BTN_PAD - BTN_COL_W;
            let half_w = (BTN_COL_W - 4.0) / 2.0;
            let btn1_rect = egui::Rect::from_min_size(egui::pos2(btn_x, y0), egui::vec2(half_w, ROW_H));
            let btn2_rect = egui::Rect::from_min_size(egui::pos2(btn_x + half_w + 4.0, y0), egui::vec2(half_w, ROW_H));
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
      ui.painter().rect_stroke(block_rect, 4.0, Stroke::new(1.0_f32, sep_color), egui::StrokeKind::Inside);

      // Parent header underline — painted after the ScrollArea so scrolled
      // row backgrounds (which can bleed up to clip_rect_margin above the
      // scroll clip) can't cover it when the block has a scrollbar.
      ui.painter().line_segment(
        [egui::pos2(left0, hdr_y + ROW_H), egui::pos2(left0 + row_w, hdr_y + ROW_H)],
        Stroke::new(1.0_f32, sep_color),
      );
    },
  );
}
