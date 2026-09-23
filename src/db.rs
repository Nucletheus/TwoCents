use crate::models::*;
use eframe::egui::{ecolor::Hsva, Color32};
use rusqlite::{params, Connection};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Legacy data location: the roaming AppData folder used before installs
/// became self-contained.
fn legacy_db_path() -> PathBuf {
    let base = env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join("TwoCents").join("twocents.sqlite")
}

/// Data directory for the database. Self-contained installs keep everything
/// in one folder: when the installer has created a `data` folder next to the
/// executable, the database lives there. Otherwise (dev builds, legacy
/// installs) the roaming AppData location is used.
pub fn data_dir() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let portable = dir.join("data");
            if portable.is_dir() {
                return portable;
            }
        }
    }
    env::var("APPDATA")
        .map(|base| PathBuf::from(base).join("TwoCents"))
        .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn db_path() -> PathBuf {
    data_dir().join("twocents.sqlite")
}

/// One-time migration: when the self-contained `data` folder exists but has
/// no database yet, and a legacy AppData database does, copy it in so
/// existing users keep their data after updating.
fn migrate_legacy_db() {
    let portable = db_path();
    if portable.is_file() {
        return;
    }
    let legacy = legacy_db_path();
    if !legacy.is_file() {
        return;
    }
    if !portable.parent().is_some_and(|dir| dir.is_dir()) {
        return;
    }
    let _ = fs::copy(&legacy, &portable);
    for suffix in ["-wal", "-shm"] {
        let from = legacy.with_file_name(format!(
            "{}{suffix}",
            legacy
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
        ));
        if from.is_file() {
            let _ = fs::copy(
                &from,
                portable.with_file_name(format!(
                    "{}{suffix}",
                    portable
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                )),
            );
        }
    }
}

pub fn open_database() -> rusqlite::Result<Connection> {
    migrate_legacy_db();
    let path = db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?;
    }

    let conn = Connection::open(path)?;
    conn.execute_batch(
        "
    CREATE TABLE IF NOT EXISTS households (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE TABLE IF NOT EXISTS accounts (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL DEFAULT 1 REFERENCES households(id),
      name TEXT NOT NULL,
      kind TEXT NOT NULL,
      balance_cents INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE IF NOT EXISTS expenses (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL DEFAULT 1 REFERENCES households(id),
      account_id INTEGER REFERENCES accounts(id),
      description TEXT NOT NULL,
      vendor TEXT,
      category TEXT NOT NULL,
      amount_cents INTEGER NOT NULL,
      date TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS categories (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL DEFAULT 1 REFERENCES households(id),
      name TEXT NOT NULL,
      parent_id INTEGER REFERENCES categories(id) ON DELETE CASCADE,
      color_rgb INTEGER
    );
    CREATE TABLE IF NOT EXISTS vendor_category_rules (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL DEFAULT 1 REFERENCES households(id),
      vendor_pattern TEXT NOT NULL,
      category TEXT NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      UNIQUE(household_id, vendor_pattern)
    );
    CREATE TABLE IF NOT EXISTS app_settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );
    ",
    )?;
    let _ = conn.execute("ALTER TABLE expenses ADD COLUMN vendor TEXT", []);
    let _ = conn.execute("ALTER TABLE categories ADD COLUMN color_rgb INTEGER", []);
    migrate_category_colors(&conn)?;
    migrate_category_hierarchy(&conn)?;
    migrate_households(&conn)?;
    migrate_category_defaults_v2(&conn)?;
    migrate_category_palette_colors_v3(&conn)?;
    migrate_household_members(&conn)?;
    migrate_member_colors(&conn)?;
    migrate_theme_accent_members_v4(&conn)?;
    migrate_theme_accent_categories_v4(&conn)?;
    migrate_budgets_v5(&conn)?;
    migrate_budget_snapshots_v6(&conn)?;
    migrate_analytics_filters_v7(&conn)?;
    migrate_category_palette_vibrant_v8(&conn)?;
    migrate_category_splits_v9(&conn)?;
    migrate_sign_protected_v10(&conn)?;
    migrate_demo_cleanup_v11(&conn)?;
    migrate_category_exclusion_v12(&conn)?;

    // legacy DBs carry UNIQUE(vendor_pattern) only — the vendor-rule
    // upserts target (household_id, vendor_pattern), which matches nothing and
    // fails every import with "ON CONFLICT clause does not match...". One
    // idempotent index fixes both upsert sites (import + grid learning).
    let _ = conn.execute(
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_vendor_rules_household_pattern ON vendor_category_rules(household_id, vendor_pattern)",
    [],
  );

    seed_default_categories(&conn, 1)?;
    Ok(conn)
}

pub fn migrate_households(conn: &Connection) -> rusqlite::Result<()> {
    let household_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM households", [], |row| row.get(0))?;
    if household_count == 0 {
        conn.execute("INSERT INTO households (name) VALUES ('My Household')", [])?;
    }
    for table in [
        "accounts",
        "expenses",
        "categories",
        "vendor_category_rules",
    ] {
        let _ = conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN household_id INTEGER"),
            [],
        );
        conn.execute(
            &format!("UPDATE {table} SET household_id = 1 WHERE household_id IS NULL"),
            [],
        )?;
    }
    Ok(())
}

pub fn load_active_household(conn: &Connection) -> rusqlite::Result<(i64, String)> {
    conn.query_row(
        "SELECT id, name FROM households ORDER BY id LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

/// tiny app-wide key/value settings (theme preset, variant
/// mode) — persisted immediately on change so relaunch resumes on last
/// session's theme instead of the hardcoded default.
pub fn get_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .ok()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2) \
     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn migrate_household_members(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
    CREATE TABLE IF NOT EXISTS household_members (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL REFERENCES households(id),
      name TEXT NOT NULL,
      is_self INTEGER NOT NULL DEFAULT 0,
      UNIQUE(household_id, name)
    );
    ",
    )?;
    let _ = conn.execute("ALTER TABLE expenses ADD COLUMN member TEXT", []);

    let mut stmt = conn.prepare("SELECT id FROM households ORDER BY id")?;
    let household_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for household_id in household_ids {
        let member_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM household_members WHERE household_id = ?1",
            params![household_id],
            |row| row.get(0),
        )?;
        if member_count == 0 {
            conn.execute(
        "INSERT INTO household_members (household_id, name, is_self, color_rgb) VALUES (?1, 'Me', 1, ?2)",
        params![household_id, color_to_rgb(spectrum_color(0))],
      )?;
        }
    }
    Ok(())
}

pub fn migrate_member_colors(conn: &Connection) -> rusqlite::Result<()> {
    let _ = conn.execute(
        "ALTER TABLE household_members ADD COLUMN color_rgb INTEGER",
        [],
    );
    let mut stmt =
        conn.prepare("SELECT id, COALESCE(color_rgb, 0) FROM household_members ORDER BY id")?;
    let rows: Vec<(i64, i32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (index, (member_id, rgb)) in rows.iter().enumerate() {
        if *rgb == 0 {
            conn.execute(
                "UPDATE household_members SET color_rgb = ?1 WHERE id = ?2",
                params![color_to_rgb(spectrum_color(index)), member_id],
            )?;
        }
    }
    Ok(())
}

pub fn load_household_members(
    conn: &Connection,
    household_id: i64,
) -> rusqlite::Result<Vec<HouseholdMember>> {
    let mut stmt = conn.prepare(
    "SELECT id, name, is_self, COALESCE(color_rgb, 0) FROM household_members WHERE household_id = ?1 ORDER BY is_self DESC, name",
  )?;
    let rows = stmt
        .query_map(params![household_id], |row| {
            Ok(HouseholdMember {
                id: row.get(0)?,
                name: row.get(1)?,
                is_self: row.get::<_, i64>(2)? != 0,
                color: rgb_to_color(row.get(3)?),
            })
        })?
        .collect();
    rows
}

pub fn load_default_member_name(conn: &Connection, household_id: i64) -> rusqlite::Result<String> {
    conn.query_row(
        "SELECT name FROM household_members WHERE household_id = ?1 AND is_self = 1 LIMIT 1",
        params![household_id],
        |row| row.get(0),
    )
}

pub fn update_self_member_name(
    conn: &Connection,
    household_id: i64,
    name: &str,
) -> rusqlite::Result<()> {
    let old_name: String = conn.query_row(
        "SELECT name FROM household_members WHERE household_id = ?1 AND is_self = 1 LIMIT 1",
        params![household_id],
        |row| row.get(0),
    )?;
    conn.execute(
        "UPDATE household_members SET name = ?1 WHERE household_id = ?2 AND is_self = 1",
        params![name, household_id],
    )?;
    conn.execute(
        "UPDATE expenses SET member = ?1 WHERE household_id = ?2 AND member = ?3",
        params![name, household_id, old_name],
    )?;
    Ok(())
}

pub fn update_household_name(
    conn: &Connection,
    household_id: i64,
    name: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE households SET name = ?1 WHERE id = ?2",
        params![name, household_id],
    )?;
    Ok(())
}

pub fn add_household_member(
    conn: &Connection,
    household_id: i64,
    name: &str,
) -> rusqlite::Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(());
    }
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM household_members WHERE household_id = ?1 AND lower(name) = lower(?2)",
            params![household_id, name],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if exists {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE),
            Some(format!("member '{name}' already exists")),
        ));
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM household_members WHERE household_id = ?1",
        params![household_id],
        |row| row.get(0),
    )?;
    let color = spectrum_color(count as usize);
    conn.execute(
    "INSERT INTO household_members (household_id, name, is_self, color_rgb) VALUES (?1, ?2, 0, ?3)",
    params![household_id, name, color_to_rgb(color)],
  )?;
    Ok(())
}

pub fn update_member_color_db(
    conn: &Connection,
    household_id: i64,
    member_id: i64,
    color: Color32,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE household_members SET color_rgb = ?1 WHERE id = ?2 AND household_id = ?3",
        params![color_to_rgb(color), member_id, household_id],
    )?;
    Ok(())
}

pub fn delete_household_member(
    conn: &Connection,
    household_id: i64,
    member_id: i64,
) -> rusqlite::Result<()> {
    let name: String = conn.query_row(
        "SELECT name FROM household_members WHERE id = ?1 AND household_id = ?2",
        params![member_id, household_id],
        |row| row.get(0),
    )?;
    conn.execute(
        "UPDATE expenses SET member = '' WHERE household_id = ?1 AND member = ?2",
        params![household_id, name],
    )?;
    conn.execute(
        "DELETE FROM household_members WHERE id = ?1 AND household_id = ?2",
        params![member_id, household_id],
    )?;
    Ok(())
}

pub fn load_accounts(conn: &Connection, household_id: i64) -> rusqlite::Result<Vec<Account>> {
    let mut stmt = conn.prepare(
    "SELECT id, name, kind, balance_cents, csv_name FROM accounts WHERE household_id = ?1 ORDER BY id",
  )?;
    let rows = stmt
        .query_map(params![household_id], |row| {
            Ok(Account {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                balance_cents: row.get(3)?,
                csv_name: row.get(4)?,
            })
        })?
        .collect();
    rows
}

/// accounts are born from imports — the CSV's detected account
/// value maps to a user-named account via csv_name, so the next import
/// detecting the same value auto-selects the same account. Renames propagate.
pub fn resolve_or_create_account(
    conn: &Connection,
    household_id: i64,
    detected: &str,
    name: &str,
) -> rusqlite::Result<i64> {
    let name = name.trim();
    let name = if name.is_empty() { "Checking" } else { name };
    let detected = detected.trim();
    if !detected.is_empty() {
        if let Ok(id) = conn.query_row(
            "SELECT id FROM accounts WHERE household_id = ?1 AND csv_name = ?2 COLLATE NOCASE",
            params![household_id, detected],
            |row| row.get::<_, i64>(0),
        ) {
            conn.execute(
                "UPDATE accounts SET name = ?2 WHERE id = ?1",
                params![id, name],
            )?;
            return Ok(id);
        }
    }
    if let Ok(id) = conn.query_row(
        "SELECT id FROM accounts WHERE household_id = ?1 AND name = ?2 COLLATE NOCASE",
        params![household_id, name],
        |row| row.get::<_, i64>(0),
    ) {
        // Only record a mapping for a real detected value — never an empty one.
        if !detected.is_empty() {
            conn.execute(
                "UPDATE accounts SET csv_name = ?2 WHERE id = ?1",
                params![id, detected],
            )?;
        }
        return Ok(id);
    }
    conn.execute(
    "INSERT INTO accounts (household_id, name, kind, balance_cents, csv_name) VALUES (?1, ?2, 'checking', 0, ?3)",
    params![household_id, name, detected],
  )?;
    Ok(conn.last_insert_rowid())
}

pub fn load_expenses(conn: &Connection, household_id: i64) -> rusqlite::Result<Vec<Expense>> {
    let mut stmt = conn.prepare(
    "SELECT e.id, e.date, e.amount_cents, COALESCE(e.member, ''), e.category, COALESCE(e.vendor, ''), e.description, COALESCE(e.account_id, 1), COALESCE(a.name, '')
     FROM expenses e
     LEFT JOIN accounts a ON a.id = e.account_id
     WHERE e.household_id = ?1
     ORDER BY e.date DESC, e.id DESC",
  )?;
    let rows = stmt
        .query_map(params![household_id], |row| {
            let amount_cents = row.get(2)?;
            Ok(Expense {
                id: row.get(0)?,
                date: row.get(1)?,
                amount_input: money(amount_cents).replace('$', ""),
                amount_cents,
                member: row.get(3)?,
                category: row.get(4)?,
                vendor: row.get(5)?,
                description: row.get(6)?,
                account_id: row.get(7)?,
                account: row.get(8)?,
            })
        })?
        .collect();
    rows
}

pub fn load_categories(conn: &Connection, household_id: i64) -> rusqlite::Result<Vec<Category>> {
    let mut stmt = conn.prepare(
    "SELECT id, name, parent_id, COALESCE(color_rgb, 0), excluded FROM categories WHERE household_id = ?1 ORDER BY COALESCE(parent_id, id), name",
  )?;
    let rows = stmt
        .query_map(params![household_id], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                color: rgb_to_color(row.get(3)?),
                excluded: row.get::<_, i64>(4)? != 0,
            })
        })?
        .collect();
    rows
}

pub fn migrate_category_hierarchy(conn: &Connection) -> rusqlite::Result<()> {
    let _ = conn.execute(
    "ALTER TABLE categories ADD COLUMN parent_id INTEGER REFERENCES categories(id) ON DELETE CASCADE",
    [],
  );
    Ok(())
}

pub fn ensure_migrations_table(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
    CREATE TABLE IF NOT EXISTS app_migrations (
      name TEXT PRIMARY KEY,
      applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    ",
    )?;
    Ok(())
}

pub fn migration_applied(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM app_migrations WHERE name = ?1",
            params![name],
            |_| Ok(true),
        )
        .unwrap_or(false))
}

pub fn mark_migration_applied(conn: &Connection, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO app_migrations (name) VALUES (?1)",
        params![name],
    )?;
    Ok(())
}

pub const CATEGORY_DEFAULTS_V2_MIGRATION: &str = "category_defaults_v2";
pub const CATEGORY_PALETTE_COLORS_V3_MIGRATION: &str = "category_palette_colors_v3";
pub const THEME_ACCENT_MEMBERS_V4_MIGRATION: &str = "theme_accent_members_v4";
pub const THEME_ACCENT_CATEGORIES_V4_MIGRATION: &str = "theme_accent_categories_v4";
pub const BUDGETS_V5_MIGRATION: &str = "budgets_v5";
pub const BUDGET_SNAPSHOTS_V6_MIGRATION: &str = "budget_snapshots_v6";
pub const ANALYTICS_FILTERS_V7_MIGRATION: &str = "analytics_filters_v7";
pub const CATEGORY_PALETTE_VIBRANT_V8_MIGRATION: &str = "category_palette_vibrant_v8";

pub fn migrate_category_defaults_v2(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, CATEGORY_DEFAULTS_V2_MIGRATION)? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT id FROM households ORDER BY id")?;
    let household_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for household_id in household_ids {
        reset_household_categories_to_defaults(conn, household_id)?;
    }
    mark_migration_applied(conn, CATEGORY_DEFAULTS_V2_MIGRATION)?;
    Ok(())
}

pub fn migrate_category_palette_colors_v3(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, CATEGORY_PALETTE_COLORS_V3_MIGRATION)? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT id FROM households ORDER BY id")?;
    let household_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for household_id in household_ids {
        recolor_household_categories(conn, household_id)?;
    }
    mark_migration_applied(conn, CATEGORY_PALETTE_COLORS_V3_MIGRATION)?;
    Ok(())
}

pub fn migrate_theme_accent_members_v4(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, THEME_ACCENT_MEMBERS_V4_MIGRATION)? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT id FROM household_members ORDER BY id")?;
    let member_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for (index, member_id) in member_ids.iter().enumerate() {
        conn.execute(
            "UPDATE household_members SET color_rgb = ?1 WHERE id = ?2",
            params![color_to_rgb(accent_color_from_index(index)), member_id],
        )?;
    }
    mark_migration_applied(conn, THEME_ACCENT_MEMBERS_V4_MIGRATION)?;
    Ok(())
}

pub fn migrate_theme_accent_categories_v4(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, THEME_ACCENT_CATEGORIES_V4_MIGRATION)? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT id FROM households ORDER BY id")?;
    let household_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for household_id in household_ids {
        recolor_household_categories(conn, household_id)?;
    }
    mark_migration_applied(conn, THEME_ACCENT_CATEGORIES_V4_MIGRATION)?;
    Ok(())
}

pub fn migrate_budgets_v5(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, BUDGETS_V5_MIGRATION)? {
        return Ok(());
    }
    conn.execute(
        "CREATE TABLE IF NOT EXISTS budgets (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL DEFAULT 1 REFERENCES households(id),
      category TEXT NOT NULL,
      amount_cents INTEGER NOT NULL,
      year INTEGER NOT NULL,
      month INTEGER NOT NULL,
      UNIQUE(household_id, category, year, month)
    );",
        [],
    )?;
    mark_migration_applied(conn, BUDGETS_V5_MIGRATION)?;
    Ok(())
}

pub fn recolor_household_categories(conn: &Connection, household_id: i64) -> rusqlite::Result<()> {
    let categories = load_categories(conn, household_id)?;
    let mut parent_ids: Vec<i64> = categories
        .iter()
        .filter(|category| category.parent_id.is_none())
        .map(|category| category.id)
        .collect();
    parent_ids.sort_by(|a, b| {
        let name_a = categories
            .iter()
            .find(|category| category.id == *a)
            .map(|category| category.name.as_str())
            .unwrap_or("");
        let name_b = categories
            .iter()
            .find(|category| category.id == *b)
            .map(|category| category.name.as_str())
            .unwrap_or("");
        name_a.to_lowercase().cmp(&name_b.to_lowercase())
    });

    for (parent_index, parent_id) in parent_ids.iter().enumerate() {
        let parent_color = category_parent_palette_color(parent_index);
        update_category_color_db(conn, household_id, *parent_id, parent_color)?;

        let mut sub_ids: Vec<i64> = categories
            .iter()
            .filter(|category| category.parent_id == Some(*parent_id))
            .map(|category| category.id)
            .collect();
        sub_ids.sort_by(|a, b| {
            let name_a = categories
                .iter()
                .find(|category| category.id == *a)
                .map(|category| category.name.as_str())
                .unwrap_or("");
            let name_b = categories
                .iter()
                .find(|category| category.id == *b)
                .map(|category| category.name.as_str())
                .unwrap_or("");
            name_a.to_lowercase().cmp(&name_b.to_lowercase())
        });
        let sub_count = sub_ids.len().max(1);
        for (sub_index, sub_id) in sub_ids.iter().enumerate() {
            let sub_color = subcategory_color_from_parent(parent_color, sub_index, sub_count);
            update_category_color_db(conn, household_id, *sub_id, sub_color)?;
        }
    }
    Ok(())
}

pub fn reset_household_categories_to_defaults(
    conn: &Connection,
    household_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM categories WHERE household_id = ?1",
        params![household_id],
    )?;
    conn.execute(
        "UPDATE expenses SET category = '' WHERE household_id = ?1",
        params![household_id],
    )?;
    conn.execute(
        "UPDATE vendor_category_rules SET category = '' WHERE household_id = ?1",
        params![household_id],
    )?;
    insert_default_category_tree(conn, household_id)
}

pub fn migrate_category_colors(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt =
        conn.prepare("SELECT rowid, COALESCE(color_rgb, 0) FROM categories ORDER BY rowid")?;
    let rows: Vec<(i64, i32)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (index, (rowid, rgb)) in rows.iter().enumerate() {
        if *rgb == 0 {
            let color = category_parent_palette_color(index);
            conn.execute(
                "UPDATE categories SET color_rgb = ?1 WHERE rowid = ?2",
                params![color_to_rgb(color), rowid],
            )?;
        }
    }
    Ok(())
}

pub const ACCENT_PALETTE_COUNT: usize = 12;

pub fn accent_palette_swatches() -> Vec<Color32> {
    accent_palette_swatches_from(crate::ui::theme::accent())
}

pub fn accent_palette_swatches_from(base: Color32) -> Vec<Color32> {
    let base_hsva = Hsva::from_srgba_unmultiplied([base.r(), base.g(), base.b(), 255]);
    (0..ACCENT_PALETTE_COUNT)
        .map(|i| {
            let mut hsva = base_hsva;
            hsva.h = (base_hsva.h + i as f32 / ACCENT_PALETTE_COUNT as f32) % 1.0;
            // Vibrant swatch palette: saturation 0.65..0.85, value 0.78..0.92.
            // Milder clamps produce pastel, washed-out swatches.
            hsva.s = (0.65 + (i % 3) as f32 * 0.07).clamp(0.65, 0.85);
            hsva.v = (0.80 + ((i / 3) % 3) as f32 * 0.05).clamp(0.80, 0.92);
            Color32::from(hsva)
        })
        .collect()
}

pub fn accent_color_from_index(index: usize) -> Color32 {
    let swatches = accent_palette_swatches();
    swatches[index % swatches.len()]
}

pub fn spectrum_color(index: usize) -> Color32 {
    accent_color_from_index(index)
}

pub fn category_parent_palette_color(parent_index: usize) -> Color32 {
    accent_color_from_index(parent_index)
}

pub fn subcategory_color_from_parent(
    parent: Color32,
    sub_index: usize,
    sub_count: usize,
) -> Color32 {
    let hsva = Hsva::from_srgba_unmultiplied([parent.r(), parent.g(), parent.b(), 255]);
    let n = sub_count.max(1) as f32;
    let idx = sub_index as f32;
    let mut sub = hsva;
    sub.h = (sub.h + (idx / n) * 0.06 + idx * 0.01) % 1.0;
    // Vibrant subcategory clamp: s 0.60..0.92, v 0.72..0.95, so children
    // stay recognizably vivid against the parent palette.
    sub.s = (sub.s * (0.95 + idx * 0.04 / n)).clamp(0.60, 0.92);
    sub.v = (sub.v * (1.00 - idx * 0.02 / n)).clamp(0.72, 0.95);
    Color32::from(sub)
}

pub fn colors_match(a: Color32, b: Color32) -> bool {
    a.r() == b.r() && a.g() == b.g() && a.b() == b.b()
}

pub fn color_to_rgb(color: Color32) -> i32 {
    ((color.r() as i32) << 16) | ((color.g() as i32) << 8) | color.b() as i32
}

pub fn rgb_to_color(rgb: i32) -> Color32 {
    if rgb == 0 {
        return Color32::TRANSPARENT;
    }
    Color32::from_rgb(
        ((rgb >> 16) & 0xFF) as u8,
        ((rgb >> 8) & 0xFF) as u8,
        (rgb & 0xFF) as u8,
    )
}

pub fn category_labels_for_subtree(categories: &[Category], root_id: i64) -> Vec<String> {
    let parents = category_parent_map(categories);
    let mut labels = Vec::new();
    if let Some(root) = categories.iter().find(|category| category.id == root_id) {
        let mut has_children = false;
        for category in categories {
            if category.parent_id == Some(root_id) {
                has_children = true;
                labels.push(category.full_label(&parents));
            }
        }
        if !has_children {
            labels.push(root.name.clone());
        }
    }
    labels
}

pub fn add_parent_category_db(
    conn: &Connection,
    household_id: i64,
    name: &str,
) -> rusqlite::Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(());
    }
    let exists: bool = conn
    .query_row(
      "SELECT 1 FROM categories WHERE household_id = ?1 AND parent_id IS NULL AND lower(name) = lower(?2)",
      params![household_id, name],
      |_| Ok(true),
    )
    .unwrap_or(false);
    if exists {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE),
            Some(format!("parent category '{name}' already exists")),
        ));
    }
    let parent_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE household_id = ?1 AND parent_id IS NULL",
        params![household_id],
        |row| row.get(0),
    )?;
    let color = category_parent_palette_color(parent_count as usize);
    conn.execute(
    "INSERT INTO categories (household_id, name, parent_id, color_rgb) VALUES (?1, ?2, NULL, ?3)",
    params![household_id, name, color_to_rgb(color)],
  )?;
    Ok(())
}

pub fn add_subcategory_db(
    conn: &Connection,
    household_id: i64,
    parent_id: i64,
    name: &str,
) -> rusqlite::Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(());
    }
    let parent_exists: bool = conn
        .query_row(
            "SELECT 1 FROM categories WHERE id = ?1 AND household_id = ?2 AND parent_id IS NULL",
            params![parent_id, household_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !parent_exists {
        return Err(rusqlite::Error::InvalidParameterName(
            "parent category not found".into(),
        ));
    }
    let exists: bool = conn
    .query_row(
      "SELECT 1 FROM categories WHERE household_id = ?1 AND parent_id = ?2 AND lower(name) = lower(?3)",
      params![household_id, parent_id, name],
      |_| Ok(true),
    )
    .unwrap_or(false);
    if exists {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE),
            Some(format!(
                "subcategory '{name}' already exists under that parent"
            )),
        ));
    }
    let parent_rgb: i32 = conn.query_row(
        "SELECT COALESCE(color_rgb, 0) FROM categories WHERE id = ?1 AND household_id = ?2",
        params![parent_id, household_id],
        |row| row.get(0),
    )?;
    let parent_color = rgb_to_color(parent_rgb);
    let sub_index: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE household_id = ?1 AND parent_id = ?2",
        params![household_id, parent_id],
        |row| row.get(0),
    )?;
    let sub_count = (sub_index + 1) as usize;
    let color = subcategory_color_from_parent(parent_color, sub_index as usize, sub_count);
    conn.execute(
        "INSERT INTO categories (household_id, name, parent_id, color_rgb) VALUES (?1, ?2, ?3, ?4)",
        params![household_id, name, parent_id, color_to_rgb(color)],
    )?;
    Ok(())
}

pub fn update_category_color_db(
    conn: &Connection,
    household_id: i64,
    category_id: i64,
    color: Color32,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE categories SET color_rgb = ?1 WHERE id = ?2 AND household_id = ?3",
        params![color_to_rgb(color), category_id, household_id],
    )?;
    Ok(())
}

pub fn delete_category_from_db(
    conn: &Connection,
    household_id: i64,
    category_id: i64,
) -> rusqlite::Result<()> {
    let categories = load_categories(conn, household_id)?;
    for label in category_labels_for_subtree(&categories, category_id) {
        conn.execute(
            "UPDATE expenses SET category = '' WHERE household_id = ?1 AND category = ?2",
            params![household_id, label],
        )?;
        conn.execute(
      "UPDATE vendor_category_rules SET category = '' WHERE household_id = ?1 AND category = ?2",
      params![household_id, label],
    )?;
    }
    conn.execute(
        "DELETE FROM categories WHERE household_id = ?1 AND (id = ?2 OR parent_id = ?2)",
        params![household_id, category_id],
    )?;
    Ok(())
}

pub fn seed_default_categories(conn: &Connection, household_id: i64) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE household_id = ?1",
        params![household_id],
        |row| row.get(0),
    )?;
    if count > 0 {
        return Ok(());
    }
    insert_default_category_tree(conn, household_id)
}

pub fn insert_default_category_tree(conn: &Connection, household_id: i64) -> rusqlite::Result<()> {
    let defaults: &[(&str, &[&str])] = &[
        (
            "Housing",
            &[
                "Mortgage / Rent",
                "Property Tax",
                "HOA Fees",
                "Home Insurance",
                "Repairs & Maintenance",
                "Renovations",
            ],
        ),
        (
            "Utilities",
            &[
                "Electricity",
                "Gas / Heating Oil",
                "Water & Sewer",
                "Trash & Recycling",
                "Internet",
                "Cell Phone",
            ],
        ),
        (
            "Food",
            &[
                "Groceries",
                "Dining Out",
                "Coffee Shops",
                "Alcohol",
                "Food Delivery",
            ],
        ),
        (
            "Transportation",
            &[
                "Car Payment",
                "Car Insurance",
                "Fuel",
                "EV Charging",
                "Parking & Tolls",
                "Public Transit",
                "Rideshare / Taxi",
                "Vehicle Maintenance",
                "Registration & Licensing",
            ],
        ),
        (
            "Healthcare",
            &[
                "Health Insurance",
                "Dental Insurance",
                "Vision Insurance",
                "Prescriptions",
                "Doctor / Specialist Visits",
                "Dental Care",
                "Vision Care",
                "Mental Health & Therapy",
                "Fitness & Gym",
            ],
        ),
        (
            "Personal Care",
            &[
                "Haircuts & Styling",
                "Skincare & Beauty",
                "Clothing & Shoes",
                "Laundry & Dry Cleaning",
            ],
        ),
        (
            "Children",
            &[
                "Childcare / Daycare",
                "School Tuition & Fees",
                "School Supplies",
                "Extracurriculars",
                "Toys & Books",
            ],
        ),
        (
            "Pets",
            &[
                "Food & Treats",
                "Vet Care",
                "Grooming",
                "Pet Insurance",
                "Supplies",
            ],
        ),
        (
            "Entertainment & Leisure",
            &[
                "Streaming Services",
                "Movies & Events",
                "Hobbies",
                "Sports & Recreation",
                "Vacations & Travel",
                "Gaming",
            ],
        ),
        (
            "Education",
            &[
                "Tuition",
                "Student Loan Payments",
                "Courses & Certifications",
                "Supplies & Books",
            ],
        ),
        (
            "Financial",
            &[
                "Emergency Fund",
                "Retirement Contributions",
                "Investments",
                "Crypto / Digital Assets",
                "Life Insurance",
                "Debt Repayment (Credit Cards)",
                "BNPL Payments",
                "Bank Fees",
            ],
        ),
        (
            "Subscriptions & Memberships",
            &[
                "AI Tools & Subscriptions",
                "Software & Apps",
                "Cloud Storage",
                "Clubs & Memberships",
            ],
        ),
        (
            "Gifts & Donations",
            &[
                "Gifts (Birthdays, Holidays)",
                "Charitable Donations",
                "Religious Contributions",
            ],
        ),
        (
            "Household Supplies",
            &[
                "Cleaning Products",
                "Paper Products",
                "Kitchen Supplies",
                "Furniture & Decor",
            ],
        ),
        (
            "Taxes",
            &[
                "Federal Income Tax",
                "Provincial / State Tax",
                "Self-Employment Tax",
            ],
        ),
        (
            "Business / Side Income Expenses",
            &[
                "Home Office",
                "Content Creation Equipment",
                "Gig Work Expenses",
                "Professional Services",
                "Marketing",
            ],
        ),
        (
            "Miscellaneous",
            &[
                "ATM / Cash",
                "Postage & Shipping",
                "Legal Fees",
                "Uncategorized",
            ],
        ),
    ];

    for (parent_name, subs) in defaults {
        add_parent_category_db(conn, household_id, parent_name)?;
        let parent_id: i64 = conn.query_row(
            "SELECT id FROM categories WHERE household_id = ?1 AND parent_id IS NULL AND name = ?2",
            params![household_id, parent_name],
            |row| row.get(0),
        )?;
        for sub_name in *subs {
            add_subcategory_db(conn, household_id, parent_id, sub_name)?;
        }
    }

    // protected parents — seeding removed; exclusion is now a
    // user-set flag on any category (see migrate_category_exclusion_v12).
    Ok(())
}

pub fn update_expense_row(conn: &Connection, row: &Expense, field: &str) -> rusqlite::Result<()> {
    match field {
        "date" => {
            conn.execute(
                "UPDATE expenses SET date = ?1 WHERE id = ?2",
                params![row.date, row.id],
            )?;
        }
        "amount" => {
            conn.execute(
                "UPDATE expenses SET amount_cents = ?1 WHERE id = ?2",
                params![row.amount_cents, row.id],
            )?;
        }
        "account" => {
            conn.execute(
                "UPDATE expenses SET account_id = ?1 WHERE id = ?2",
                params![row.account_id, row.id],
            )?;
        }
        "category" => {
            conn.execute(
                "UPDATE expenses SET category = ?1 WHERE id = ?2",
                params![row.category, row.id],
            )?;
        }
        "member" => {
            conn.execute(
                "UPDATE expenses SET member = ?1 WHERE id = ?2",
                params![row.member, row.id],
            )?;
        }
        "vendor" => {
            conn.execute(
                "UPDATE expenses SET vendor = ?1 WHERE id = ?2",
                params![row.vendor, row.id],
            )?;
        }
        "description" => {
            conn.execute(
                "UPDATE expenses SET description = ?1 WHERE id = ?2",
                params![row.description, row.id],
            )?;
        }
        _ => {}
    }
    Ok(())
}

pub fn save_import_rows(
    conn: &Connection,
    household_id: i64,
    rows: &[ImportRow],
    account_id: i64,
) -> rusqlite::Result<usize> {
    let categories = load_categories(conn, household_id)?;
    let parents = category_parent_map(&categories);
    for row in rows {
        let category = if row.category.trim().is_empty() {
            String::new()
        } else if let Some(matched) = find_category_by_label(&categories, &row.category) {
            matched.full_label(&parents)
        } else {
            String::new()
        };
        // sign is a function of category — debits negative, Income
        // positive. Excluded rows keep their real amount; the flag only filters
        // them out of aggregation math.
        let sign = category_sign(&categories, &category);
        let amount_cents = if sign == 1 {
            row.amount_cents.abs()
        } else {
            -row.amount_cents.abs()
        };
        conn.execute(
      "INSERT INTO expenses (household_id, account_id, description, vendor, category, member, amount_cents, date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        household_id,
        account_id,
        row.description,
        row.vendor,
        category,
        row.member,
        amount_cents,
        row.date
      ],
    )?;
        // upsert instead of INSERT OR IGNORE — a rule was learned once
        // and never updated, so corrections on later imports never stuck. Skip
        // short vendors: learning from vendor "e" poisoned every future match.
        if !category.is_empty()
            && !category.contains("Uncategorized")
            && row.vendor.trim().chars().count() >= 3
        {
            conn.execute(
        "INSERT INTO vendor_category_rules (household_id, vendor_pattern, category) VALUES (?1, ?2, ?3)
         ON CONFLICT(household_id, vendor_pattern) DO UPDATE SET category = excluded.category",
        params![household_id, row.vendor.to_lowercase(), category],
      )?;
        }
    }
    Ok(rows.len())
}

pub fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.abs();
    format!("{sign}${}.{:02}", cents / 100, cents % 100)
}

pub fn format_date(raw: &str) -> Option<String> {
    let cleaned = raw.trim().replace(|c: char| !c.is_ascii_digit(), "-");
    let parts: Vec<&str> = cleaned.split('-').filter(|s| !s.is_empty()).collect();
    if parts.len() == 3 {
        let part1 = parts[0];
        let part2 = parts[1];
        let part3 = parts[2];

        if part1.len() == 4 {
            if let (Ok(m), Ok(d)) = (part2.parse::<u32>(), part3.parse::<u32>()) {
                return Some(format!("{}-{:02}-{:02}", part1, m, d));
            }
        } else if part3.len() == 4 {
            if let (Ok(m), Ok(d)) = (part1.parse::<u32>(), part2.parse::<u32>()) {
                if m > 12 && d <= 12 {
                    return Some(format!("{}-{:02}-{:02}", part3, d, m));
                }
                return Some(format!("{}-{:02}-{:02}", part3, m, d));
            }
        }
    }
    None
}

pub fn parse_amount_cents(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let negative = trimmed.starts_with('-') || (trimmed.starts_with('(') && trimmed.ends_with(')'));
    let cleaned = trimmed
        .replace(['$', ',', '(', ')', ' '], "")
        .trim_start_matches('-')
        .to_string();
    let amount = cleaned.parse::<f64>().ok()?;
    let cents = (amount * 100.0).round() as i64;
    Some(if negative { -cents } else { cents }.abs())
}

pub fn normalize_date(raw: &str) -> String {
    let trimmed = raw.trim();
    let parts: Vec<&str> = trimmed.split(['/', '-']).collect();
    if parts.len() == 3 && parts[0].len() == 4 {
        return format!("{:0>4}-{:0>2}-{:0>2}", parts[0], parts[1], parts[2]);
    }
    if parts.len() == 3 {
        return format!("{:0>4}-{:0>2}-{:0>2}", parts[2], parts[0], parts[1]);
    }
    trimmed.to_string()
}

pub fn normalize_vendor(description: &str) -> String {
    // no '-' split — "E-TRANSFER 12345" collapsed to vendor "e",
    // which was learned as a rule and matched every vendor containing 'e'.
    description
        .split(['*', '#'])
        .next()
        .unwrap_or(description)
        .trim()
        .to_string()
}

pub fn normalize_header(header: &str) -> String {
    header.trim().to_lowercase().replace([' ', '_', '-'], "")
}

pub fn find_column(headers: &[String], candidates: &[&str]) -> Option<usize> {
    headers.iter().position(|header| {
        let normalized = normalize_header(header);
        candidates
            .iter()
            .any(|candidate| normalized.contains(candidate))
    })
}

pub fn category_for_vendor(conn: &Connection, household_id: i64, vendor: &str) -> String {
    // recency-first — the user's most recent correction wins. The old
    // order matched rules oldest-first and short-circuited before history was
    // ever consulted, so re-categorizations never stuck.
    // 1) Most recent non-empty category for this exact vendor.
    if let Ok(mut stmt) = conn.prepare(
        "SELECT category
     FROM expenses
     WHERE household_id = ?1
       AND lower(vendor) = lower(?2)
       AND category IS NOT NULL
       AND category != ''
       AND category != 'Uncategorized'
     ORDER BY date DESC, id DESC
     LIMIT 1",
    ) {
        if let Ok(history_category) =
            stmt.query_row(params![household_id, vendor], |row| row.get::<_, String>(0))
        {
            return history_category;
        }
    }

    // 2) Vendor rules, newest first (skipping empty-category zombie rules left
    // behind by deleted categories).
    if let Ok(mut stmt) = conn.prepare(
        "SELECT vendor_pattern, category FROM vendor_category_rules
     WHERE household_id = ?1 AND category IS NOT NULL AND category != ''
     ORDER BY id DESC",
    ) {
        let normalized = vendor.to_lowercase();
        let rules = stmt
            .query_map(params![household_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
            .unwrap_or_default();
        if let Some(category) = rules
            .into_iter()
            // short patterns like the learned 'e' rule substring-match
            // nearly every vendor — never match on anything under 3 chars.
            .filter(|(pattern, _)| pattern.trim().chars().count() >= 3)
            .find_map(|(pattern, category)| {
                normalized
                    .contains(&pattern.to_lowercase())
                    .then_some(category)
            })
        {
            return category;
        }
    }

    "Uncategorized".to_string()
}

pub fn parse_csv_statement(
    conn: &Connection,
    household_id: i64,
    path: &std::path::Path,
) -> Result<Vec<ImportRow>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(&path)
        .map_err(|err| format!("Could not open CSV: {err}"))?;
    let headers = reader
        .headers()
        .map_err(|err| format!("Could not read CSV headers: {err}"))?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let date_idx = find_column(&headers, &["date", "posted"]);
    let amount_idx = find_column(
        &headers,
        &["amount", "debit", "withdrawal", "charge", "credit"],
    );
    let description_idx = find_column(
        &headers,
        &[
            "description",
            "memo",
            "details",
            "transaction",
            "payee",
            "merchant",
        ],
    );
    let category_idx = find_column(&headers, &["category"]);
    let account_idx = find_column(&headers, &["account"]);

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|err| format!("Bad CSV row: {err}"))?;
        let date = date_idx
            .and_then(|idx| record.get(idx))
            .map(normalize_date)
            .unwrap_or_else(|| "2026-01-01".to_string());
        let description = description_idx
            .and_then(|idx| record.get(idx))
            .unwrap_or("")
            .trim()
            .to_string();
        let amount_cents = amount_idx
            .and_then(|idx| record.get(idx))
            .and_then(parse_amount_cents)
            .unwrap_or(0);
        if description.is_empty() || amount_cents == 0 {
            continue;
        }
        let vendor = normalize_vendor(&description);
        let category = category_idx
            .and_then(|idx| record.get(idx))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| category_for_vendor(conn, household_id, &vendor));
        let account = account_idx
            .and_then(|idx| record.get(idx))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_default();
        rows.push(ImportRow {
            date,
            amount_input: money(amount_cents).replace('$', ""),
            amount_cents,
            member: load_default_member_name(conn, household_id).unwrap_or_default(),
            category,
            vendor,
            description,
            account,
        });
    }
    Ok(rows)
}

pub fn import_row_status(row: &ImportRow) -> &'static str {
    if row.date.trim().is_empty() || row.amount_cents == 0 || row.description.trim().is_empty() {
        "needs edit"
    } else if row.category.trim().is_empty() || row.category == "Uncategorized" {
        "needs category"
    } else {
        "ready"
    }
}

pub fn duplicate_import_count(conn: &Connection, household_id: i64, rows: &[ImportRow]) -> usize {
    rows
    .iter()
    .filter(|row| {
      conn
        .query_row(
          "SELECT COUNT(*) FROM expenses WHERE household_id = ?1 AND date = ?2 AND abs(amount_cents) = ?3 AND lower(COALESCE(vendor, '')) = lower(?4) AND lower(description) = lower(?5)",
          params![household_id, row.date, row.amount_cents, row.vendor, row.description],
          |count_row| count_row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0
    })
    .count()
}

pub fn load_budgets(
    conn: &Connection,
    household_id: i64,
    year: i32,
    month: i32,
) -> rusqlite::Result<Vec<Budget>> {
    let mut stmt = conn.prepare(
    "SELECT id, category, amount_cents, year, month FROM budgets WHERE household_id = ?1 AND year = ?2 AND month = ?3",
  )?;
    let rows = stmt
        .query_map(params![household_id, year, month], |row| {
            Ok(Budget {
                id: row.get(0)?,
                category: row.get(1)?,
                amount_cents: row.get(2)?,
                year: row.get(3)?,
                month: row.get(4)?,
            })
        })?
        .collect();
    rows
}

pub fn save_budget(
    conn: &Connection,
    household_id: i64,
    category: &str,
    amount_cents: i64,
    year: i32,
    month: i32,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO budgets (household_id, category, amount_cents, year, month)
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT(household_id, category, year, month)
     DO UPDATE SET amount_cents = excluded.amount_cents",
        params![household_id, category.trim(), amount_cents, year, month],
    )?;
    Ok(())
}

// ── Budget Snapshots v6 ──────────────────────────────────────────────────

pub fn migrate_budget_snapshots_v6(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, BUDGET_SNAPSHOTS_V6_MIGRATION)? {
        return Ok(());
    }
    conn.execute(
        "CREATE TABLE IF NOT EXISTS budget_snapshots (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL DEFAULT 1,
      category TEXT NOT NULL,
      year INTEGER NOT NULL,
      period_code INTEGER NOT NULL,
      amount_cents INTEGER NOT NULL DEFAULT 0,
      is_override INTEGER NOT NULL DEFAULT 0,
      UNIQUE(household_id, category, year, period_code)
    );",
        [],
    )?;
    mark_migration_applied(conn, BUDGET_SNAPSHOTS_V6_MIGRATION)?;
    Ok(())
}

pub fn save_budget_snapshot(
    conn: &Connection,
    household_id: i64,
    category: &str,
    year: i32,
    period_code: i32,
    amount_cents: i64,
    is_override: bool,
) -> rusqlite::Result<()> {
    conn.execute(
    "INSERT INTO budget_snapshots (household_id, category, year, period_code, amount_cents, is_override)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
     ON CONFLICT(household_id, category, year, period_code)
     DO UPDATE SET amount_cents = excluded.amount_cents, is_override = excluded.is_override",
    params![household_id, category.trim(), year, period_code, amount_cents, is_override as i32],
  )?;
    Ok(())
}

pub fn load_budget_snapshots_for_year(
    conn: &Connection,
    household_id: i64,
    year: i32,
) -> rusqlite::Result<Vec<BudgetSnapshot>> {
    let mut stmt = conn.prepare(
        "SELECT id, category, year, period_code, amount_cents, is_override
     FROM budget_snapshots WHERE household_id = ?1 AND year = ?2",
    )?;
    let rows = stmt
        .query_map(params![household_id, year], |row| {
            Ok(BudgetSnapshot {
                id: row.get(0)?,
                category: row.get(1)?,
                year: row.get(2)?,
                period_code: row.get(3)?,
                amount_cents: row.get(4)?,
                is_override: row.get::<_, i32>(5)? != 0,
            })
        })?
        .collect();
    rows
}

// ── Analytics Filters v7 ─────────────────────────────────────────────────

pub fn migrate_category_palette_vibrant_v8(conn: &Connection) -> rusqlite::Result<()> {
    // re-derive every existing category's color with the new
    // vibrant palette. recolor_household_categories uses the live
    // accent_palette_swatches_from + subcategory_color_from_parent, so
    // calling it after the palette function changed re-derives colors
    // for every category in every household. Idempotent: the migration
    // is marked applied so the next launch skips it.
    ensure_migrations_table(conn)?;
    if migration_applied(conn, CATEGORY_PALETTE_VIBRANT_V8_MIGRATION)? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT id FROM households ORDER BY id")?;
    let household_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for household_id in household_ids {
        recolor_household_categories(conn, household_id)?;
    }
    mark_migration_applied(conn, CATEGORY_PALETTE_VIBRANT_V8_MIGRATION)?;
    Ok(())
}

pub fn migrate_analytics_filters_v7(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, ANALYTICS_FILTERS_V7_MIGRATION)? {
        return Ok(());
    }
    conn.execute(
        "CREATE TABLE IF NOT EXISTS analytics_filters (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      date_preset TEXT NOT NULL DEFAULT 'last_3_months',
      date_start TEXT,
      date_end TEXT,
      granularity TEXT NOT NULL DEFAULT 'monthly',
      active_chart TEXT NOT NULL DEFAULT 'category_breakdown',
      category_filter_mode TEXT NOT NULL DEFAULT 'include',
      selected_categories TEXT NOT NULL DEFAULT '[]',
      selected_members TEXT NOT NULL DEFAULT '[]',
      selected_vendors TEXT NOT NULL DEFAULT '[]',
      candlestick_year INTEGER NOT NULL DEFAULT 2026,
      candlestick_period TEXT NOT NULL DEFAULT 'monthly',
      comparison_mode TEXT NOT NULL DEFAULT 'overlay',
      comparison_date_preset TEXT NOT NULL DEFAULT 'last_month'
    );",
        [],
    )?;
    mark_migration_applied(conn, ANALYTICS_FILTERS_V7_MIGRATION)?;
    Ok(())
}

pub fn load_analytics_state(conn: &Connection) -> rusqlite::Result<Option<AnalyticsFilterRow>> {
    let mut stmt = conn.prepare(
        "SELECT date_preset, date_start, date_end, active_chart,
            category_filter_mode, selected_categories, selected_members,
            selected_vendors, comparison_mode, comparison_date_preset
     FROM analytics_filters WHERE id = 1",
    )?;
    let mut rows = stmt.query_map([], |row| {
        Ok(AnalyticsFilterRow {
            date_preset: row.get(0)?,
            date_start: row.get(1)?,
            date_end: row.get(2)?,
            active_chart: row.get(3)?,
            category_filter_mode: row.get(4)?,
            selected_categories: row.get(5)?,
            selected_members: row.get(6)?,
            selected_vendors: row.get(7)?,
            comparison_mode: row.get(8)?,
            comparison_date_preset: row.get(9)?,
        })
    })?;
    match rows.next() {
        Some(Ok(row)) => Ok(Some(row)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn save_analytics_state(conn: &Connection, state: &AnalyticsFilterRow) -> rusqlite::Result<()> {
    conn.execute(
        // the table's unused granularity/candlestick columns stay in
        // the schema (NOT NULL with defaults) — omitted from the insert, so the
        // defaults apply and no migration is needed.
        "INSERT INTO analytics_filters (
       id, date_preset, date_start, date_end, active_chart,
       category_filter_mode, selected_categories, selected_members,
       selected_vendors, comparison_mode, comparison_date_preset
     ) VALUES (
       1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
     )
     ON CONFLICT(id) DO UPDATE SET
       date_preset = excluded.date_preset,
       date_start = excluded.date_start,
       date_end = excluded.date_end,
       active_chart = excluded.active_chart,
       category_filter_mode = excluded.category_filter_mode,
       selected_categories = excluded.selected_categories,
       selected_members = excluded.selected_members,
       selected_vendors = excluded.selected_vendors,
       comparison_mode = excluded.comparison_mode,
       comparison_date_preset = excluded.comparison_date_preset",
        params![
            state.date_preset,
            state.date_start,
            state.date_end,
            state.active_chart,
            state.category_filter_mode,
            state.selected_categories,
            state.selected_members,
            state.selected_vendors,
            state.comparison_mode,
            state.comparison_date_preset,
        ],
    )?;
    Ok(())
}

pub struct AnalyticsFilterRow {
    pub date_preset: String,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
    pub active_chart: String,
    pub category_filter_mode: String,
    pub selected_categories: String,
    pub selected_members: String,
    pub selected_vendors: String,
    pub comparison_mode: String,
    pub comparison_date_preset: String,
}

pub const CATEGORY_SPLITS_V9_MIGRATION: &str = "category_splits_v9";

pub fn migrate_category_splits_v9(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, CATEGORY_SPLITS_V9_MIGRATION)? {
        return Ok(());
    }
    conn.execute_batch(
        "
    CREATE TABLE IF NOT EXISTS category_splits (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      household_id INTEGER NOT NULL REFERENCES households(id),
      category TEXT NOT NULL,
      member_name TEXT NOT NULL,
      percentage REAL NOT NULL,
      UNIQUE(household_id, category, member_name)
    );
    ",
    )?;
    mark_migration_applied(conn, CATEGORY_SPLITS_V9_MIGRATION)?;
    Ok(())
}

pub fn load_category_splits(
    conn: &Connection,
    household_id: i64,
) -> rusqlite::Result<Vec<CategorySplit>> {
    let mut stmt = conn.prepare(
    "SELECT id, category, member_name, percentage FROM category_splits WHERE household_id = ?1 ORDER BY category, member_name",
  )?;
    let rows = stmt.query_map(params![household_id], |row| {
        Ok(CategorySplit {
            id: row.get(0)?,
            category: row.get(1)?,
            member_name: row.get(2)?,
            percentage: row.get(3)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
}

pub const SIGN_PROTECTED_V10_MIGRATION: &str = "sign_protected_categories_v10";

/// idempotent re-sign of legacy amounts (stored all-positive) to
/// the new signed convention. (Formerly also seeded protected parents.)
pub fn migrate_sign_protected_v10(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, SIGN_PROTECTED_V10_MIGRATION)? {
        return Ok(());
    }
    // idempotent re-sign — legacy rows stored all-positive debits,
    // so a blind flip breaks on re-run and on income rows. Normalize magnitude
    // first, then sign by category: Income tree = credit, everything else =
    // debit (excluded rows keep magnitude, filtered out by label downstream).
    conn.execute("UPDATE expenses SET amount_cents = abs(amount_cents)", [])?;
    conn.execute(
        "UPDATE expenses SET amount_cents = -amount_cents
     WHERE NOT EXISTS (
       SELECT 1 FROM categories c LEFT JOIN categories p ON c.parent_id = p.id
       WHERE (c.name = expenses.category OR (p.name || ' / ' || c.name) = expenses.category)
         AND COALESCE(p.name, c.name) = 'Income'
     )",
        [],
    )?;
    // protected-parent seeding removed — exclusion is a user-set
    // flag now (migrate_category_exclusion_v12). Kept as a no-op marker so
    // already-migrated DBs don't re-run the sign normalization.
    mark_migration_applied(conn, SIGN_PROTECTED_V10_MIGRATION)?;
    Ok(())
}

pub const DEMO_CLEANUP_V11_MIGRATION: &str = "demo_data_cleanup_v11";

/// one-time purge of fake demo data (seeded accounts/expenses/
/// starter rules) and poisoned vendor rules — single-letter and
/// transaction-type patterns like 'e' or 'bill payment' substring-match
/// nearly every vendor. Also adds accounts.csv_name so imports remember
/// which detected CSV account value maps to which user-named account.
pub fn migrate_demo_cleanup_v11(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, DEMO_CLEANUP_V11_MIGRATION)? {
        return Ok(());
    }
    let _ = conn.execute("ALTER TABLE accounts ADD COLUMN csv_name TEXT", []);
    conn.execute(
        "DELETE FROM expenses WHERE (description, vendor, date) IN (
       ('Groceries','Groceries','2026-05-01'),
       ('Electric bill','Electric Company','2026-05-03'),
       ('Date night','Restaurant','2026-05-09'),
       ('Gas','Gas Station','2026-05-12'),
       ('Internet','Internet Provider','2026-05-15')
     )",
        [],
    )?;
    conn.execute(
        "DELETE FROM vendor_category_rules WHERE category IS NULL OR category = ''
       OR length(vendor_pattern) < 3
       OR lower(vendor_pattern) IN (
         'bill payment','federal payment','crd. card bill payment','payroll deposit',
         'mortgage payment','fee','withdrawal','provincial payment','insurance',
         'nsf fee','overlimit fee','annual fee','payment from','needs','the co',
         'interest charges','miscellaneous payment','0810','dal'
       )",
        [],
    )?;
    conn.execute(
        "DELETE FROM accounts WHERE id NOT IN (SELECT DISTINCT account_id FROM expenses)",
        [],
    )?;
    mark_migration_applied(conn, DEMO_CLEANUP_V11_MIGRATION)?;
    Ok(())
}

pub const CATEGORY_EXCLUSION_V12_MIGRATION: &str = "category_exclusion_v12";

/// exclusion becomes a user-set flag on any category instead of
/// name-matched "protected" parents. Adds categories.excluded, flags the
/// known transfer/payment categories, drops the v10-seeded (now unused)
/// 'Credit Card Payments' root, and re-signs expenses idempotently so
/// Income-tree rows stored under the old broken walk flip to credits.
pub fn migrate_category_exclusion_v12(conn: &Connection) -> rusqlite::Result<()> {
    ensure_migrations_table(conn)?;
    if migration_applied(conn, CATEGORY_EXCLUSION_V12_MIGRATION)? {
        return Ok(());
    }
    let _ = conn.execute(
        "ALTER TABLE categories ADD COLUMN excluded INTEGER NOT NULL DEFAULT 0",
        [],
    );
    conn.execute(
    "UPDATE categories SET excluded = 1 WHERE name IN ('Account Transfers', 'Credit Card Payments')",
    [],
  )?;
    conn.execute(
        "DELETE FROM categories WHERE name = 'Credit Card Payments' AND parent_id IS NULL
       AND NOT EXISTS (SELECT 1 FROM categories c WHERE c.parent_id = categories.id)
       AND NOT EXISTS (SELECT 1 FROM expenses e WHERE e.category LIKE 'Credit Card Payments%')",
        [],
    )?;
    conn.execute("UPDATE expenses SET amount_cents = abs(amount_cents)", [])?;
    conn.execute(
        "UPDATE expenses SET amount_cents = -amount_cents
     WHERE NOT EXISTS (
       SELECT 1 FROM categories c LEFT JOIN categories p ON c.parent_id = p.id
       WHERE (c.name = expenses.category OR (p.name || ?1 || c.name) = expenses.category)
         AND COALESCE(p.name, c.name) = 'Income'
     )",
        params![CATEGORY_LABEL_SEP],
    )?;
    mark_migration_applied(conn, CATEGORY_EXCLUSION_V12_MIGRATION)?;
    Ok(())
}

pub fn save_category_split(
    conn: &Connection,
    household_id: i64,
    category: &str,
    member_name: &str,
    percentage: f64,
) -> rusqlite::Result<()> {
    conn.execute(
    "INSERT INTO category_splits (household_id, category, member_name, percentage)
     VALUES (?1, ?2, ?3, ?4)
     ON CONFLICT(household_id, category, member_name) DO UPDATE SET percentage = excluded.percentage",
    params![household_id, category, member_name, percentage],
  )?;
    Ok(())
}

pub fn delete_category_splits_for(
    conn: &Connection,
    household_id: i64,
    category: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM category_splits WHERE household_id = ?1 AND category = ?2",
        params![household_id, category],
    )?;
    Ok(())
}

pub fn delete_all_category_splits(conn: &Connection, household_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM category_splits WHERE household_id = ?1",
        params![household_id],
    )?;
    Ok(())
}
