# Contributing

## Prerequisites

- Rust stable (1.85+)
- On Windows: Visual Studio Build Tools with the MSVC target and a C compiler
  (`rusqlite` builds SQLite from C source)

## Before opening a pull request

```powershell
cargo fmt --check
cargo clippy --all-targets
cargo test
```

CI runs exactly these three commands on every push and pull request.

## Project rules

- All persistent files live under `<install folder>\data`. No new code may
  write to `%APPDATA%`, `%TEMP%`, the current directory, or the home directory.
- Schema changes need an idempotent migration recorded in `app_migrations`,
  wrapped so an existing user database is never partially upgraded.
- User-visible failures must be shown in the UI; `expect`, `unwrap`, and
  `eprintln!` are not acceptable on a path that can lose data.
- No new dependencies without a reason that standard library, an existing
  dependency, or a native platform feature cannot cover.
