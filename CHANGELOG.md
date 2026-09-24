# Changelog

All notable changes to TwoCents are documented here.

## [Unreleased]

## [v0.1.2] - 2026-09-24

### Added
- Year-scoped budget planning with midyear actual-spend catch-up, selected-period suffix propagation, calendar-week boundaries, atomic persistence, and full-year Copy Limits.
- Persistent Household and Settlements undo history with contextual Ctrl+Z and destructive-action confirmations.
- Responsive unified Household Members and Categories layout with a Default member marker and adaptive two-column cards.
- Improved budget currency editing with select-all, Enter-to-save, Escape/click-away cancel, strict parsing, and safe limits.
- Compact content-sized destructive confirmations that keep the underlying app visible.

### Changed
- Budgets now use fresh plans instead of stale cached propagation, with quarterly, monthly, and weekly values derived consistently.
- Analytics period labels, chart grids, tapered connectors, and transient status placement were refined.
- Household and Settlement confirmations now preserve the surrounding interface while remaining interaction-protected.

### Fixed
- Prevented budget values from bleeding across calendar years.
- Fixed stale weekly/monthly/quarterly propagation and boundary-week handling.
- Fixed destructive confirmations appearing oversized or hiding the underlying Household/Settlements view.

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
