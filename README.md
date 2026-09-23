# TwoCents

<p align="center">
  <img src="docs/logo.png" alt="TwoCents logo" width="60%">
</p>

<p align="center">
  <img src="docs/screenshots/expenses.png" alt="TwoCents, the expenses tab with a spreadsheet grid of transactions" width="100%">
</p>

TwoCents is a personal finance desktop app for couples. It tracks shared expenses, splits categories between household members, manages budgets, and calculates who owes whom. It runs fully offline and stores everything in one local SQLite file.

[![Platform](https://img.shields.io/badge/platform-Windows-blue)](#) [![Rust](https://img.shields.io/badge/made%20with-Rust-orange)](#) [![Storage](https://img.shields.io/badge/storage-local%20SQLite-green)](#) [![Tracking](https://img.shields.io/badge/tracking-none-success)](#) [![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-FFDD00?logo=buy-me-a-coffee&logoColor=black)](https://www.buymeacoffee.com/Nucletheus)

<!-- TODO: demo video embed goes here -->

## About this project

This is a personal project and a minimum viable product, built for my own household. It works, but expect rough edges, limited features, and changes along the way. If you find it useful, that's great.

## Features

### Expenses
A spreadsheet-style grid shared by the expense sheet and the import review modal:

- Inline editors for date (with picker), amount, member, category, vendor, description, and account.
- Autocomplete with suggestions for members, categories, vendors, and descriptions.
- Multi-row selection: click or drag a column, Shift+drag to pan, copy/paste, and bulk edit. Enter applies an edit to every selected row.
- Keyboard navigation: arrow keys, Tab/Shift-Tab, Escape to cancel.

### CSV import
- Import a bank-exported CSV statement and review every staged row before it is saved.
- Resolve duplicates during review, or audit existing data in the Duplicates view.

### Budgets
- Per-category limits at weekly, monthly, quarterly, or yearly granularity.
- Editing a limit automatically propagates to other periods.
- Copy Limits from the previous period or year in one click.
- Progress bars per category with over/under/unbudgeted filters.
- Budget prices are year-scoped, so viewing a past year never borrows the current year's numbers.

<p align="center"><img src="docs/screenshots/budgets.png" alt="Budgets tab with allocation limits and progress bars" width="100%"></p>

### Analytics
Three chart views over the same filters (members, vendors, dates):

- **Category Breakdown**: donut chart with a clickable category legend.
- **Budget vs Actual**: paired horizontal bars per category on a log dollar axis. Tooltips show budget, actual, and the signed variance.
- **Period Comparison**: compare any two periods with direction-colored connectors and a signed difference.
- One shared Category/Cost sort applies across all three charts.

<p align="center"><img src="docs/screenshots/analytics_bva.png" alt="Budget vs Actual chart" width="100%"></p>
<p align="center"><img src="docs/screenshots/analytics_pc.png" alt="Period Comparison chart" width="100%"></p>

### Settlements
- Assign split percentages per category (for example, Groceries 50/50, Fuel 70/30), with equal-split and clear shortcuts.
- Live balance per member over all split-covered expenses.
- Minimal who-owes-whom payment suggestions.

<p align="center"><img src="docs/screenshots/settlements.png" alt="Settlements tab with split percentages and balances" width="100%"></p>

### Theming
- 13 presets: One Dark, Nord, Dracula, Catppuccin (Mocha and Macchiato), GitHub, Solarized, Tokyonight, Everforest, Gruvbox, Kanagawa, Ayu, Matrix.
- Dark, light, and follow-system variants for each preset.
- Hovering a preset previews it live; the choice is saved and restored on restart.
- All colors in the app come from one palette in the theme module.

<p align="center"><img src="docs/screenshots/theming.png" alt="Theme selector dropdown" width="100%"></p>

## Install

Everything is contained in one folder: the executable and its `data` subfolder (the SQLite database) live side by side. Deleting the folder removes the app and its data. No Rust toolchain needed for the installs below.

### Current user (default)
Installs to `%LOCALAPPDATA%\TwoCents` and adds a Start Menu shortcut:

```powershell
irm https://raw.githubusercontent.com/Nucletheus/TwoCents/main/install.ps1 | iex
```

### Program Files (all users of this PC)
Installs to `C:\Program Files\TwoCents` with a Start Menu shortcut for every user. Run in an **elevated (Run as Administrator) PowerShell**:

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/Nucletheus/TwoCents/main/install.ps1))) -Machine
```

Windows prevents normal users from writing to Program Files, so the installer grants write access to the `data` subfolder only; the executable itself stays read-only as intended.

> **SmartScreen note:** the Windows binary is not code-signed, so the first launch may show "Windows protected your PC". Click **More info → Run anyway** to proceed, or verify the download against the release artifacts.

### Build from source
Requires the [Rust toolchain](https://rustup.rs/) (stable) and, on Windows, Visual Studio Build Tools (MSVC target).

```powershell
git clone https://github.com/Nucletheus/TwoCents.git
cd TwoCents
cargo run --release
```

On Windows, `restart_app.bat` rebuilds and relaunches in one step. Builds run from `cargo` keep their data in `%APPDATA%\TwoCents` unless a `data` folder exists next to the executable.

## Getting Started

1. Launch the app. It creates its database on first run.
2. Household tab: add household members. This drives splits and balances.
3. Expenses: add rows manually or import a CSV statement.
4. Settlements: set split percentages per category.
5. Budgets: pick a timeframe and allocate limits.
6. Analytics: pick a chart and hover the bars for tooltips.
7. Household tab: if TwoCents is useful to you, there is a Buy Me a Coffee link under Support.

All data lives in one file inside the install folder: `<install dir>\data\twocents.sqlite`. Back it up or move it as you like. Updating from an older version that stored data in `%APPDATA%\TwoCents` is automatic: the existing database is imported on first launch.

## Tech Stack

| Component | Library |
|---|---|
| Language | Rust (Edition 2021) |
| GUI | [egui](https://github.com/emilk/egui) + [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) |
| Charts | [egui_plot](https://github.com/emilk/egui/tree/master/crates/egui_plot) |
| Database | SQLite via [rusqlite](https://github.com/rusqlite/rusqlite) (bundled) |
| Time | [chrono](https://github.com/chronotope/chrono), [jiff](https://github.com/burntsushi/jiff) |
| Extras | egui_extras (date picker), custom virtualized grid |

## Project Structure

```
src/
├── main.rs          # App state, budget pricing engine, window/event plumbing
├── models.rs        # Domain structs and the shared GridRow trait
├── db.rs            # SQLite schema, migrations, queries
└── ui/
    ├── theme.rs     # Presets and the derived palette, the single color source
    ├── components.rs# Chrome builders: cards, buttons, inputs, progress bars
    ├── grid.rs      # The unified spreadsheet engine
    ├── expenses.rs / budgets.rs / settlements.rs / households.rs
    ├── import_modal.rs / duplicates_modal.rs / popups.rs / widgets.rs
    └── analytics/   # Category breakdown, Budget vs Actual, Period Comparison
```

## License

Copyright © 2026 Nucletheus. All rights reserved.

Source is provided for review and personal use; redistribution or reuse requires permission.
