# TwoCents

A modern, high-performance finance and budgeting desktop application built in Rust, powered by `egui` and `eframe`. TwoCents helps couples track their shared household expenses, manage categories, assign members, review imported bank statements, and gain rich analytical insights into their spending habits—completely offline and backed by a local SQLite database.

## Technical Stack

- **Graphics & GUI**: `egui` & `eframe` (High-performance immediate-mode desktop framework)
- **Data Grid**: `egui_extras` (Striped, resizable, and virtualized custom spreadsheets)
- **Local Database**: SQLite via `rusqlite` (with bundled static linking)
- **Date & Time**: `chrono` & `jiff`
- **Analytics Charts**: `plotters` (Vector-based chart generation with bitmap engines)
- **Language**: Rust (Edition 2021)

---

## Core Features

### 1. Unified Interactive Spreadsheet Grid
Both the main **Expenses** panel and the **CSV Statement Import review** modal share a single, high-performance grid component that supports:
- **Interactive Editors**: Custom inline date pickers, amount inputs with automatic numeric sanitization, and text fields.
- **Smart Auto-Completion**: Typeahead candidates for Members, Categories, Vendors, and Descriptions, with group headers and interactive chevron dropdown menus.
- **Advanced Multi-Row Selection**: Copy, paste, bulk edit, and drag-to-fill features.
- **Smooth Navigation**: Arrow key navigation, Tab/Shift-Tab focus shifting, and Escape cancellation.
- **Intuitive Gestures**: Drag selection and Shift + Drag-panning across large lists.

### 2. Local Database Transaction Layer
Uses an embedded SQLite database (`TwoCentsApp` manages connections statically) ensuring that data is persisted instantaneously and loaded instantly upon launching the app.

### 3. Beautiful Financial Analytics
Generates clean analytics and budget/expense breakdowns over time, rendered with modern font rendering and dynamic sizing layout engines.

---

## Getting Started

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (Stable channel)
- Visual Studio Build Tools (for C++ compilation on Windows / MSVC target)

### Building and Running
Two-click launch scripts are provided for local Windows environments:

1. **Rebuild and Run**:
   Double-click or execute the launcher batch script to rebuild the application from scratch and launch the executable:
   ```powershell
   .\restart_app.bat
   ```

2. **Manual Compilation**:
   Or run the compiler tool directly:
   ```powershell
   cargo build --release
   cargo run --release
   ```

---

## Project Structure

- `src/` - Entire Rust codebase of the application:
  - `src/main.rs` - Application entry point, window configuration, and event hook managers.
  - `src/models.rs` - Structs and the core `GridRow` trait.
  - `src/db.rs` - SQLite schema definitions, loading queries, and database updates.
  - `src/ui/` - Layout sections and panels:
    - `src/ui/grid.rs` - The unified spreadsheet component.
    - `src/ui/expenses.rs` - The main expenses manager view.
    - `src/ui/import_modal.rs` - Staged CSV statement imports reviewer.
    - `src/ui/popups.rs` - Decoupled context picker menus, colors, and dropdown frames.
    - `src/ui/widgets.rs` - Custom cell layout builders.
