//! Component builders — the only place chrome is constructed.
//!
//! Spacing/geometry live in `theme_tokens`; ALL colors come from `theme`
//! (the palette is declared and derived there — the app's one color source).
//! Every card, button, modal, and input in the app should be built through
//! these helpers — never construct a raw `egui::Frame` for chrome.
//!
//! Add new chrome primitives here rather than constructing `egui::Frame` at call sites.
use crate::ui::theme;
use crate::ui::theme_tokens::*;
use eframe::egui::{self, Color32, Frame, Margin, Stroke, Vec2};

// ---- Color access -----------------------------------------------------------
//
// colors live ONLY in `theme::palette()`. The chrome builders below
// read the private helper; UI call sites use `crate::ui::theme::*` directly.

fn palette() -> theme::Palette {
    theme::palette()
}

// ---- Spacing helpers --------------------------------------------------------

pub fn vspace(amount: f32) -> f32 {
    amount
}
pub fn hspace(amount: f32) -> f32 {
    amount
}

// ---- Cards ------------------------------------------------------------------

/// Standard content card: 1px border, 6px radius, 12px inner padding.
/// Used for the budget summary cards, settings panels, household members, etc.
pub fn card(ui: &egui::Ui) -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_primary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_md())
        .inner_margin(Margin::same(SPACE_3 as i8))
}

/// Subtle card — used for sections inside a parent card or a sub-panel.
pub fn card_subtle(_ui: &egui::Ui) -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_secondary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_md())
        .inner_margin(Margin::same(SPACE_3 as i8))
}

/// Wrapper for `themed_panel_frame` callers that hold a `&Style`
/// but have no `Ui` to read the palette from.
pub fn card_subtle_dummy() -> Frame {
    card_subtle_dummy_inner()
}
fn card_subtle_dummy_inner() -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_secondary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_md())
        .inner_margin(Margin::symmetric(SPACE_3 as i8, SPACE_2 as i8))
}

/// shared chrome for the expense / import / duplicates grid tables.
/// All three call sites previously built their own Frame inline; this guarantees
/// the three grids have identical chrome (1px border, 6px radius, 2px inner
/// margin) so the user can't tell them apart visually until they read the data.
pub fn grid_table_frame(ui: &egui::Ui) -> Frame {
    let p = palette();
    // Use the table's `extreme_bg` (the deepest surface in the theme) so the
    // grid reads as a recessed plane relative to the surrounding card chrome.
    Frame::NONE
        .fill(theme::bg_primary())
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_md())
        .inner_margin(Margin::same(2))
}

/// Repaint a [`grid_table_frame`]'s border over its content: scrolled inner
/// rows can bleed past the scroll clip (clip_rect_margin) and cover the
/// frame's bottom edge when the table is scrolled to the end. Call this with
/// the frame's `Response::rect` after `.show(...)`.
pub fn repaint_grid_frame_stroke(ui: &egui::Ui, outer_rect: egui::Rect) {
    ui.painter().rect_stroke(
        outer_rect,
        radius_md(),
        Stroke::new(BORDER_W, palette().border),
        egui::StrokeKind::Inside,
    );
}

/// Filled "well" — used for the terminal dock at the bottom.
pub fn well(ui: &egui::Ui) -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_secondary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_sm())
        .inner_margin(Margin {
            left: SPACE_2 as i8,
            right: SPACE_2 as i8,
            top: SPACE_1 as i8,
            bottom: SPACE_1 as i8,
        })
}

// ---- Modals -----------------------------------------------------------------

/// Modal window frame: 1px border, 8px radius, 24px padding, drop shadow.
/// Reads the active palette from `theme` (not from `ui`) so it can be built
/// at any time, including before a `ui` exists in the call chain.
pub fn modal() -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_primary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_lg())
        .inner_margin(Margin::same(SPACE_5 as i8))
        .shadow(egui::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(if p.is_dark { 80 } else { 30 }),
        })
}

/// Modal header bar — 1px bottom border, 16px vertical padding.
pub fn modal_header(ui: &egui::Ui) -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_primary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .inner_margin(Margin {
            left: 0,
            right: 0,
            top: SPACE_3 as i8,
            bottom: SPACE_3 as i8,
        })
}

// ---- Inputs -----------------------------------------------------------------

/// Input field frame: 1px border, 4px radius, 10×6 padding.
pub fn input(ui: &egui::Ui) -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_primary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_sm())
        .inner_margin(Margin {
            left: SPACE_2 as i8,
            right: SPACE_2 as i8,
            top: 6,
            bottom: 6,
        })
}

/// Input field in a "subtle" context — no fill, just a hover/focus border.
pub fn input_ghost(ui: &egui::Ui) -> Frame {
    let p = palette();
    Frame::NONE
        .fill(p.bg_secondary)
        .stroke(Stroke::new(BORDER_W, p.border))
        .corner_radius(radius_sm())
        .inner_margin(Margin {
            left: SPACE_2 as i8,
            right: SPACE_2 as i8,
            top: 6,
            bottom: 6,
        })
}

// ---- Buttons ----------------------------------------------------------------
//
// `egui::Button` already supports fill/stroke/text_color. These helpers return
// the visual configuration; call sites do `ui.add(egui::Button::new(label).fill(...))`.
//
// button is just (fill, stroke, text_color, padding). Five flavors:
//   * `button_subtle` — default 13-theme secondary look
//   * `button_primary` — accent fill, white text
//   * `button_ghost`   — borderless, hover-only background
//   * `button_danger`  — error-tinted stroke for destructive actions
//   * `button_icon`    — small square ghost for chevron/close buttons

pub struct ButtonStyle {
    pub fill: Color32,
    pub stroke: Stroke,
    pub text_color: Color32,
    pub padding: Vec2,
}

pub fn button_subtle(ui: &egui::Ui) -> ButtonStyle {
    let p = palette();
    ButtonStyle {
        fill: p.bg_secondary,
        stroke: Stroke::new(BORDER_W, p.border),
        text_color: p.text_primary,
        padding: Vec2::new(SPACE_3, SPACE_2),
    }
}

pub fn button_primary(ui: &egui::Ui) -> ButtonStyle {
    let p = palette();
    ButtonStyle {
        fill: p.accent,
        stroke: Stroke::new(BORDER_W, p.accent),
        text_color: p.selection_fg,
        padding: Vec2::new(SPACE_3, SPACE_2),
    }
}

pub fn button_ghost(ui: &egui::Ui) -> ButtonStyle {
    let p = palette();
    ButtonStyle {
        fill: Color32::TRANSPARENT,
        stroke: Stroke::NONE,
        text_color: p.text_secondary,
        padding: Vec2::new(SPACE_2, SPACE_1),
    }
}

pub fn button_danger(ui: &egui::Ui) -> ButtonStyle {
    let p = palette();
    ButtonStyle {
        fill: p.bg_secondary,
        stroke: Stroke::new(BORDER_W, p.error),
        text_color: p.error,
        padding: Vec2::new(SPACE_3, SPACE_2),
    }
}

pub fn button_icon(ui: &egui::Ui) -> ButtonStyle {
    let p = palette();
    ButtonStyle {
        fill: Color32::TRANSPARENT,
        stroke: Stroke::NONE,
        text_color: p.text_secondary,
        padding: Vec2::splat(SPACE_1),
    }
}

// ---- Tabs -------------------------------------------------------------------
//
// Notion-style underline tabs. Returned as a tuple of (active_text, inactive_text)
// so the call site can render them with a manual underline.
pub fn tab_label(ui: &egui::Ui, text: &str, active: bool) -> egui::RichText {
    let p = palette();
    let color = if active {
        p.text_primary
    } else {
        p.text_secondary
    };
    egui::RichText::new(text).size(14.0).color(color)
}

pub fn tab_underline(ui: &egui::Ui, rect: egui::Rect, active: bool) {
    if !active {
        return;
    }
    let p = palette();
    let y = rect.bottom() - 1.5;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(2.0_f32, p.accent),
    );
}

/// Complete tab button: themed label with a 2px accent underline when
/// `active` is true. Keeps every tab strip visually identical; the caller
/// lays these out in a `horizontal()` row.
pub fn tab_label_button(ui: &mut egui::Ui, active: bool, label: &str) -> egui::Response {
    let padding = egui::Margin::symmetric(SPACE_2 as i8, 0);
    let frame = egui::Frame::NONE.inner_margin(padding);
    let response = frame
        .show(ui, |ui| {
            ui.add(
                egui::Label::new(tab_label(ui, label, active))
                    .selectable(false)
                    .sense(egui::Sense::click()),
            )
            .interact(egui::Sense::click())
        })
        .inner
        .interact(egui::Sense::click());
    tab_underline(ui, response.rect, active);
    response
}

// ---- Progress bar -----------------------------------------------------------

/// return a color `amount` darker (in HSVA value space)
/// than the input. Used by `progress_bar` and the chart bars so each
/// colored segment's outline matches its own hue rather than the
/// theme's neutral border. Clamp at 0.10 so very dark colors don't
/// collapse to black.
pub fn darker(color: Color32, amount: f32) -> Color32 {
    let mut hsva = egui::ecolor::Hsva::from(color);
    hsva.v = (hsva.v - amount).clamp(0.10, 1.0);
    Color32::from(hsva)
}

/// progress bar component. Pure data-in, paint-out — caller picks
/// the rect, this draws the track + fill + percent label. Used by the budget
/// sheet's per-row and parent-row progress cells. The bar fill color shifts:
/// ≤80% → success, ≤100% → warning, >100% → error.
pub fn progress_bar(ui: &mut egui::Ui, cell_rect: egui::Rect, percent: f32) {
    let bar_left = cell_rect.left() + 6.0;
    let avail_bar = (cell_rect.width() - 6.0 - 4.0).max(4.0);
    let label_w = 34.0;
    let bar_w = (avail_bar - label_w).max(4.0);
    let bar_h = 8.0;
    let bar_top = cell_rect.center().y - bar_h * 0.5;
    let bar_rect =
        egui::Rect::from_min_size(egui::pos2(bar_left, bar_top), egui::vec2(bar_w, bar_h));
    let p = palette();
    // Track uses the themed bg_stroke (a softer, slightly-tinted surface) so it
    // reads as recessed relative to the row.
    ui.painter().rect_filled(bar_rect, 4.0, p.border_strong);
    if percent > 0.0 {
        let fill_w = bar_w * percent.min(1.0);
        let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_w, bar_h));
        let fill_color = if percent <= 0.8 {
            p.success
        } else if percent <= 1.0 {
            p.warning
        } else {
            p.error
        };
        // 1px outline on the colored fill in the same color,
        // 18% darker. Defines the leading edge of the bar and gives a
        // clean silhouette when the bar reaches 100% (track no longer
        // visible behind it). The track itself stays borderless.
        let fill_outline = darker(fill_color, 0.18);
        ui.painter()
            .add(egui::Shape::Rect(egui::epaint::RectShape::new(
                fill_rect,
                egui::CornerRadius::same(4),
                fill_color,
                egui::Stroke::new(1.0_f32, fill_outline),
                egui::StrokeKind::Inside,
            )));
    }
    let pct_label = format!("{:.0}%", percent * 100.0);
    let pct_color = if percent > 1.0 {
        p.error
    } else {
        p.text_primary
    };
    ui.painter().text(
        egui::pos2(bar_rect.right() + 4.0, bar_rect.center().y),
        egui::Align2::LEFT_CENTER,
        &pct_label,
        egui::FontId::proportional(11.0),
        pct_color,
    );
}

/// visual swatch with a 1px themed border. Used in the budget
/// parent-row and the per-row swatches. Not interactive — for visual identity
/// only. The interactive version (in households/popups) is `color_swatch_button`
/// in `widgets.rs`.
pub fn color_swatch_decorated(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    border: egui::Color32,
) {
    let inner = rect.shrink(1.0);
    painter.circle_filled(inner.center(), inner.width() * 0.5, color);
    painter.circle_stroke(
        inner.center(),
        inner.width() * 0.5,
        egui::Stroke::new(1.0_f32, border),
    );
}

#[allow(dead_code)]
fn _components_use_no_dead_warning() {
    // silence "unused" if both color_swatch_decorated and
    // progress_bar aren't reached from a call site yet.
    let _ = (color_swatch_decorated, progress_bar);
}

// ---- Empty state ------------------------------------------------------------

/// "No data yet" / "No selection" placeholder. Caller supplies the headline
/// and subtext; we draw the icon, message, and optional CTA button slot.
pub fn empty_state(ui: &mut egui::Ui, headline: &str, subtext: &str) {
    let p = palette();
    ui.vertical_centered(|ui| {
        ui.add_space(SPACE_6);
        // 64x64 muted icon placeholder — uses a hollow circle to stand in for an
        // empty-state illustration without a new icon dependency.
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(SPACE_7), egui::Sense::hover());
        ui.painter()
            .circle_stroke(rect.center(), 24.0, Stroke::new(1.5_f32, p.text_faint));
        ui.add_space(SPACE_2);
        ui.label(
            egui::RichText::new(headline)
                .size(16.0)
                .strong()
                .color(p.text_primary),
        );
        ui.add_space(SPACE_1);
        ui.label(
            egui::RichText::new(subtext)
                .size(13.0)
                .color(p.text_secondary),
        );
        ui.add_space(SPACE_4);
    });
}

// ---- Text styles ------------------------------------------------------------
//
// these are the *only* places that should pick a font size. A
// 14-point body is a 14-point body everywhere.

pub fn text_xs(ui: &egui::Ui) -> egui::RichText {
    egui::RichText::new("")
        .size(11.0)
        .color(palette().text_secondary)
}

pub fn text_sm(ui: &egui::Ui) -> egui::RichText {
    egui::RichText::new("")
        .size(13.0)
        .color(palette().text_primary)
}

pub fn text_body(ui: &egui::Ui) -> egui::RichText {
    egui::RichText::new("")
        .size(14.0)
        .color(palette().text_primary)
}

pub fn text_md(ui: &egui::Ui) -> egui::RichText {
    egui::RichText::new("")
        .size(15.0)
        .strong()
        .color(palette().text_primary)
}

pub fn text_lg(ui: &egui::Ui) -> egui::RichText {
    egui::RichText::new("")
        .size(20.0)
        .strong()
        .color(palette().text_primary)
}

pub fn text_xl(ui: &egui::Ui) -> egui::RichText {
    egui::RichText::new("")
        .size(28.0)
        .strong()
        .color(palette().text_primary)
}

pub fn text_2xl(ui: &egui::Ui) -> egui::RichText {
    egui::RichText::new("")
        .size(40.0)
        .strong()
        .color(palette().text_primary)
}

// Convenience: call-site wants a label, not a builder pattern.
pub fn label(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(text)
            .size(14.0)
            .color(palette().text_primary),
    )
}

pub fn label_muted(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(text)
            .size(13.0)
            .color(palette().text_secondary),
    )
}

pub fn label_faint(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(text)
            .size(11.0)
            .color(palette().text_faint),
    )
}

pub fn label_strong(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(text)
            .size(15.0)
            .strong()
            .color(palette().text_primary),
    )
}

pub fn heading_md(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(text)
            .size(15.0)
            .strong()
            .color(palette().text_primary),
    )
}

pub fn heading_md_text(ui: &egui::Ui, text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(15.0)
        .strong()
        .color(palette().text_primary)
}

pub fn heading_lg(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(text)
            .size(20.0)
            .strong()
            .color(palette().text_primary),
    )
}

/// Small uppercase section header. Reads as a quiet marker, not a title.
/// Use inside cards and panels to mark sub-sections.
pub fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(11.0)
            .strong()
            .color(palette().text_secondary)
            .extra_letter_spacing(0.5),
    );
}

pub fn heading_lg_text(ui: &egui::Ui, text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(20.0)
        .strong()
        .color(palette().text_primary)
}

pub fn heading_xl(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.label(
        egui::RichText::new(text)
            .size(28.0)
            .strong()
            .color(palette().text_primary),
    )
}

pub fn heading_xl_text(ui: &egui::Ui, text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(28.0)
        .strong()
        .color(palette().text_primary)
}
