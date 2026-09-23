use crate::db::*;
use crate::models::*;
use crate::ui::popups::styled_button;
use crate::ui::widgets::*;
use crate::TwoCentsApp;
use eframe::egui;

/// combo box for the statement account — arrow painted inside the
/// same field, ▾ shows the full past-account list, typing contains-searches,
/// unknown names become new accounts at save. Returns state the caller
/// applies (picked name, focus, toggle) so this stays a pure widget.
struct AccountComboOut {
    picked: Option<String>,
    focused: bool,
    toggled: bool,
}

fn account_combo(
    ui: &mut egui::Ui,
    name: &mut String,
    candidates: &[String],
    width: f32,
    text_id: egui::Id,
    dropdown_open: &mut bool,
) -> AccountComboOut {
    let h = ui.spacing().interact_size.y.max(24.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, h), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        crate::ui::theme_tokens::RADIUS_SM,
        crate::ui::theme::bg_primary(),
    );
    painter.rect_stroke(
        rect,
        crate::ui::theme_tokens::RADIUS_SM,
        egui::Stroke::new(1.0, crate::ui::theme::border()),
        egui::StrokeKind::Middle,
    );
    // Frameless text edit fills the left side; chevron lives inside the right
    // edge of the same box, exactly like egui's ComboBox.
    let edit_w = width - 22.0;
    let response = ui
        .scope_builder(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                rect.min + egui::vec2(6.0, 2.0),
                egui::vec2(edit_w, h - 4.0),
            )),
            |ui| {
                ui.add(
                    egui::TextEdit::singleline(name)
                        .id(text_id)
                        .frame(egui::Frame::NONE)
                        .desired_width(edit_w)
                        .hint_text("Account name"),
                )
            },
        )
        .inner;
    let chevron_center = egui::pos2(rect.right() - 11.0, rect.center().y);
    paint_chevron_down(
        &ui.painter(),
        egui::Rect::from_center_size(chevron_center, egui::vec2(10.0, 8.0)),
        crate::ui::theme::fg_secondary(),
    );
    // only the chevron sliver is click-sensitive here — the previous
    // design stacked overlapping full-box interacts and egui's hit-test let
    // the last one swallow the arrow clicks (the dropdown looked dead). Text
    // clicks go to the TextEdit itself; `anchor` (hover-only) spans the whole
    // box so the popup opens below it, flush with both edges.
    let anchor = ui.interact(rect, text_id.with("anchor"), egui::Sense::hover());
    let chevron_zone = ui.interact(
        egui::Rect::from_min_max(
            egui::pos2(rect.right() - 22.0, rect.top()),
            egui::pos2(rect.right(), rect.bottom()),
        ),
        text_id.with("chevron"),
        egui::Sense::click(),
    );
    if chevron_zone.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let mut toggled = false;
    if chevron_zone.clicked() {
        *dropdown_open = !*dropdown_open;
        toggled = true;
        ui.memory_mut(|mem| mem.request_focus(text_id));
    }
    let picked = show_autocomplete_popup(ui, &response, name, candidates, *dropdown_open, &anchor);
    // Enter only confirms the name — importing stays exclusively
    // with the "Save Reviewed Import" button (an earlier Enter-to-save here
    // silently imported every staged row as soon as the name was confirmed).
    AccountComboOut {
        picked,
        focused: response.has_focus(),
        toggled,
    }
}

impl TwoCentsApp {
    #[allow(deprecated)]
    pub fn ui_import_review(&mut self, ctx: &egui::Context) {
        if !self.show_import_review {
            return;
        }
        if let Some(sort) = self.pending_import_sort.take() {
            self.import_sort = sort;
            self.import_grid_state.clear_selection();
            self.rebuild_sorted_import_indices();
        }
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("is_rendering_import_grid"), true));

        let screen_rect = ctx.content_rect();
        let pad_x = if screen_rect.width() < 900.0 {
            21.0
        } else {
            60.0
        };
        let pad_y = if screen_rect.height() < 600.0 {
            21.0
        } else {
            60.0
        };
        // fixed_size constrains the CONTENT; the modal frame wraps it in its own
        // inner margin + stroke. Subtract that chrome or the panel is wider than
        // the window (pad 42 < chrome 50 → the panel sat flush on both edges).
        let frame = themed_modal_frame(ctx);
        let chrome_x = (frame.inner_margin.left + frame.inner_margin.right) as f32
            + 2.0 * crate::ui::theme_tokens::BORDER_W;
        let chrome_y = (frame.inner_margin.top + frame.inner_margin.bottom) as f32
            + 2.0 * crate::ui::theme_tokens::BORDER_W;
        let win_w = (screen_rect.width() - pad_x * 2.0 - chrome_x).clamp(320.0, 1200.0);
        let win_h = (screen_rect.height() - pad_y * 2.0 - chrome_y).clamp(300.0, 800.0);

        // Use title_bar(false) to suppress the draggable title bar and × close button.
        // Those interactive elements competed for keyboard focus with the grid cells,
        // causing TextEdit focus to be stolen on every frame (manifesting as
        // "one letter at a time" erasure during typing). We render our own header row.
        egui::Window::new("Statement Import Review")
      .title_bar(false)
      .resizable(false)
      .collapsible(false)
      .fixed_size([win_w, win_h])
      .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
      .frame(frame)
      .show(ctx, |ui| {
        // ── Header row (our own title bar replacement) ───────────────────────
        let account_candidates: Vec<String> = self.accounts.iter()
          .map(|account| account.name.clone())
          .collect();
        let mut account_picked: Option<String> = None;
        let mut account_focused = false;
        let account_text_id = egui::Id::new("import_account_text");
        let dropdown_id = egui::Id::new("import_account_dropdown");
        let mut dropdown_open = ui.ctx().data_mut(|d| d.get_temp::<bool>(dropdown_id).unwrap_or(false));
        let mut account_own_row = false;
        ui.horizontal(|ui| {
          // heading + secondary text, same hierarchy as the
          // Settings window. Was a single same-color label.
          ui.vertical(|ui| {
            crate::ui::components::heading_lg(ui, "Statement Import Review");
            crate::ui::components::label_muted(
              ui,
              "Edits here are staged. Click 'Save Reviewed Import' to commit them to your spreadsheet.",
            );
          });
          let remaining_width = ui.available_width();
          ui.allocate_ui_with_layout(
            egui::vec2(remaining_width, ui.spacing().interact_size.y),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
              if styled_button(ui, "Cancel Review", false).clicked() {
                self.show_import_review = false;
              }
              if styled_button(ui, "Save Reviewed Import", true).clicked() {
                self.save_import();
              }
              // Statement-level account: prefilled from the CSV's Account
              // column when present; rename it freely before saving — the
              // detected value keeps mapping to the new name next import.
              // the combo never shrinks below ~200px — if the
              // header strip can't fit it, it drops to its own full-width
              // row under the header instead.
              let available = ui.available_width() - 60.0; // reserve for "Account:"
              if available >= 200.0 {
                let out = account_combo(
                  ui,
                  &mut self.import_account_name,
                  &account_candidates,
                  available.clamp(200.0, 240.0),
                  account_text_id,
                  &mut dropdown_open,
                );
                account_picked = out.picked;
                account_focused = out.focused;
                if out.toggled {
                  ui.ctx().data_mut(|d| d.insert_temp(dropdown_id, dropdown_open));
                }
                if self.import_account_name.trim().is_empty() {
                  crate::ui::components::label_muted(ui, "Name the account before saving");
                }
                ui.label(egui::RichText::new("Account:").weak());
              } else {
                account_own_row = true;
              }
            },
          );
        });
        if account_own_row {
          ui.add_space(2.0);
          // right-aligned AND one row high — bare with_layout let
          // Align::Center float the picker in the window's leftover height
          // (huge gap); allocating the row pins it under the header.
          ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
              let width = ui.available_width().min(340.0);
              let out = account_combo(
                ui,
                &mut self.import_account_name,
                &account_candidates,
                width,
                account_text_id,
                &mut dropdown_open,
              );
              account_picked = out.picked;
              account_focused = out.focused;
              if out.toggled {
                ui.ctx().data_mut(|d| d.insert_temp(dropdown_id, dropdown_open));
              }
              if self.import_account_name.trim().is_empty() {
                crate::ui::components::label_muted(ui, "Name the account before saving");
              }
              ui.label(egui::RichText::new("Account:").weak());
            },
          );
        }
        if let Some(picked) = account_picked {
          self.import_account_name = picked;
          dropdown_open = false;
          ui.ctx().data_mut(|d| d.insert_temp(dropdown_id, false));
        }
        // the statement picker IS the account assignment — mirror
        // its value into every staged row so the grid's Account column shows
        // live what will be saved (typed names and dropdown picks alike).
        if !self.import_rows.is_empty() {
          let name = self.import_account_name.trim().to_string();
          for row in &mut self.import_rows {
            row.account = name.clone();
          }
        }

        // Escape handling: only close the modal if nothing is being edited.
        // If a cell is being edited, let the grid handle Escape first (cancel edit);
        // if the account picker is focused, Escape just clears its suggestion list.
        if self.import_grid_state.edit_cell.is_none() {
          if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if account_focused || dropdown_open {
              ui.memory_mut(|mem| mem.surrender_focus(account_text_id));
              ui.ctx().data_mut(|d| d.insert_temp(dropdown_id, false));
            } else if self.import_grid_state.selection.is_some() {
              self.import_grid_state.clear_selection();
            } else {
              self.show_import_review = false;
              return;
            }
          }
        }
        ui.add_space(4.0);
        crate::ui::components::label_muted(
          ui,
          "Click or drag a column to select rows. Hold Shift and drag to pan. Type to edit; Enter applies to all selected rows; Tab or Enter picks autocomplete; Escape clears (Escape again cancels review).",
        );
        ui.add_space(8.0);

        let category_candidates = self.cached_category_candidates.clone();
        let member_candidates = self.cached_member_candidates.clone();
        let vendor_candidates = self.cached_import_vendor_candidates.clone();
        let description_candidates = self.cached_import_description_candidates.clone();
        let mut autocomplete_selection = self.autocomplete_selection;


        // shared grid_table_frame so the three grids have identical chrome.
        let frame_resp = crate::ui::components::grid_table_frame(ui)
          .show(ui, |ui| {
            let old_spacing = ui.spacing().item_spacing;
            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;

            let grid_res = crate::ui::grid::render_grid(
              ui,
              &mut self.import_rows,
              &self.cached_sorted_import_indices,
              &mut self.import_grid_state,
              &mut autocomplete_selection,
              &vendor_candidates,
              &category_candidates,
              &member_candidates,
              &description_candidates,
              &self.members,
              &self.categories,
              "import_cell",
              true,
            );

            self.import_grid_state.active_cell = grid_res.active_cell;

            if let Some(col) = grid_res.clicked_sort_column {
              let ascending = if self.import_sort.column == col {
                !self.import_sort.ascending
              } else {
                true
              };
              self.pending_import_sort = Some(ExpenseSort { column: col, ascending });
              ui.ctx().request_repaint();
            }

            if let Some((cat_id, name, color)) = grid_res.open_color_popup {
              self.category_color_popup = Some(CategoryColorPopup {
                id: cat_id,
                name,
                color,
              });
            }

            if let Some(rows_to_delete) = grid_res.force_delete_rows {
              self.delete_import_rows_by_indices(&rows_to_delete);
            } else if let Some(rows_to_delete) = grid_res.delete_rows {
              self.show_delete_import_confirm = true;
              self.delete_import_indices = rows_to_delete;
            }

            let pending_updates = grid_res.pending_field_updates;
            let pending_category_commits = grid_res.pending_category_commits;
            let pending_member_commits = grid_res.pending_member_commits;

            for &(idx, field) in &pending_updates {
              if field == "amount" {
                if let Some(row) = self.import_rows.get_mut(idx) {
                  if let Some(magnitude) = parse_amount_cents(&row.amount_input) {
                    // sign is a function of category; excluded rows
                    // keep their real amount (flag filters aggregation only).
                    let sign = category_sign(&self.categories, &row.category);
                    row.amount_cents = if sign == 1 { magnitude } else { -magnitude };
                  }
                }
              }
            }
            for idx in &pending_member_commits {
              if let Some(row) = self.import_rows.get_mut(*idx) {
                row.member = row.member.trim().to_string();
              }
            }
            if !pending_category_commits.is_empty() {
              self.categories = load_categories(&self.conn, self.household_id).unwrap_or_default();
              // sign is a function of category — re-derive after
              // re-categorization so the review grid never shows stale signs.
              self.sync_import_amounts();
            }

            if self.import_grid_state.drag.is_some() {
              ui.ctx().request_repaint();
            }
            ui.spacing_mut().item_spacing = old_spacing;
          });
        // Frame stroke on top of the scrolled content (clip_rect_margin bleed).
        crate::ui::components::repaint_grid_frame_stroke(ui, frame_resp.response.rect);
        self.autocomplete_selection = autocomplete_selection;
      });

        ctx.data_mut(|d| {
            d.remove_temp::<bool>(egui::Id::new("is_rendering_import_grid"));
        });
    }

    fn rebuild_import_cached_candidates(&mut self) {
        self.cached_import_vendor_candidates =
            unique_nonempty_values(self.import_rows.iter().map(|row| row.vendor.as_str()));
        self.cached_import_description_candidates =
            unique_nonempty_values(self.import_rows.iter().map(|row| row.description.as_str()));
    }

    pub fn import_csv(&mut self) {
        if self.csv_import_rx.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.csv_import_rx = Some(rx);
        std::thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .set_title("Import statement CSV")
                .add_filter("CSV statements", &["csv"])
                .pick_file();
            let _ = tx.send(path);
        });
    }

    pub fn process_csv_file(&mut self, path: &std::path::Path) {
        match parse_csv_statement(&self.conn, self.household_id, path) {
            Ok(rows) if rows.is_empty() => {}
            Ok(rows) => {
                let duplicates = duplicate_import_count(&self.conn, self.household_id, &rows);
                let ready = rows
                    .iter()
                    .filter(|row| import_row_status(row) == "ready")
                    .count();
                // prefill the account name from the CSV's Account column
                // (most common non-empty value). A previously-mapped csv_name wins,
                // otherwise the raw value (rename it if you like); nothing detected
                // stays blank — save is blocked until a name is typed.
                let mut counts: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                for row in &rows {
                    if !row.account.trim().is_empty() {
                        *counts.entry(row.account.trim()).or_insert(0) += 1;
                    }
                }
                let detected = counts
                    .iter()
                    .max_by_key(|(_, count)| **count)
                    .map(|(name, _)| name.to_string())
                    .unwrap_or_default();
                self.import_detected_account = detected.clone();
                self.import_account_name = if let Some(matched) =
                    self.accounts.iter().find(|account| {
                        account
                            .csv_name
                            .as_deref()
                            .is_some_and(|value| value.eq_ignore_ascii_case(&detected))
                    }) {
                    matched.name.clone()
                } else {
                    detected
                };
                self.import_rows = rows;
                self.sync_import_amounts();
                self.rebuild_import_cached_candidates();
                self.rebuild_sorted_import_indices();
                self.show_import_review = true;
            }
            Err(_err) => {}
        }
    }

    fn save_import(&mut self) {
        if self.import_rows.is_empty() || self.import_account_name.trim().is_empty() {
            return;
        }

        self.sync_import_amounts();
        let rows = self.import_rows.clone();
        let account = resolve_or_create_account(
            &self.conn,
            self.household_id,
            &self.import_detected_account,
            &self.import_account_name,
        );
        match account.and_then(|account_id| {
            save_import_rows(&self.conn, self.household_id, &rows, account_id)
        }) {
            Ok(count) => {
                self.import_rows.clear();
                self.show_import_review = false;
                self.reload();
            }
            Err(err) => eprintln!("[import] save failed: {err}"),
        }
    }

    pub fn delete_import_rows_by_indices(&mut self, indices: &[usize]) {
        let mut sorted_indices = indices.to_vec();
        sorted_indices.sort_by(|a, b| b.cmp(a));
        for &idx in &sorted_indices {
            if idx < self.import_rows.len() {
                self.import_rows.remove(idx);
            }
        }
        self.rebuild_import_cached_candidates();
        self.rebuild_sorted_import_indices();
        self.import_grid_state.clear_selection();
    }

    fn sync_import_amounts(&mut self) {
        let categories = self.categories.clone();
        for row in &mut self.import_rows {
            if let Some(magnitude) = parse_amount_cents(&row.amount_input) {
                // sign is a function of category; excluded rows keep
                // their real amount (flag filters aggregation only).
                let sign = category_sign(&categories, &row.category);
                row.amount_cents = if sign == 1 { magnitude } else { -magnitude };
            }
        }
    }
}
