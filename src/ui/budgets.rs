use chrono::Datelike;
use eframe::egui::{self, Color32, RichText, Stroke};
use crate::models::*;
use crate::db::*;
use crate::ui::theme;


fn clip_text(ui: &mut egui::Ui, cell: egui::Rect, p: egui::Pos2, a: egui::Align2, t: &str, f: egui::FontId, c: Color32) {
  let old = ui.clip_rect();
  ui.set_clip_rect(cell.intersect(old));
  ui.painter().text(p, a, t, f, c);
  ui.set_clip_rect(old);
}
use crate::TwoCentsApp;


fn week_range(year: i32, week: u32) -> (chrono::NaiveDate, chrono::NaiveDate) {
  let w = week.clamp(1, 53);
  let first_day = chrono::NaiveDate::from_isoywd_opt(year, w, chrono::Weekday::Mon)
    .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year, 1, 1).unwrap());
  let last_day = first_day + chrono::Duration::days(6);
  (first_day, last_day)
}




impl TwoCentsApp {
  pub fn ui_budgets(&mut self, ui: &mut egui::Ui) {
    ui.heading("Budgets");
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
    if changed { self.reload(); }
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
      
      let prev_clicked = ui.button(RichText::new("‹").size(16.0)).clicked();
      
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
            "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"
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
        ui.visuals().text_color(),
      );
      
      let next_clicked = ui.button(RichText::new("›").size(16.0)).clicked();
      let snap_clicked = ui.button(RichText::new("◎").size(16.0))
        .on_hover_text("Snap to current period")
        .clicked();
      
      if prev_clicked {
        match self.budget_granularity {
          BudgetGranularity::Weekly => {
            if self.budget_week == 1 {
              self.budget_week = 52;
              self.budget_year -= 1;
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
            if self.budget_week == 52 {
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
        self.budget_week = now.iso_week().week();
        self.budget_editing_category = None;
        self.reload();
      }

      ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // Copy allocations from the previous period (Weekly, Monthly, Quarterly)
        let (prev_val, prev_year) = match self.budget_granularity {
          BudgetGranularity::Weekly => {
            let pw = if self.budget_week == 1 { 52 } else { self.budget_week - 1 };
            let py = if self.budget_week == 1 { self.budget_year - 1 } else { self.budget_year };
            (pw as i32 + 100, py)
          }
          BudgetGranularity::Monthly => {
            let pm = if self.budget_month == 1 { 12 } else { self.budget_month - 1 };
            let py = if self.budget_month == 1 { self.budget_year - 1 } else { self.budget_year };
            (pm, py)
          }
          BudgetGranularity::Quarterly => {
            let pq = if self.budget_quarter == 1 { 4 } else { self.budget_quarter - 1 };
            let py = if self.budget_quarter == 1 { self.budget_year - 1 } else { self.budget_year };
            (pq as i32 + 20, py)
          }
          BudgetGranularity::Yearly => {
            (0, self.budget_year - 1)
          }
        };

        let copy_label = match self.budget_granularity {
          BudgetGranularity::Weekly => format!("Copy Limits from W{}", prev_val - 100),
          BudgetGranularity::Monthly => format!("Copy Limits from {:02}/{}", prev_val, prev_year),
          BudgetGranularity::Quarterly => format!("Copy Limits from Q{}", prev_val - 20),
          BudgetGranularity::Yearly => format!("Copy Limits from {}", prev_year),
        };
        
        if ui.button(copy_label).on_hover_text("Copy all budget allocations from the previous period to the current period.").clicked() {
          let mut copied_count = 0;

          // Primary: copy yearly caps from previous year's snapshots
          if let Ok(prev_snaps) = load_budget_snapshots_for_year(&self.conn, self.household_id, prev_year) {
            for snap in &prev_snaps {
              if snap.period_code == 0 && snap.is_override && snap.amount_cents > 0 {
                let _ = save_budget_snapshot(
                  &self.conn, self.household_id, &snap.category,
                  self.budget_year, 0, snap.amount_cents, true,
                );
                self.propagate_yearly_to_others(&snap.category, snap.amount_cents);
                copied_count += 1;
              }
            }
          }

          // Fallback: legacy budgets table (pre-snapshot data)
          if copied_count == 0 {
            if let Ok(prev_budgets) = load_budgets(&self.conn, self.household_id, prev_year, prev_val) {
              let current_period_code = match self.budget_granularity {
                BudgetGranularity::Yearly => 0,
                BudgetGranularity::Monthly => self.budget_month,
                BudgetGranularity::Quarterly => 20 + self.budget_quarter as i32,
                BudgetGranularity::Weekly => 100 + self.budget_week as i32,
              };
              for pb in prev_budgets {
                if let Ok(_) = save_budget(&self.conn, self.household_id, &pb.category, pb.amount_cents, self.budget_year, current_period_code) {
                  copied_count += 1;
                }
              }
              self.log(format!("[budgets] copied {copied_count} limits from previous period (legacy)"));
            }
          } else {
            self.log(format!("[budgets] carried forward {copied_count} budget(s) from {prev_year} to {}", self.budget_year));
          }

          self.reload();
        }
      });
    });

    ui.add_space(10.0);

    // 3. Compute Actual Spent per category dynamically based on active timeframe
    let assignable_categories = category_assignable_labels(&self.categories);
    let mut spent_map = std::collections::HashMap::new();

    match self.budget_granularity {
      BudgetGranularity::Weekly => {
        let (start_date, end_date) = week_range(self.budget_year, self.budget_week);
        for exp in &self.expenses {
          if let Ok(exp_date) = chrono::NaiveDate::parse_from_str(&exp.date, "%Y-%m-%d") {
            if exp_date >= start_date && exp_date <= end_date {
              *spent_map.entry(exp.category.clone()).or_insert(0) += exp.amount_cents;
            }
          }
        }
      }
      BudgetGranularity::Monthly => {
        let prefix = format!("{}-{:02}", self.budget_year, self.budget_month);
        for exp in &self.expenses {
          if exp.date.starts_with(&prefix) {
            *spent_map.entry(exp.category.clone()).or_insert(0) += exp.amount_cents;
          }
        }
      }
      BudgetGranularity::Quarterly => {
        let prefix1 = format!("{}-{:02}", self.budget_year, (self.budget_quarter - 1) * 3 + 1);
        let prefix2 = format!("{}-{:02}", self.budget_year, (self.budget_quarter - 1) * 3 + 2);
        let prefix3 = format!("{}-{:02}", self.budget_year, (self.budget_quarter - 1) * 3 + 3);
        for exp in &self.expenses {
          if exp.date.starts_with(&prefix1) || exp.date.starts_with(&prefix2) || exp.date.starts_with(&prefix3) {
            *spent_map.entry(exp.category.clone()).or_insert(0) += exp.amount_cents;
          }
        }
      }
      BudgetGranularity::Yearly => {
        let prefix = format!("{}-", self.budget_year);
        for exp in &self.expenses {
          if exp.date.starts_with(&prefix) {
            *spent_map.entry(exp.category.clone()).or_insert(0) += exp.amount_cents;
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

    // Render cards
    let card_fill = ui.visuals().widgets.noninteractive.bg_fill;
    let card_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    let text_col = ui.visuals().text_color();
    let success_col = theme::current_success();
    let error_col = theme::current_error();
    let total_w = ui.available_width();
    let card_content_w = ((total_w - 72.0) / 3.0).max(20.0); // 72 = 3 frames × 24px inner margin
    ui.horizontal(|ui| {
      ui.spacing_mut().item_spacing.x = 0.0;
      // Total Budgeted Card
      egui::Frame::default()
        .fill(card_fill)
        .stroke(card_stroke)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
          ui.set_width(card_content_w);
          ui.vertical(|ui| {
            ui.label(RichText::new("TOTAL ALLOCATED").size(10.0).weak());
            ui.add_space(4.0);
            ui.heading(RichText::new(money(total_budgeted)).color(text_col));
          });
        });

      // Total Spent Card
      egui::Frame::default()
        .fill(card_fill)
        .stroke(card_stroke)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
          ui.set_width(card_content_w);
          ui.vertical(|ui| {
            ui.label(RichText::new("TOTAL SPENT").size(10.0).weak());
            ui.add_space(4.0);
            ui.heading(RichText::new(money(total_spent)).color(text_col));
          });
        });

      // Remaining Card
      egui::Frame::default()
        .fill(card_fill)
        .stroke(card_stroke)
        .corner_radius(8.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
          ui.set_width(card_content_w);
          ui.vertical(|ui| {
            ui.label(RichText::new("REMAINING OVERALL").size(10.0).weak());
            ui.add_space(4.0);
            let color = if remaining_overall >= 0 { success_col } else { error_col };
            ui.heading(RichText::new(money(remaining_overall)).color(color));
          });
        });
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
        let button = ui.selectable_label(selected, crate::ui::theme::sel_text(ui, selected, label));
        if button.clicked() {
          self.budget_filter = filter;
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
    for &pid in &parent_ids {
      let Some(parent) = self.categories.iter().find(|c| c.id == pid) else { continue; };
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
        let mut child_ids: Vec<i64> = self.categories.iter()
          .filter(|c| c.parent_id == Some(pid))
          .map(|c| c.id)
          .collect();
        child_ids.sort_by(|a, b| {
          let na = self.categories.iter().find(|c| c.id == *a).map(|c| c.name.as_str()).unwrap_or("");
          let nb = self.categories.iter().find(|c| c.id == *b).map(|c| c.name.as_str()).unwrap_or("");
          na.to_lowercase().cmp(&nb.to_lowercase())
        });
        for &cid in &child_ids {
          let Some(child) = self.categories.iter().find(|c| c.id == cid) else { continue; };
          let full_label = child.full_label(&cat_parents);
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
      ui.add_space(20.0);
      ui.centered_and_justified(|ui| {
        ui.label(RichText::new("No categories match the active filter.").weak().italics());
      });
      return;
    }

    // Table
    let sep_color = ui.visuals().widgets.noninteractive.bg_stroke.color;
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

    // Paint a background rect + outer stroke for the entire table area
    let table_tl = ui.cursor().left_top();
    let table_avail_h = ui.available_height().max(120.0);
    let table_rect = egui::Rect::from_min_size(table_tl, egui::vec2(avail, table_avail_h));
    ui.painter().rect_filled(table_rect, 8.0, ui.visuals().extreme_bg_color);
    ui.painter().rect_stroke(table_rect, 8.0, Stroke::new(1.0, sep_color), egui::StrokeKind::Inside);

    let left0 = table_rect.left() + 2.0;

    let col_x = |i: usize| -> f32 {
      left0 + col_w[..i].iter().sum::<f32>()
    };

    let divider_xs: Vec<f32> = col_w.iter().scan(left0, |acc, &w| { *acc += w; Some(*acc) }).collect();

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
        let prev = if i > 0 { self.budget_col_dividers[i - 1] } else { 0.0 };
        let next = if i < 3 { self.budget_col_dividers[i + 1] } else { 1.0 };
        self.budget_col_dividers[i] = new_frac.clamp(prev + MIN_COL_PCT, next - MIN_COL_PCT);
        ui.ctx().request_repaint();
      }
    }

    let paint_vseps = |ui: &egui::Ui, y0: f32, y1: f32| {
      for &x in &divider_xs[..divider_xs.len() - 1] {
        ui.painter().line_segment(
          [egui::pos2(x, y0), egui::pos2(x, y1)],
          Stroke::new(1.0, sep_color),
        );
      }
    };

    // ---- Header row (fixed) ----
    let hdr_top = table_rect.top() + 1.0;
    let hdr_bg = ui.visuals().widgets.noninteractive.bg_fill.gamma_multiply(0.5);
    let hdr_rect = egui::Rect::from_min_size(egui::pos2(left0, hdr_top), egui::vec2(cw, row_h));
    ui.painter().rect_filled(hdr_rect, 0.0, hdr_bg);
    ui.painter().line_segment(
      [egui::pos2(left0, hdr_top + row_h), egui::pos2(left0 + cw, hdr_top + row_h)],
      Stroke::new(1.5, sep_color),
    );

    let hdr_labels = ["Category", "Allocated Limit", "Actual Spending", "Remaining", "Progress"];
    for (i, label) in hdr_labels.iter().enumerate() {
      let cx = col_x(i) + 8.0;
      ui.painter().text(
        egui::pos2(cx, hdr_top + row_h / 2.0),
        egui::Align2::LEFT_CENTER,
        *label,
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
      );
    }
    paint_vseps(ui, hdr_top, hdr_top + row_h);

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
            let past_period = if self.budget_granularity != BudgetGranularity::Yearly {
              let period = match self.budget_granularity {
                BudgetGranularity::Monthly => self.budget_month,
                BudgetGranularity::Quarterly => self.budget_quarter as i32,
                BudgetGranularity::Weekly => self.budget_week as i32,
                _ => unreachable!(),
              };
              self.is_period_past(self.budget_granularity, period)
            } else {
              false
            };
            while i < rows.len() {
              let cat = &rows[i];
              let y0 = ui.cursor().top();

              if !cat.is_child && i + 1 < rows.len() && rows[i + 1].is_child && rows[i + 1].parent_name == cat.parent_name {
                // ===== Parent header row (with summed children, tinted bg) =====
                let children: Vec<&CatRow> = rows[i+1..].iter().take_while(|r| r.is_child && r.parent_name == cat.parent_name).collect();
                let child_count = children.len();
                let limit_sum: i64 = children.iter().map(|c| c.limit).sum();
                let spent_sum: i64 = children.iter().map(|c| c.spent).sum();
                let rem_sum = limit_sum - spent_sum;
                let pct = if limit_sum > 0 { (spent_sum as f32 / limit_sum as f32).clamp(0.0, 2.0) } else if spent_sum > 0 { 1.5 } else { 0.0 };

                let cat_bg = Color32::from_rgba_unmultiplied(cat.color.r(), cat.color.g(), cat.color.b(), 35);
                let row_rect = egui::Rect::from_min_size(egui::pos2(left0, y0), egui::vec2(cw, row_h));
                ui.painter().rect_filled(row_rect, 0.0, cat_bg);
                ui.painter().line_segment(
                  [egui::pos2(left0, y0 + row_h), egui::pos2(left0 + cw, y0 + row_h)],
                  Stroke::new(1.5, sep_color),
                );

                let c0 = egui::Rect::from_min_size(egui::pos2(col_x(0), y0), egui::vec2(col_w[0], row_h));
                let c1 = egui::Rect::from_min_size(egui::pos2(col_x(1), y0), egui::vec2(col_w[1], row_h));
                let c2 = egui::Rect::from_min_size(egui::pos2(col_x(2), y0), egui::vec2(col_w[2], row_h));
                let c3 = egui::Rect::from_min_size(egui::pos2(col_x(3), y0), egui::vec2(col_w[3], row_h));

                let swatch_rect = egui::Rect::from_min_size(egui::pos2(left0 + 6.0, y0 + row_h / 2.0 - 5.0), egui::vec2(10.0, 10.0));
                ui.painter().circle_filled(swatch_rect.center(), 5.0, cat.color);
                clip_text(ui, c0, egui::pos2(left0 + 22.0, y0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &cat.parent_name, egui::FontId::proportional(13.0), ui.visuals().text_color());

                clip_text(ui, c1, egui::pos2(col_x(1) + 8.0, y0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &money(limit_sum), egui::FontId::monospace(13.0), ui.visuals().text_color());
                clip_text(ui, c2, egui::pos2(col_x(2) + 8.0, y0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &money(spent_sum), egui::FontId::monospace(13.0), ui.visuals().text_color());
                let rem_col = if rem_sum >= 0 { ui.visuals().text_color() } else { theme::current_error() };
                clip_text(ui, c3, egui::pos2(col_x(3) + 8.0, y0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &money(rem_sum), egui::FontId::monospace(13.0), rem_col);
                let bar_cell_w = col_w[4];
                let bar_rect = egui::Rect::from_min_size(egui::pos2(col_x(4), y0), egui::vec2(bar_cell_w, row_h));
                draw_progress_bar(ui, bar_rect, pct);

                paint_vseps(ui, y0, y0 + row_h);
                ui.allocate_space(egui::vec2(0.0, row_h));
                row_idx += 1;

                // ===== Child rows (indented, editable) =====
                for child in &rows[i+1..i+1+child_count] {
                  let cy0 = ui.cursor().top();
                  let c0c = egui::Rect::from_min_size(egui::pos2(col_x(0), cy0), egui::vec2(col_w[0], row_h));
                  let c2c = egui::Rect::from_min_size(egui::pos2(col_x(2), cy0), egui::vec2(col_w[2], row_h));
                  let c3c = egui::Rect::from_min_size(egui::pos2(col_x(3), cy0), egui::vec2(col_w[3], row_h));
                  let row_bg = if row_idx % 2 == 0 { ui.visuals().faint_bg_color } else { ui.visuals().extreme_bg_color };
                  ui.painter().rect_filled(
                    egui::Rect::from_min_size(egui::pos2(left0, cy0), egui::vec2(cw, row_h)),
                    0.0, row_bg,
                  );

                  clip_text(ui, c0c, egui::pos2(left0 + 24.0, cy0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &child.display_name,
                    egui::FontId::proportional(13.0), ui.visuals().text_color());
                  let bgt_rect = egui::Rect::from_min_size(
                    egui::pos2(col_x(1), cy0),
                    egui::vec2(col_w[1], row_h),
                  );
                  self.render_budget_limit_cell(ui, &child.full_label, child.limit, bgt_rect, past_period);
                  clip_text(ui, c2c, egui::pos2(col_x(2) + 8.0, cy0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &money(child.spent),
                    egui::FontId::monospace(13.0), ui.visuals().text_color());
                  let rem = child.limit - child.spent;
                  let rem_col = if rem >= 0 { ui.visuals().text_color() } else { theme::current_error() };
                  clip_text(ui, c3c, egui::pos2(col_x(3) + 8.0, cy0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &money(rem),
                    egui::FontId::monospace(13.0), rem_col);
                  let pct = if child.limit > 0 { (child.spent as f32 / child.limit as f32).clamp(0.0, 2.0) } else if child.spent > 0 { 1.5 } else { 0.0 };
                  let bar_cell_w = col_w[4];
                  let bar_rect = egui::Rect::from_min_size(egui::pos2(col_x(4), cy0), egui::vec2(bar_cell_w, row_h));
                  draw_progress_bar(ui, bar_rect, pct);

                  paint_vseps(ui, cy0, cy0 + row_h);
                  ui.allocate_space(egui::vec2(0.0, row_h));
                  row_idx += 1;
                }
                i += 1 + child_count;

              } else {
                // ===== Leaf row (no children) =====
                let c0 = egui::Rect::from_min_size(egui::pos2(col_x(0), y0), egui::vec2(col_w[0], row_h));
                let c2 = egui::Rect::from_min_size(egui::pos2(col_x(2), y0), egui::vec2(col_w[2], row_h));
                let c3 = egui::Rect::from_min_size(egui::pos2(col_x(3), y0), egui::vec2(col_w[3], row_h));
                let row_bg = if row_idx % 2 == 0 { ui.visuals().faint_bg_color } else { ui.visuals().extreme_bg_color };
                ui.painter().rect_filled(
                  egui::Rect::from_min_size(egui::pos2(left0, y0), egui::vec2(cw, row_h)),
                  0.0, row_bg,
                );

                let swatch_rect = egui::Rect::from_min_size(egui::pos2(left0 + 6.0, y0 + row_h / 2.0 - 5.0), egui::vec2(10.0, 10.0));
                ui.painter().circle_filled(swatch_rect.center(), 5.0, cat.color);
                clip_text(ui, c0, egui::pos2(left0 + 22.0, y0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &cat.display_name,
                  egui::FontId::proportional(13.0), ui.visuals().text_color());
                let bgt_rect = egui::Rect::from_min_size(
                  egui::pos2(col_x(1), y0),
                  egui::vec2(col_w[1], row_h),
                );
                self.render_budget_limit_cell(ui, &cat.full_label, cat.limit, bgt_rect, past_period);
                clip_text(ui, c2, egui::pos2(col_x(2) + 8.0, y0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &money(cat.spent),
                  egui::FontId::monospace(13.0), ui.visuals().text_color());
                let rem = cat.limit - cat.spent;
                let rem_col = if rem >= 0 { ui.visuals().text_color() } else { theme::current_error() };
                clip_text(ui, c3, egui::pos2(col_x(3) + 8.0, y0 + row_h / 2.0), egui::Align2::LEFT_CENTER, &money(rem),
                  egui::FontId::monospace(13.0), rem_col);
                let pct = if cat.limit > 0 { (cat.spent as f32 / cat.limit as f32).clamp(0.0, 2.0) } else if cat.spent > 0 { 1.5 } else { 0.0 };
                let bar_cell_w = col_w[4];
                let bar_rect = egui::Rect::from_min_size(egui::pos2(col_x(4), y0), egui::vec2(bar_cell_w, row_h));
                draw_progress_bar(ui, bar_rect, pct);

                paint_vseps(ui, y0, y0 + row_h);
                ui.allocate_space(egui::vec2(0.0, row_h));
                row_idx += 1;
                i += 1;
              }
            }
          });
      },
    );
  }

  fn render_budget_limit_cell(&mut self, ui: &mut egui::Ui, full_label: &str, limit_cents: i64, cell_rect: egui::Rect, past_period: bool) {
    if !past_period && self.budget_editing_category.as_deref() == Some(full_label) {
      // Use an Area overlay so the TextEdit doesn't participate in layout
      // (no cursor advancement, no ghost rows).
      let area_id = egui::Id::new(("budget_edit_area", full_label));
      let resp = egui::Area::new(area_id)
        .fixed_pos(cell_rect.left_top())
        .show(ui.ctx(), |ui| {
          ui.set_width(cell_rect.width());
          ui.add(
            egui::TextEdit::singleline(&mut self.budget_editing_input)
              .font(egui::FontId::monospace(13.0))
              .desired_width(cell_rect.width() - 8.0),
          )
        }).inner;
      resp.request_focus();
      let pressed_enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
      let pressed_esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
      if pressed_esc {
        self.budget_editing_category = None;
      } else if pressed_enter || resp.lost_focus() {
        let cleaned = self.budget_editing_input.trim().replace('$', "");
        if let Ok(val) = cleaned.parse::<f32>() {
          let val_cents = (val * 100.0).round() as i64;
          self.save_budget_with_forward_propagation(full_label, val_cents);
        }
        self.budget_editing_category = None;
      }
    } else {
      let limit_text = if limit_cents > 0 {
        format!("${:.2}", (limit_cents as f32) / 100.0)
      } else {
        "$0.00".to_string()
      };
      if past_period {
        // Dimmed display, no interaction for past periods
        ui.painter().text(
          egui::pos2(cell_rect.left() + 8.0, cell_rect.center().y),
          egui::Align2::LEFT_CENTER,
          &limit_text,
          egui::FontId::monospace(13.0),
          ui.visuals().weak_text_color(),
        );
      } else {
        ui.painter().text(
          egui::pos2(cell_rect.left() + 8.0, cell_rect.center().y),
          egui::Align2::LEFT_CENTER,
          &limit_text,
          egui::FontId::monospace(13.0),
          if limit_cents > 0 { ui.visuals().text_color() } else { ui.visuals().weak_text_color() },
        );
        let id = egui::Id::new(("budget_limit_click", full_label));
        let resp = ui.interact(cell_rect, id, egui::Sense::click());
        if resp.clicked() {
          self.budget_editing_category = Some(full_label.to_string());
          self.budget_editing_input = limit_text.trim_start_matches('$').to_string();
        }
      }
    }
  }

  fn save_budget_with_forward_propagation(&mut self, full_label: &str, val_cents: i64) {
    let gran = self.budget_granularity;
    
    // Special handling for yearly budgets
    if gran == BudgetGranularity::Yearly {
      // Save yearly cap as override
      self.apply_budget_override(full_label, 0, val_cents);
      
      // Propagate to monthly, quarterly, and weekly
      self.propagate_yearly_to_others(full_label, val_cents);
      
      self.log(format!(
        "[budgets] set yearly budget for '{}' → {}",
        full_label, money(val_cents)
      ));
      self.reload();
      return;
    }
    
    // Existing logic for other granularities
    let cur = self.current_period(gran);
    let end = self.period_end(gran);
    
    // 1. Save current period as override
    let cur_code = self.period_code(gran, cur);
    self.apply_budget_override(full_label, cur_code, val_cents);
    
    // 2. Save all future periods as computed defaults
    for p in (cur + 1)..=end {
      let code = self.period_code(gran, p);
      let _ = save_budget_snapshot(
        &self.conn, self.household_id, full_label,
        self.budget_year, code, val_cents, false,
      );
    }
    
    // 3. Propagate to other granularities
    self.propagate_to_other_granularities(full_label, gran, val_cents);
    
    self.log(format!(
      "[budgets] set {} for '{}' → {}",
      format!("{:?}", gran).to_lowercase(),
      full_label, money(val_cents)
    ));
    self.reload();
  }
}

fn draw_progress_bar(ui: &egui::Ui, cell_rect: egui::Rect, percent: f32) {
  let bar_left = cell_rect.left() + 6.0;
  let avail_bar = (cell_rect.width() - 6.0 - 4.0).max(4.0);
  let label_w = 34.0;
  let bar_w = (avail_bar - label_w).max(4.0);
  let bar_h = 8.0;
  let bar_top = cell_rect.center().y - bar_h * 0.5;
  let bar_rect = egui::Rect::from_min_size(
    egui::pos2(bar_left, bar_top),
    egui::vec2(bar_w, bar_h),
  );
  ui.painter().rect_filled(bar_rect, 4.0, ui.visuals().widgets.noninteractive.bg_stroke.color.linear_multiply(0.2));
  if percent > 0.0 {
    let fill_w = bar_w * percent.min(1.0);
    let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_w, bar_h));
    let fill_color = if percent <= 0.8 {
      theme::current_success()
    } else if percent <= 1.0 {
      theme::current_warning()
    } else {
      theme::current_error()
    };
    ui.painter().rect_filled(fill_rect, 4.0, fill_color);
  }
  let pct_label = format!("{:.0}%", percent * 100.0);
  let pct_color = if percent > 1.0 { theme::current_error() } else { ui.visuals().text_color() };
  ui.painter().text(
    egui::pos2(bar_rect.right() + 4.0, bar_rect.center().y),
    egui::Align2::LEFT_CENTER,
    &pct_label,
    egui::FontId::proportional(11.0),
    pct_color,
  );
}
