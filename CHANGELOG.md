# Changelog

All notable changes to TwoCents are documented here.

## [Unreleased]

### Added
- **Add expense** button and empty-state action that insert a row into the grid on today's date under the default member and open the amount editor ready to type.
- Income is now part of the seeded category tree, so a fresh install can record credits.
- Keyboard navigation in every suggestion list: the grid's Member, Category, Vendor and Description cells, the member, category and account chevron menus, and the import review's account combo all take ArrowUp/ArrowDown (wrapping) and Enter to accept.
- The Account column is a real combo box: type to filter, arrows to move, Enter or Tab to accept, or open the ▾ for the full list. Picks resolve to the matching account when saved.
- Tab and Shift+Tab walk the whole table — including across row ends — for as long as a cell is selected, being edited, or a menu is open; Escape hands Tab back to the app.
- Settlements has its own date range filter (same presets plus Custom, with a Reset back to All Time) that scopes balances, who-owes-whom, and a new per-category breakdown to the selected window.
- Real calendar pickers for the Custom Start/End fields, reusing the expense grid's date picker, on both Settlements and Analytics Category Breakdown.
- App version shown in the Household support panel.
- LICENSE, SECURITY.md, and CONTRIBUTING.md.

### Changed
- Enter or Tab in a grid cell accepts the highlighted suggestion and moves to the next cell; Enter alone accepts and stays.
- While a suggestion list is open the arrow keys drive the list instead of the text caret; in an empty Amount cell they still move the caret.
- The Duplicates grid is keyboard-operable like the other two grids.
- All app-owned files now live in `<install folder>\data`, including egui's window state, regardless of how the executable was obtained. Cargo builds use `target\<profile>\data`.
- The installer replaces only application files, verifies the release asset's SHA-256 digest, refuses to run while the app is open, and no longer moves the database through `%TEMP%`.
- The Program Files (`-Machine`) install mode was removed; installs are per-user at `%LOCALAPPDATA%\TwoCents`.
- The legacy AppData database is snapshotted with SQLite, integrity-checked, and renamed to `twocents.sqlite.migrated.bak` instead of being raw-copied with errors ignored.
- Database startup failures now show an error window with the data path instead of panicking.
- Analytics opens on All Time so a first import of older statements is visible.
- Release builds are stripped, LTO'd, and locked; release runs tests and no longer fails when the changelog section is missing.

### Fixed
- The edited grid cell now draws a full box on all four sides; the previous rings were painted outside the cell and clipped away, leaving a stray line inside Amount, Member, Category, Vendor, Description, and Account.
- Tab escaped the table and started cycling the app's buttons, and Enter never accepted the highlighted suggestion: the arrow handler matched every unmodified key press and swallowed the Enter and Tab cases. The table now keeps the keyboard while a cell is engaged.
- Suggestion lists could not be driven from the keyboard: the arrows were read after the focused text editor had already taken them as caret movement, so the highlight never moved and the list could collapse. Arrows and Enter are now captured before any widget runs.
- Add expense opens the new row's own amount cell instead of whichever expense happened to sit at that position in the sheet.
- Committing a date re-sorts the grid and the row follows to its new position, staying selected.
- Closing the window during the expense edit debounce no longer discards pending edits, and a failed save is retried instead of dropped.
- Deleting every category no longer re-seeds the defaults on the next launch.
- CSV imports are atomic: a failed save rolls back the account and every row and reports the error in the UI.
- Manual expenses can be added on a fresh install with no accounts (no foreign-key failure).

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
