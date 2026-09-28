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

CI runs the same three commands on every push and pull request (plus `--locked`
for clippy and tests).

## Branch protection and how `main` actually gates

`main` has one required status check: **`check`**, the job in `.github/workflows/ci.yml`.
"Do not allow bypassing the above settings" is **off**, so an admin push lands on
`main` immediately and CI runs *afterwards* — the push output prints
`Bypassed rule violations … Required status check "check" is expected`, and that
is a notice, not a failure.

Two consequences worth knowing:

- A direct push to `main` gets no green-before-land guarantee. Watch the run
  yourself: `gh run list` or the Actions tab. CI on a full build takes ~3 min.
- Anything that can only be caught by CI (fmt, clippy, a failing test) reaches
  `main` regardless. For work you want gated, push a branch and open a PR — the
  required check then blocks the merge until `check` passes.

## Cutting a release

1. Bump `version` in `Cargo.toml`, then run `cargo test` **without** `--locked`
   so `Cargo.lock` picks up the new version. A version bump legitimately changes
   the lock file, and `--locked` fails until you commit that update.
2. Move the accumulated `## [Unreleased]` changelog entries under a
   `## [vX.Y.Z] - YYYY-MM-DD` heading. The release job extracts that exact
   section for the tag, and now only warns when it is missing.
3. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`. The tag triggers
   the build, the ZIP and the GitHub release.

## Project rules

- All persistent files live under `<install folder>\data`. No new code may
  write to `%APPDATA%`, `%TEMP%`, the current directory, or the home directory.
- Schema changes need an idempotent migration recorded in `app_migrations`,
  wrapped so an existing user database is never partially upgraded.
- User-visible failures must be shown in the UI; `expect`, `unwrap`, and
  `eprintln!` are not acceptable on a path that can lose data.
- No new dependencies without a reason that standard library, an existing
  dependency, or a native platform feature cannot cover.
