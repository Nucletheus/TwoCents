# Changelog

All notable changes to TwoCents are documented here.

## [Unreleased]

## [v0.1.1] - 2026-09-23

### Added
- Self-contained installs: the executable and its `data` folder (SQLite database) live side by side; deleting the folder removes everything.
- Program Files install option (`-Machine`, elevated PowerShell) with write access granted to the data subfolder only.
- Household tab: a Support panel with a Buy Me a Coffee link.
- Buy Me a Coffee badge in the README header and GitHub Sponsor button.

### Changed
- Databases from older versions (stored in `%APPDATA%\TwoCents`) are imported automatically on first launch.
- Installer preserves the data folder across reinstalls and updates.

## [v0.1.0] - 2026-09-23

### Added
- First public release.
- Expense spreadsheet with inline editors, autocomplete, multi-row selection, and full keyboard navigation.
- CSV statement import with staged review and duplicate resolution.
- Envelope budgets at weekly/monthly/quarterly/yearly granularity with automatic propagation and copy-from-previous.
- Analytics: Category Breakdown donut, Budget vs Actual paired bars, Period Comparison A/B, shared Category/Cost sort, dollar tooltips.
- Settlements: per-category split percentages, live balances, who-owes-whom suggestions.
- 13 theme presets with dark/light/system variants, live hover preview, and persistence.
- Fully offline; data in one local SQLite file.
