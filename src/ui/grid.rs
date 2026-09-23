use crate::db::*;
use crate::models::*;
use crate::ui::popups::styled_button;
use crate::ui::popups::{category_picker_menu_ui, member_color_for};
use crate::ui::widgets::*;
/// Unified expense/import grid body renderer.
///
/// Both the Expense spreadsheet and the Import Review grid call this single function.
/// The only differences between the two call sites are:
///   - which row slice they pass (Expense vs ImportRow — both implement GridRow)
///   - which cell-ID prefix they use (avoids egui ID collisions)
///   - how they process the returned `GridResult` (DB commit vs in-memory update)
use eframe::egui::{self, Color32, RichText};
use egui_extras::TableBuilder;

// ---------------------------------------------------------------------------
// Result type — carries everything the caller needs to commit after rendering
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct GridResult {
    /// (row_index, field_name) pairs for simple field writes
    pub pending_field_updates: Vec<(usize, &'static str)>,
    /// Row indices whose category field needs validated & committed
    pub pending_category_commits: Vec<usize>,
    /// Row indices whose member field was changed
    pub pending_member_commits: Vec<usize>,
    /// If Some, the caller should open a category color popup for (id, label, current_color)
    pub open_color_popup: Option<(i64, String, Color32)>,
    /// The cell that currently has keyboard focus (for status-bar display)
    pub active_cell: Option<(GridColumn, usize)>,
    /// Selected indices requested for deletion
    pub delete_rows: Option<Vec<usize>>,
    /// Selected indices requested for forced deletion (bypassing confirmation)
    pub force_delete_rows: Option<Vec<usize>>,
    /// If Some, the user clicked a column header to trigger sorting
    pub clicked_sort_column: Option<ExpenseSortColumn>,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Thin row separator painted at the top of a table cell. Called at the top
/// of every column's cell closure so the lines join into full-width row
/// separators (painting it only in the first column left the Date column
/// looking boxed while the rest of the table was borderless).
fn paint_row_separator_top(ui: &egui::Ui, show: bool) {
    if !show {
        return;
    }
    let r = ui.max_rect();
    ui.painter().line_segment(
        [
            egui::pos2(r.left(), r.top()),
            egui::pos2(r.right(), r.top()),
        ],
        egui::Stroke::new(1.0_f32, crate::ui::theme::border()),
    );
}

/// Thin vertical separator painted at the left edge of a table cell. Called
/// for every column except the first, in both the header and the body, so
/// the per-row segments join into full-height column separators.
fn paint_col_separator_left(ui: &egui::Ui) {
    let r = ui.max_rect();
    ui.painter().line_segment(
        [
            egui::pos2(r.left(), r.top()),
            egui::pos2(r.left(), r.bottom()),
        ],
        egui::Stroke::new(1.0_f32, crate::ui::theme::border()),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn render_grid<R: GridRow>(
    ui: &mut egui::Ui,
    rows: &mut Vec<R>,
    sorted_indices: &[usize],
    state: &mut GridState,
    autocomplete_selection: &mut usize,
    // --- Candidate lists for autocomplete ---
    vendor_candidates: &[String],
    category_candidates: &[String],
    member_candidates: &[String],
    description_candidates: &[String],
    // --- People / category data ---
    members: &[HouseholdMember],
    categories: &[Category],

    // --- Unique string prefix so expense vs import cells don't share egui IDs ---
    cell_id_prefix: &'static str,
    // --- Whether to draw per-row separator lines (expense grid has them) ---
    show_row_separator: bool,
) -> GridResult {
    let mut result = GridResult::default();

    // ── KEYBOARD INTERACTION HOOKS ───────────────────────────────────────────
    // Only handle Escape at the grid level when NOT editing a cell.
    // When editing, the per-cell Escape handlers restore the original value and
    // cancel the edit — clearing selection here would preempt that logic.
    if state.edit_cell.is_none() {
        if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            state.clear_selection();
        }
    }
    if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
        if let Some(sel) = &state.selection {
            if let Some(&row) = sel.rows.first() {
                state.edit_cell = Some((sel.column, row));
            }
        }
    }
    if state.edit_cell.is_none() && state.selection.is_some() {
        let is_ctrl_del = ui.input_mut(|input| {
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::Delete)
                || input.consume_key(egui::Modifiers::CTRL, egui::Key::Delete)
        });
        if is_ctrl_del {
            if let Some(sel) = &state.selection {
                result.force_delete_rows = Some(sel.rows.clone());
            }
        } else {
            let is_del =
                ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Delete));
            if is_del {
                if let Some(sel) = &state.selection {
                    result.delete_rows = Some(sel.rows.clone());
                }
            }
        }
    }

    // ── PENDING KEYBOARD EVENTS COMMITMENTS ──────────────────────────────────
    // candidate lists were cloned to owned Vecs every frame even with
    // nothing being edited. The closure is only consumed by editing paths, so
    // clone lazily inside it.
    let editing_now = state.edit_cell.is_some();
    let candidates_fn = move |col| match col {
        GridColumn::Member => {
            if editing_now {
                member_candidates.to_vec()
            } else {
                Vec::new()
            }
        }
        GridColumn::Category => {
            if editing_now {
                category_candidates.to_vec()
            } else {
                Vec::new()
            }
        }
        GridColumn::Vendor => {
            if editing_now {
                vendor_candidates.to_vec()
            } else {
                Vec::new()
            }
        }
        GridColumn::Description => {
            if editing_now {
                description_candidates.to_vec()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };

    let indices_to_use = sorted_indices;

    if let Some((column, _, targets)) =
        state.apply_pending_keyboard(rows, indices_to_use, *autocomplete_selection, candidates_fn)
    {
        match column {
            GridColumn::Date => result
                .pending_field_updates
                .extend(targets.into_iter().map(|t| (t, "date"))),
            GridColumn::Amount => result
                .pending_field_updates
                .extend(targets.into_iter().map(|t| (t, "amount"))),
            GridColumn::Member => result.pending_member_commits.extend(targets),
            GridColumn::Category => result.pending_category_commits.extend(targets),
            GridColumn::Vendor => result
                .pending_field_updates
                .extend(targets.into_iter().map(|t| (t, "vendor"))),
            GridColumn::Description => result
                .pending_field_updates
                .extend(targets.into_iter().map(|t| (t, "description"))),
            GridColumn::Account => result
                .pending_field_updates
                .extend(targets.into_iter().map(|t| (t, "account"))),
        }
    }

    let selection = &mut state.selection;
    let drag = &mut state.drag;
    let edit_cell = &mut state.edit_cell;
    let edit_original = &mut state.edit_original;
    let typeahead = &mut state.typeahead;
    let scroll_offset = &mut state.scroll_offset;

    // Apply any pending typeahead character before rendering so the first frame
    // shows the typed character in the correct cell.
    if let Some((tc, ti)) = edit_cell.as_ref().copied().zip(typeahead.take()) {
        let Some(row) = rows.get_mut(tc.1) else {
            *typeahead = None;
            return result;
        };
        match tc.0 {
            GridColumn::Date => {
                row.row_date_mut().clear();
                row.row_date_mut().push(ti);
            }
            GridColumn::Amount => {
                row.row_amount_input_mut().clear();
                row.row_amount_input_mut().push(ti);
            }
            GridColumn::Member => {
                row.row_member_mut().clear();
                row.row_member_mut().push(ti);
            }
            GridColumn::Category => {
                row.row_category_mut().clear();
                row.row_category_mut().push(ti);
            }
            GridColumn::Vendor => {
                row.row_vendor_mut().clear();
                row.row_vendor_mut().push(ti);
            }
            GridColumn::Description => {
                row.row_description_mut().clear();
                row.row_description_mut().push(ti);
            }
            GridColumn::Account => {
                row.row_account_mut().clear();
                row.row_account_mut().push(ti);
            }
        }
    } else {
        *typeahead = None; // discard if no edit cell
    }

    // menu width measured once per (prefix, category count) and
    // cached in egui temp data — layout_no_wrap per category per frame was
    // pure waste; the measurement can only change when categories change.
    let menu_w_id = egui::Id::new((cell_id_prefix, "cat_menu_w", categories.len()));
    let category_menu_w = if ui.ctx().data(|d| d.get_temp::<f32>(menu_w_id).is_some()) {
        ui.ctx()
            .data(|d| d.get_temp::<f32>(menu_w_id).unwrap_or(200.0))
    } else {
        let measured = category_picker_menu_width(ui, categories, 200.0);
        ui.ctx().data_mut(|d| d.insert_temp(menu_w_id, measured));
        measured
    };
    let member_menu_w = MEMBER_PICKER_MIN_WIDTH;
    let member_count = members.len();
    let member_scroll = if member_count > 10 {
        Some(member_picker_max_height(member_count))
    } else {
        None
    };

    // label→color map built once per frame — the old path rebuilt a
    // HashMap + format!-ed every candidate full label PER CATEGORY CELL.
    let parents_map = category_parent_map(categories);
    let mut cat_colors: std::collections::HashMap<String, Color32> =
        std::collections::HashMap::with_capacity(categories.len() * 2);
    for category in categories {
        cat_colors.insert(category.name.clone(), category.color);
        cat_colors.insert(category.full_label(&parents_map), category.color);
    }

    let mut row_drag_bands: Vec<(usize, f32, f32)> = Vec::new();
    let mut scroll_y = *scroll_offset;

    // Helper closures to make cell egui::Id stable based on visual row positions
    let cell_id_by_visual = |col: GridColumn, visual_row: usize| -> egui::Id {
        // hash the (prefix, column, row) tuple directly — format!-ing
        // the enum was 12 String allocations per row per frame.
        egui::Id::new((cell_id_prefix, col as u8, visual_row))
    };
    let get_cell_id_for_raw_row = |col: GridColumn, raw_idx: usize| -> Option<egui::Id> {
        sorted_indices
            .iter()
            .position(|&i| i == raw_idx)
            .map(|vis_row| cell_id_by_visual(col, vis_row))
    };
    let cell_id = |col: GridColumn, idx: usize| -> egui::Id {
        get_cell_id_for_raw_row(col, idx).unwrap_or_else(|| cell_id_by_visual(col, idx))
    };
    let cell_has_focus = |ctx: &egui::Context, col: GridColumn, raw_idx: usize| -> bool {
        get_cell_id_for_raw_row(col, raw_idx).is_some_and(|id| ctx.memory(|m| m.has_focus(id)))
    };

    let w = ui.available_width();
    let fractions = [0.11, 0.09, 0.09, 0.09, 0.17, 0.14, 0.31];
    let labels: [(GridColumn, &str); 7] = [
        (GridColumn::Date, "Date"),
        (GridColumn::Account, "Account"),
        (GridColumn::Amount, "Amount"),
        (GridColumn::Member, "Member"),
        (GridColumn::Category, "Category"),
        (GridColumn::Vendor, "Vendor"),
        (GridColumn::Description, "Description"),
    ];
    let sort_cols: [ExpenseSortColumn; 7] = [
        ExpenseSortColumn::Date,
        ExpenseSortColumn::Account,
        ExpenseSortColumn::Amount,
        ExpenseSortColumn::Member,
        ExpenseSortColumn::Category,
        ExpenseSortColumn::Vendor,
        ExpenseSortColumn::Description,
    ];

    // Frozen header row
    ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (i, (_, label)) in labels.iter().enumerate() {
            ui.allocate_ui_with_layout(
                egui::vec2(w * fractions[i], GRID_HEADER_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    // paint the header fill FIRST, then the separator on
                    // top — the old order let grid_header's opaque rect_filled bury
                    // its own column border (worst at fractional widths).
                    if grid_header(ui, label).clicked() {
                        result.clicked_sort_column = Some(sort_cols[i]);
                    }
                    if i > 0 {
                        paint_col_separator_left(ui);
                    }
                },
            );
        }
    });

    // Scrollable body
    let scroll_response = egui::ScrollArea::vertical()
        .id_salt(format!("{cell_id_prefix}_scroll"))
        .auto_shrink([false, false])
        // drag_to_scroll was removed in egui 0.36 — ScrollSource.drag now
        // defaults to OnTouch; pin Never so mouse-drag stays cell-selection.
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            drag: egui::containers::scroll_area::DragScroll::Never,
            ..Default::default()
        })
        .scroll_offset(egui::vec2(0.0, scroll_y))
        .show(ui, |ui| {
            ui.style_mut().spacing.item_spacing = egui::Vec2::ZERO;
            let mut builder = TableBuilder::new(ui)
                .id_salt(format!("{cell_id_prefix}_table"))
                .striped(true)
                .vscroll(false)
                .min_scrolled_height(120.0)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
            for &f in &fractions {
                builder = builder.column(egui_extras::Column::initial(w * f).clip(true));
            }

            builder.body(|body| {
                // body.rows virtualizes — only the visible window of rows
                // runs their cell closures. The old `for … body.row()` loop ran all
                // N rows every frame even though egui clipped the painting only.
                body.rows(GRID_ROW_HEIGHT, sorted_indices.len(), |mut row_ui| {
                    let visual_row_index = row_ui.index();
                    let idx = sorted_indices[visual_row_index];

                    // ── DATE ────────────────────────────────────────────────────
                    row_ui.col(|ui| {
                        let column = GridColumn::Date;
                        let cell_rect = ui.max_rect();
                        row_drag_bands.push((idx, cell_rect.top(), cell_rect.bottom()));
                        process_grid_column_cell(
                            ui,
                            column,
                            idx,
                            cell_id_by_visual(column, visual_row_index).with("bg"),
                            selection,
                            drag,
                            edit_cell,
                        );
                        ui.horizontal(|ui| {
                            let editing = grid_cell_editing(edit_cell, column, idx);
                            let (_resp, date_changed) = ui_grid_date_edit(
                                ui,
                                rows[idx].row_date_mut(),
                                edit_original,
                                edit_cell,
                                cell_id_by_visual(column, visual_row_index),
                                editing,
                            );
                            let press_enter =
                                editing && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if date_changed || press_enter {
                                result.pending_field_updates.push((idx, "date"));
                                if press_enter {
                                    *edit_cell = None;
                                    *edit_original = None;
                                }
                                // propagate to selected rows
                                let targets = grid_commit_targets(selection, column, idx);
                                let value = rows[idx].row_date().to_string();
                                for &t in &targets {
                                    if t != idx {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_date_mut() = value.clone();
                                        }
                                    }
                                    result.pending_field_updates.push((t, "date"));
                                }
                            }
                            if editing && cell_has_focus(ui.ctx(), column, idx) {
                                result.active_cell = Some((column, idx));
                            }
                        });
                        // separators painted AFTER content — the opaque
                        // selection/ghost fill in process_grid_column_cell used to
                        // bury them, erasing borders along selected rows.
                        paint_row_separator_top(ui, show_row_separator);
                    });

                    // ── ACCOUNT ─────────────────────────────────────────────────
                    row_ui.col(|ui| {
                        let column = GridColumn::Account;
                        process_grid_column_cell(
                            ui,
                            column,
                            idx,
                            cell_id_by_visual(column, visual_row_index).with("bg"),
                            selection,
                            drag,
                            edit_cell,
                        );
                        // review grid's Account is statement-level — the
                        // picker in the header owns it, so the cell is read-only.
                        if cell_id_prefix == "import_cell" {
                            if edit_cell.map(|(col, _)| col) == Some(column) {
                                *edit_cell = None;
                                *edit_original = None;
                            }
                            let text = RichText::new(rows[idx].row_account())
                                .color(crate::ui::theme::fg_primary());
                            ui.add_sized(
                                [ui.available_width(), GRID_ROW_HEIGHT],
                                egui::Label::new(text).truncate(),
                            );
                        } else {
                            ui.horizontal(|ui| {
                                let editing = grid_cell_editing(edit_cell, column, idx);
                                let response = ui_grid_text_edit(
                                    ui,
                                    rows[idx].row_account_mut(),
                                    edit_original,
                                    edit_cell,
                                    cell_id_by_visual(column, visual_row_index),
                                    editing,
                                );
                                if editing && cell_has_focus(ui.ctx(), column, idx) {
                                    result.active_cell = Some((column, idx));
                                }
                                if response.changed() {
                                    result.pending_field_updates.push((idx, "account"));
                                }
                                if editing
                                    && grid_text_field_committed(ui, &response, egui::Rect::NOTHING)
                                {
                                    let targets = grid_commit_targets(selection, column, idx);
                                    let value = rows[idx].row_account().to_string();
                                    for &t in &targets {
                                        if t != idx {
                                            if let Some(r) = rows.get_mut(t) {
                                                *r.row_account_mut() = value.clone();
                                            }
                                        }
                                        result.pending_field_updates.push((t, "account"));
                                    }
                                    *edit_cell = None;
                                    *edit_original = None;
                                }
                            });
                        }
                        paint_row_separator_top(ui, show_row_separator);
                        paint_col_separator_left(ui);
                    });

                    // ── AMOUNT ──────────────────────────────────────────────────
                    row_ui.col(|ui| {
                        let column = GridColumn::Amount;
                        process_grid_column_cell(
                            ui,
                            column,
                            idx,
                            cell_id_by_visual(column, visual_row_index).with("bg"),
                            selection,
                            drag,
                            edit_cell,
                        );
                        ui.horizontal(|ui| {
                            let editing = grid_cell_editing(edit_cell, column, idx);
                            if editing {
                                let amount_resp = ui_grid_text_edit(
                                    ui,
                                    rows[idx].row_amount_input_mut(),
                                    edit_original,
                                    edit_cell,
                                    cell_id_by_visual(column, visual_row_index),
                                    true,
                                );
                                if amount_resp.changed() {
                                    // Strip non-numeric chars then collapse extra decimals
                                    rows[idx].row_amount_input_mut().retain(|c| {
                                        c.is_ascii_digit() || c == '.' || c == ',' || c == '-'
                                    });
                                    let s = rows[idx].row_amount_input().to_string();
                                    let mut seen_dot = false;
                                    *rows[idx].row_amount_input_mut() = s
                                        .chars()
                                        .filter(|&c| {
                                            if c == '.' {
                                                if seen_dot {
                                                    return false;
                                                }
                                                seen_dot = true;
                                            }
                                            true
                                        })
                                        .collect();
                                    result.pending_field_updates.push((idx, "amount"));
                                }
                                if editing && cell_has_focus(ui.ctx(), column, idx) {
                                    result.active_cell = Some((column, idx));
                                }
                                if grid_text_field_committed(ui, &amount_resp, egui::Rect::NOTHING)
                                {
                                    let orig = edit_original.take().unwrap_or_default();
                                    if let Some(magnitude) =
                                        parse_amount_cents(rows[idx].row_amount_input())
                                    {
                                        // sign is a function of category — debit
                                        // negative, Income positive. Excluded rows keep
                                        // their real amount (flag filters aggregation only).
                                        let sign =
                                            category_sign(categories, rows[idx].row_category());
                                        let cents = if sign == 1 { magnitude } else { -magnitude };
                                        rows[idx].set_row_amount_cents(cents);
                                        let formatted = money(cents).replace('$', "");
                                        *rows[idx].row_amount_input_mut() = formatted.clone();
                                        // propagate to selected rows — each row signed by ITS category
                                        let targets = grid_commit_targets(selection, column, idx);
                                        for &t in &targets {
                                            if t != idx {
                                                if let Some(r) = rows.get_mut(t) {
                                                    let t_sign =
                                                        category_sign(categories, r.row_category());
                                                    let t_cents = if t_sign == 1 {
                                                        magnitude
                                                    } else {
                                                        -magnitude
                                                    };
                                                    r.set_row_amount_cents(t_cents);
                                                    *r.row_amount_input_mut() =
                                                        money(t_cents).replace('$', "");
                                                }
                                            }
                                            result.pending_field_updates.push((t, "amount"));
                                        }
                                    } else {
                                        *rows[idx].row_amount_input_mut() = orig;
                                    }
                                    *edit_cell = None;
                                }
                            } else {
                                // genuinely-zero rows show a dash; positives
                                // plain (no +), negatives carry the minus.
                                let cents = rows[idx].row_amount_cents();
                                let display = if cents == 0 {
                                    "—".to_string()
                                } else {
                                    money(cents)
                                };
                                let mut temp = display;
                                ui_grid_text_edit(
                                    ui,
                                    &mut temp,
                                    edit_original,
                                    edit_cell,
                                    cell_id_by_visual(column, visual_row_index),
                                    false,
                                );
                            }
                        });
                        paint_row_separator_top(ui, show_row_separator);
                        paint_col_separator_left(ui);
                    });

                    // ── MEMBER ──────────────────────────────────────────────────
                    row_ui.col(|ui| {
                        let column = GridColumn::Member;
                        process_grid_column_cell(
                            ui,
                            column,
                            idx,
                            cell_id_by_visual(column, visual_row_index).with("bg"),
                            selection,
                            drag,
                            edit_cell,
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                            let editing = grid_cell_editing(edit_cell, column, idx);
                            if editing && edit_original.is_none() {
                                *edit_original = Some(rows[idx].row_member().to_string());
                            }
                            let member_bg = member_color_for(members, rows[idx].row_member());
                            let cell = member_cell_ui(
                                ui,
                                rows[idx].row_member_mut(),
                                member_bg,
                                cell_id_by_visual(column, visual_row_index),
                                editing,
                            );
                            if editing
                                && ui.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                                })
                            {
                                if let Some(orig) = edit_original.take() {
                                    *rows[idx].row_member_mut() = orig;
                                }
                                *edit_cell = None;
                                cell.text.surrender_focus();
                            }
                            if editing && cell_has_focus(ui.ctx(), column, idx) {
                                result.active_cell = Some((column, idx));
                            }
                            if editing {
                                if let Some(picked) = show_cell_autocomplete_popup(
                                    ui,
                                    &cell.text,
                                    rows[idx].row_member(),
                                    member_candidates,
                                    autocomplete_selection,
                                ) {
                                    let targets = grid_commit_targets(selection, column, idx);
                                    for &t in &targets {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_member_mut() = picked.clone();
                                        }
                                    }
                                    result.pending_member_commits.extend(targets);
                                    *edit_cell = None;
                                    *edit_original = None;
                                }
                            }
                            // Chevron picker
                            let mut member_pick = false;
                            let mut picked_member_value: Option<String> = None;
                            let current_member = rows[idx].row_member().trim().to_string();
                            grid_chevron_picker_popup(
                                ui,
                                &cell.chevron,
                                member_menu_w,
                                member_scroll,
                                |ui| {
                                    if member_none_row_selectable(ui, current_member.is_empty()) {
                                        picked_member_value = Some(String::new());
                                        member_pick = true;
                                        ui.close();
                                    }
                                    ui.separator();
                                    for member in members {
                                        if member_row_selectable(
                                            ui,
                                            member,
                                            member.name.eq_ignore_ascii_case(&current_member),
                                        ) {
                                            picked_member_value = Some(member.name.clone());
                                            member_pick = true;
                                            ui.close();
                                        }
                                    }
                                },
                            );
                            if member_pick {
                                let value = picked_member_value.unwrap_or_default();
                                let targets = grid_commit_targets(selection, column, idx);
                                for &t in &targets {
                                    if let Some(r) = rows.get_mut(t) {
                                        *r.row_member_mut() = value.clone();
                                    }
                                }
                                result.pending_member_commits.extend(targets);
                                *edit_cell = None;
                                *edit_original = None;
                            } else if editing
                                && grid_text_field_committed(ui, &cell.text, egui::Rect::NOTHING)
                            {
                                let targets = grid_commit_targets(selection, column, idx);
                                let value = rows[idx].row_member().to_string();
                                for &t in &targets {
                                    if t != idx {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_member_mut() = value.clone();
                                        }
                                    }
                                }
                                result.pending_member_commits.extend(targets);
                                *edit_cell = None;
                                *edit_original = None;
                            }
                        });
                        paint_row_separator_top(ui, show_row_separator);
                        paint_col_separator_left(ui);
                    });

                    // ── CATEGORY ────────────────────────────────────────────────
                    row_ui.col(|ui| {
                        let column = GridColumn::Category;
                        process_grid_column_cell(
                            ui,
                            column,
                            idx,
                            cell_id_by_visual(column, visual_row_index).with("bg"),
                            selection,
                            drag,
                            edit_cell,
                        );
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                            let editing = grid_cell_editing(edit_cell, column, idx);
                            if editing && edit_original.is_none() {
                                *edit_original = Some(rows[idx].row_category().to_string());
                            }
                            let cat_bg = cat_colors
                                .get(rows[idx].row_category().trim())
                                .copied()
                                .unwrap_or(Color32::TRANSPARENT);
                            let cell = category_cell_ui(
                                ui,
                                rows[idx].row_category_mut(),
                                cat_bg,
                                cell_id_by_visual(column, visual_row_index),
                                editing,
                            );
                            if editing
                                && ui.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                                })
                            {
                                if let Some(orig) = edit_original.take() {
                                    *rows[idx].row_category_mut() = orig;
                                }
                                *edit_cell = None;
                                cell.text.surrender_focus();
                            }
                            if editing && cell_has_focus(ui.ctx(), column, idx) {
                                result.active_cell = Some((column, idx));
                            }
                            // Typeahead autocomplete
                            if editing {
                                if let Some(picked) = show_cell_autocomplete_popup(
                                    ui,
                                    &cell.text,
                                    rows[idx].row_category(),
                                    category_candidates,
                                    autocomplete_selection,
                                ) {
                                    let targets = grid_commit_targets(selection, column, idx);
                                    for &t in &targets {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_category_mut() = picked.clone();
                                        }
                                    }
                                    result.pending_category_commits.extend(targets);
                                    *edit_cell = None;
                                    *edit_original = None;
                                }
                            }
                            // Context menu: Edit color
                            let category_name = rows[idx].row_category().trim().to_string();
                            cell.text.context_menu(|ui| {
                                if category_name.is_empty() {
                                    ui.label(
                                        RichText::new("Pick a category from the list")
                                            .small()
                                            .weak(),
                                    );
                                    return;
                                }
                                if let Some(cat) =
                                    find_category_by_label(categories, &category_name)
                                {
                                    if styled_button(ui, "Edit color…", false).clicked() {
                                        result.open_color_popup =
                                            Some((cat.id, category_name.clone(), cat_bg));
                                        ui.close();
                                    }
                                }
                            });
                            // Chevron picker
                            let mut combobox_pick = false;
                            let mut picked_category_value: Option<String> = None;
                            grid_chevron_picker_popup(
                                ui,
                                &cell.chevron,
                                category_menu_w,
                                Some(280.0),
                                |ui| {
                                    if let Some(label) = category_picker_menu_ui(
                                        ui,
                                        categories,
                                        rows[idx].row_category(),
                                    ) {
                                        picked_category_value = Some(label);
                                        combobox_pick = true;
                                    }
                                },
                            );
                            if combobox_pick {
                                let mut value = picked_category_value.unwrap_or_default();
                                if let Some(matched) = find_category_by_label(categories, &value) {
                                    let parents = category_parent_map(categories);
                                    value = matched.full_label(&parents);
                                }
                                let targets = grid_commit_targets(selection, column, idx);
                                for &t in &targets {
                                    if let Some(r) = rows.get_mut(t) {
                                        *r.row_category_mut() = value.clone();
                                    }
                                }
                                result.pending_category_commits.extend(targets);
                                *edit_cell = None;
                                *edit_original = None;
                            } else if editing
                                && grid_text_field_committed(ui, &cell.text, egui::Rect::NOTHING)
                            {
                                let targets = grid_commit_targets(selection, column, idx);
                                let value = rows[idx].row_category().to_string();
                                for &t in &targets {
                                    if t != idx {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_category_mut() = value.clone();
                                        }
                                    }
                                }
                                result.pending_category_commits.extend(targets);
                                *edit_cell = None;
                                *edit_original = None;
                            }
                        });
                        paint_row_separator_top(ui, show_row_separator);
                        paint_col_separator_left(ui);
                    });

                    // ── VENDOR ──────────────────────────────────────────────────
                    row_ui.col(|ui| {
                        let column = GridColumn::Vendor;
                        process_grid_column_cell(
                            ui,
                            column,
                            idx,
                            cell_id_by_visual(column, visual_row_index).with("bg"),
                            selection,
                            drag,
                            edit_cell,
                        );
                        ui.horizontal(|ui| {
                            let editing = grid_cell_editing(edit_cell, column, idx);
                            let response = ui_grid_text_edit(
                                ui,
                                rows[idx].row_vendor_mut(),
                                edit_original,
                                edit_cell,
                                cell_id_by_visual(column, visual_row_index),
                                editing,
                            );
                            if editing && cell_has_focus(ui.ctx(), column, idx) {
                                result.active_cell = Some((column, idx));
                            }
                            if editing {
                                if let Some(picked) = show_cell_autocomplete_popup(
                                    ui,
                                    &response,
                                    rows[idx].row_vendor(),
                                    vendor_candidates,
                                    autocomplete_selection,
                                ) {
                                    let targets = grid_commit_targets(selection, column, idx);
                                    for &t in &targets {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_vendor_mut() = picked.clone();
                                        }
                                    }
                                    result
                                        .pending_field_updates
                                        .extend(targets.iter().map(|&t| (t, "vendor")));
                                    *edit_cell = None;
                                    *edit_original = None;
                                }
                            }
                            if response.changed() {
                                result.pending_field_updates.push((idx, "vendor"));
                            }
                            if editing
                                && grid_text_field_committed(ui, &response, egui::Rect::NOTHING)
                            {
                                let targets = grid_commit_targets(selection, column, idx);
                                let value = rows[idx].row_vendor().to_string();
                                for &t in &targets {
                                    if t != idx {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_vendor_mut() = value.clone();
                                        }
                                    }
                                    result.pending_field_updates.push((t, "vendor"));
                                }
                                *edit_cell = None;
                                *edit_original = None;
                            }
                        });
                        paint_row_separator_top(ui, show_row_separator);
                        paint_col_separator_left(ui);
                    });

                    // ── DESCRIPTION ─────────────────────────────────────────────
                    row_ui.col(|ui| {
                        let column = GridColumn::Description;
                        process_grid_column_cell(
                            ui,
                            column,
                            idx,
                            cell_id_by_visual(column, visual_row_index).with("bg"),
                            selection,
                            drag,
                            edit_cell,
                        );
                        ui.horizontal(|ui| {
                            let editing = grid_cell_editing(edit_cell, column, idx);
                            let response = ui_grid_text_edit(
                                ui,
                                rows[idx].row_description_mut(),
                                edit_original,
                                edit_cell,
                                cell_id_by_visual(column, visual_row_index),
                                editing,
                            );
                            if editing && cell_has_focus(ui.ctx(), column, idx) {
                                result.active_cell = Some((column, idx));
                            }
                            if editing {
                                if let Some(picked) = show_cell_autocomplete_popup(
                                    ui,
                                    &response,
                                    rows[idx].row_description(),
                                    description_candidates,
                                    autocomplete_selection,
                                ) {
                                    let targets = grid_commit_targets(selection, column, idx);
                                    for &t in &targets {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_description_mut() = picked.clone();
                                        }
                                    }
                                    result
                                        .pending_field_updates
                                        .extend(targets.iter().map(|&t| (t, "description")));
                                    *edit_cell = None;
                                    *edit_original = None;
                                }
                            }
                            if response.changed() {
                                result.pending_field_updates.push((idx, "description"));
                            }
                            if editing
                                && grid_text_field_committed(ui, &response, egui::Rect::NOTHING)
                            {
                                let targets = grid_commit_targets(selection, column, idx);
                                let value = rows[idx].row_description().to_string();
                                for &t in &targets {
                                    if t != idx {
                                        if let Some(r) = rows.get_mut(t) {
                                            *r.row_description_mut() = value.clone();
                                        }
                                    }
                                    result.pending_field_updates.push((t, "description"));
                                }
                                *edit_cell = None;
                                *edit_original = None;
                            }
                        });
                        paint_row_separator_top(ui, show_row_separator);
                        paint_col_separator_left(ui);
                    });
                }); // body row (virtualized)
            }); // body
        }); // TableBuilder

    scroll_y = scroll_response.state.offset.y;
    // skip the Shift-pan handler while a cell drag-select is active —
    // it calls set_dragged_id, stealing the drag from the cells mid-gesture.
    if drag.is_none() {
        grid_apply_shift_hand_pan(ui.ctx(), &mut scroll_y);
    }
    *scroll_offset = scroll_y;

    // Type-to-start-edit.
    // nothing focused anywhere ⇒ no grid cell holds focus either, so
    // one cheap memory read replaces the old O(selected rows) has_focus scan
    // (which itself did an O(N) raw→visual position per row). Blocks typeahead
    // whenever ANY widget has focus (grid cell, settings TextEdit, window
    // button) — keystrokes go to the focused widget, never to typeahead.
    if edit_cell.is_none() {
        let any_widget_focused = ui.ctx().memory(|m| m.focused().is_some());
        if !any_widget_focused {
            let _ = grid_try_start_edit_from_typing(ui, selection, edit_cell, typeahead);
        }
    }

    // Enter key: if multi-row selection, commit the active cell to all rows
    if let Some((column, idx)) = *edit_cell {
        if ui.input_mut(|inp| inp.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
            let multi = selection
                .as_ref()
                .is_some_and(|s| s.column == column && s.rows.len() > 1);
            let ac_active = !autocomplete_suggestions_list(
                match column {
                    GridColumn::Member => rows.get(idx).map(|r| r.row_member()).unwrap_or(""),
                    GridColumn::Category => rows.get(idx).map(|r| r.row_category()).unwrap_or(""),
                    GridColumn::Vendor => rows.get(idx).map(|r| r.row_vendor()).unwrap_or(""),
                    GridColumn::Description => {
                        rows.get(idx).map(|r| r.row_description()).unwrap_or("")
                    }
                    _ => "",
                },
                match column {
                    GridColumn::Member => member_candidates,
                    GridColumn::Category => category_candidates,
                    GridColumn::Vendor => vendor_candidates,
                    GridColumn::Description => description_candidates,
                    _ => &[],
                },
            )
            .is_empty();
            if multi && !ac_active {
                // Commit current value to all selected rows
                let targets = grid_commit_targets(selection, column, idx);
                match column {
                    GridColumn::Date => {
                        let value = rows
                            .get(idx)
                            .map(|r| r.row_date().to_string())
                            .unwrap_or_default();
                        for &t in &targets {
                            if let Some(r) = rows.get_mut(t) {
                                *r.row_date_mut() = value.clone();
                            }
                            result.pending_field_updates.push((t, "date"));
                        }
                    }
                    GridColumn::Amount => {
                        let value = rows
                            .get(idx)
                            .map(|r| r.row_amount_input().to_string())
                            .unwrap_or_default();
                        for &t in &targets {
                            if let Some(r) = rows.get_mut(t) {
                                *r.row_amount_input_mut() = value.clone();
                            }
                            result.pending_field_updates.push((t, "amount"));
                        }
                    }
                    GridColumn::Member => {
                        let value = rows
                            .get(idx)
                            .map(|r| r.row_member().to_string())
                            .unwrap_or_default();
                        for &t in &targets {
                            if let Some(r) = rows.get_mut(t) {
                                *r.row_member_mut() = value.clone();
                            }
                        }
                        result.pending_member_commits.extend(targets);
                    }
                    GridColumn::Category => {
                        let value = rows
                            .get(idx)
                            .map(|r| r.row_category().to_string())
                            .unwrap_or_default();
                        for &t in &targets {
                            if let Some(r) = rows.get_mut(t) {
                                *r.row_category_mut() = value.clone();
                            }
                        }
                        result.pending_category_commits.extend(targets);
                    }
                    GridColumn::Vendor => {
                        let value = rows
                            .get(idx)
                            .map(|r| r.row_vendor().to_string())
                            .unwrap_or_default();
                        for &t in &targets {
                            if let Some(r) = rows.get_mut(t) {
                                *r.row_vendor_mut() = value.clone();
                            }
                            result.pending_field_updates.push((t, "vendor"));
                        }
                    }
                    GridColumn::Description => {
                        let value = rows
                            .get(idx)
                            .map(|r| r.row_description().to_string())
                            .unwrap_or_default();
                        for &t in &targets {
                            if let Some(r) = rows.get_mut(t) {
                                *r.row_description_mut() = value.clone();
                            }
                            result.pending_field_updates.push((t, "description"));
                        }
                    }
                    GridColumn::Account => {
                        let value = rows
                            .get(idx)
                            .map(|r| r.row_account().to_string())
                            .unwrap_or_default();
                        for &t in &targets {
                            if let Some(r) = rows.get_mut(t) {
                                *r.row_account_mut() = value.clone();
                            }
                            result.pending_field_updates.push((t, "account"));
                        }
                    }
                }
            }
        }
    }

    // Drag
    grid_snap_drag_to_pointer_y(ui, &row_drag_bands, sorted_indices, drag, selection);
    if drag.is_some() {
        ui.ctx().request_repaint();
    }
    finish_grid_drag(drag, ui);

    // Handle double-click edit activation FIRST so that the focus request below
    // fires in the same frame the edit_cell is set (instead of one frame late).
    let pending_edit = ui.ctx().data_mut(|d| {
        d.remove_temp::<Option<(GridColumn, usize)>>(egui::Id::new("pending_edit_cell"))
    });
    if let Some(Some(cell)) = pending_edit {
        *edit_cell = Some(cell);
        ui.ctx().request_repaint();
    }

    // Focus cell — runs after pending_edit_cell is resolved so the request_focus
    // call covers double-click activations in the same frame.
    if let Some(target) = state.pending_focus_target.take() {
        *selection = Some(GridSelection {
            column: target.0,
            rows: vec![target.1],
        });
        *edit_cell = Some(target);
        if let Some(id) = get_cell_id_for_raw_row(target.0, target.1) {
            request_grid_cell_focus(ui, id);
        }
    } else if let Some((col, idx)) = *edit_cell {
        if let Some(id) = get_cell_id_for_raw_row(col, idx) {
            let chevron_id = id.with("chevron");
            ui.memory_mut(|mem| {
                mem.surrender_focus(chevron_id);
                mem.request_focus(id);
            });
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Helper: collect the target rows for a multi-row commit
// ---------------------------------------------------------------------------
// (grid_commit_targets is already in widgets.rs — we re-export nothing, just use it)
