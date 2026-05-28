use rusqlite::{params, Connection};
use std::env;
use std::fs;
use std::path::PathBuf;
use crate::models::*;
use eframe::egui::{Color32, ecolor::Hsva};

pub fn db_path() -> PathBuf {
  let base = env::var("APPDATA")
    .map(PathBuf::from)
    .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
  base.join("TwoCents").join("twocents.sqlite")
}

pub fn open_database() -> rusqlite::Result<Connection> {
  let path = db_path();
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).map_err(|err| rusqlite::Error::ToSqlConversionFailure(err.into()))?;
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

  let account_count: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))?;
  if account_count == 0 {
    conn.execute(
      "INSERT INTO accounts (household_id, name, kind, balance_cents) VALUES (1, ?1, ?2, ?3)",
      params!["Household Checking", "checking", 426_550],
    )?;
    conn.execute(
      "INSERT INTO accounts (household_id, name, kind, balance_cents) VALUES (1, ?1, ?2, ?3)",
      params!["Shared Savings", "savings", 1_240_000],
    )?;
  }

  let expense_count: i64 = conn.query_row("SELECT COUNT(*) FROM expenses", [], |row| row.get(0))?;
  if expense_count == 0 {
    for (account_id, description, vendor, category, amount_cents, date) in [
      (
        1,
        "Groceries",
        "Groceries",
        "Food › Groceries",
        18_642,
        "2026-05-01",
      ),
      (
        1,
        "Electric bill",
        "Electric Company",
        "Utilities › Electricity",
        14_280,
        "2026-05-03",
      ),
      (
        1,
        "Date night",
        "Restaurant",
        "Food › Dining Out",
        9_875,
        "2026-05-09",
      ),
      (
        1,
        "Gas",
        "Gas Station",
        "Transportation › Fuel",
        6_122,
        "2026-05-12",
      ),
      (
        1,
        "Internet",
        "Internet Provider",
        "Utilities › Internet",
        7_999,
        "2026-05-15",
      ),
    ] {
      conn.execute(
        "INSERT INTO expenses (household_id, account_id, description, vendor, category, member, amount_cents, date) VALUES (1, ?1, ?2, ?3, ?4, '', ?5, ?6)",
        params![account_id, description, vendor, category, amount_cents, date],
      )?;
    }
  }

  seed_default_categories(&conn, 1)?;
  for (pattern, category) in [
    ("grocery", "Food › Groceries"),
    ("market", "Food › Groceries"),
    ("electric", "Utilities › Electricity"),
    ("internet", "Utilities › Internet"),
    ("restaurant", "Food › Dining Out"),
    ("gas", "Transportation › Fuel"),
  ] {
    conn.execute(
      "INSERT OR IGNORE INTO vendor_category_rules (household_id, vendor_pattern, category) VALUES (1, ?1, ?2)",
      params![pattern, category],
    )?;
  }
  Ok(conn)
}

pub fn migrate_households(conn: &Connection) -> rusqlite::Result<()> {
  let household_count: i64 = conn.query_row("SELECT COUNT(*) FROM households", [], |row| row.get(0))?;
  if household_count == 0 {
    conn.execute("INSERT INTO households (name) VALUES ('My Household')", [])?;
  }
  for table in ["accounts", "expenses", "categories", "vendor_category_rules"] {
    let _ = conn.execute(&format!("ALTER TABLE {table} ADD COLUMN household_id INTEGER"), []);
    conn.execute(
      &format!("UPDATE {table} SET household_id = 1 WHERE household_id IS NULL"),
      [],
    )?;
  }
  Ok(())
}

pub fn load_active_household(conn: &Connection) -> rusqlite::Result<(i64, String)> {
  conn.query_row("SELECT id, name FROM households ORDER BY id LIMIT 1", [], |row| {
    Ok((row.get(0)?, row.get(1)?))
  })
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
  let _ = conn.execute("ALTER TABLE household_members ADD COLUMN color_rgb INTEGER", []);
  let mut stmt = conn.prepare("SELECT id, COALESCE(color_rgb, 0) FROM household_members ORDER BY id")?;
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

pub fn load_household_members(conn: &Connection, household_id: i64) -> rusqlite::Result<Vec<HouseholdMember>> {
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

pub fn update_self_member_name(conn: &Connection, household_id: i64, name: &str) -> rusqlite::Result<()> {
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

pub fn add_household_member(conn: &Connection, household_id: i64, name: &str) -> rusqlite::Result<()> {
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

pub fn delete_household_member(conn: &Connection, household_id: i64, member_id: i64) -> rusqlite::Result<()> {
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
    "SELECT name, kind, balance_cents FROM accounts WHERE household_id = ?1 ORDER BY id",
  )?;
  let rows = stmt
    .query_map(params![household_id], |row| {
      Ok(Account {
        name: row.get(0)?,
        kind: row.get(1)?,
        balance_cents: row.get(2)?,
      })
    })?
    .collect();
  rows
}

pub fn load_expenses(conn: &Connection, household_id: i64) -> rusqlite::Result<Vec<Expense>> {
  let mut stmt = conn.prepare(
    "SELECT id, date, amount_cents, COALESCE(member, ''), category, COALESCE(vendor, ''), description FROM expenses WHERE household_id = ?1 ORDER BY date DESC, id DESC",
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
      })
    })?
    .collect();
  rows
}

pub fn load_categories(conn: &Connection, household_id: i64) -> rusqlite::Result<Vec<Category>> {
  let mut stmt = conn.prepare(
    "SELECT id, name, parent_id, COALESCE(color_rgb, 0) FROM categories WHERE household_id = ?1 ORDER BY COALESCE(parent_id, id), name",
  )?;
  let rows = stmt
    .query_map(params![household_id], |row| {
      Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        parent_id: row.get(2)?,
        color: rgb_to_color(row.get(3)?),
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
  conn.execute("INSERT INTO app_migrations (name) VALUES (?1)", params![name])?;
  Ok(())
}

pub const CATEGORY_DEFAULTS_V2_MIGRATION: &str = "category_defaults_v2";
pub const CATEGORY_PALETTE_COLORS_V3_MIGRATION: &str = "category_palette_colors_v3";
pub const THEME_ACCENT_MEMBERS_V4_MIGRATION: &str = "theme_accent_members_v4";
pub const THEME_ACCENT_CATEGORIES_V4_MIGRATION: &str = "theme_accent_categories_v4";

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

pub fn reset_household_categories_to_defaults(conn: &Connection, household_id: i64) -> rusqlite::Result<()> {
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
  let mut stmt = conn.prepare("SELECT rowid, COALESCE(color_rgb, 0) FROM categories ORDER BY rowid")?;
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
  let hovered_bg = Color32::from_rgb(0x3f, 0x44, 0x47);
  let base = Hsva::from_srgba_unmultiplied([hovered_bg.r(), hovered_bg.g(), hovered_bg.b(), 255]);
  (0..ACCENT_PALETTE_COUNT)
    .map(|i| {
      let mut hsva = base;
      hsva.h = (base.h + i as f32 / ACCENT_PALETTE_COUNT as f32) % 1.0;
      hsva.s = (0.45 + (i % 3) as f32 * 0.06).clamp(0.45, 0.62);
      hsva.v = (0.55 + ((i / 3) % 3) as f32 * 0.08).clamp(0.55, 0.78);
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

pub fn subcategory_color_from_parent(parent: Color32, sub_index: usize, sub_count: usize) -> Color32 {
  let hsva = Hsva::from_srgba_unmultiplied([parent.r(), parent.g(), parent.b(), 255]);
  let n = sub_count.max(1) as f32;
  let idx = sub_index as f32;
  let mut sub = hsva;
  sub.h = (sub.h + (idx / n) * 0.06 + idx * 0.01) % 1.0;
  sub.s = (sub.s * (0.92 + idx * 0.03 / n)).clamp(0.40, 0.75);
  sub.v = (sub.v * (1.02 - idx * 0.02 / n)).clamp(0.50, 0.85);
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
  Color32::from_rgb(((rgb >> 16) & 0xFF) as u8, ((rgb >> 8) & 0xFF) as u8, (rgb & 0xFF) as u8)
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

pub fn add_parent_category_db(conn: &Connection, household_id: i64, name: &str) -> rusqlite::Result<()> {
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
    return Err(rusqlite::Error::InvalidParameterName("parent category not found".into()));
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
      Some(format!("subcategory '{name}' already exists under that parent")),
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

pub fn delete_category_from_db(conn: &Connection, household_id: i64, category_id: i64) -> rusqlite::Result<()> {
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
  Ok(())
}

pub fn update_expense_row(conn: &Connection, row: &Expense, field: &str) -> rusqlite::Result<()> {
  match field {
    "date" => { conn.execute("UPDATE expenses SET date = ?1 WHERE id = ?2", params![row.date, row.id])?; }
    "amount" => { conn.execute("UPDATE expenses SET amount_cents = ?1 WHERE id = ?2", params![row.amount_cents, row.id])?; }
    "category" => {
      conn.execute("UPDATE expenses SET category = ?1 WHERE id = ?2", params![row.category, row.id])?;
    }
    "member" => {
      conn.execute("UPDATE expenses SET member = ?1 WHERE id = ?2", params![row.member, row.id])?;
    }
    "vendor" => { conn.execute("UPDATE expenses SET vendor = ?1 WHERE id = ?2", params![row.vendor, row.id])?; }
    "description" => { conn.execute("UPDATE expenses SET description = ?1 WHERE id = ?2", params![row.description, row.id])?; }
    _ => {}
  }
  Ok(())
}

pub fn save_import_rows(conn: &Connection, household_id: i64, rows: &[ImportRow]) -> rusqlite::Result<usize> {
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
    conn.execute(
      "INSERT INTO expenses (household_id, account_id, description, vendor, category, member, amount_cents, date) VALUES (?1, 1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![
        household_id,
        row.description,
        row.vendor,
        category,
        row.member,
        row.amount_cents,
        row.date
      ],
    )?;
    if !category.is_empty() && !category.contains("Uncategorized") {
      conn.execute(
        "INSERT OR IGNORE INTO vendor_category_rules (household_id, vendor_pattern, category) VALUES (?1, ?2, ?3)",
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
  description
    .split(['*', '-', '#'])
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
    candidates.iter().any(|candidate| normalized.contains(candidate))
  })
}

pub fn category_for_vendor(conn: &Connection, household_id: i64, vendor: &str) -> String {
  let normalized = vendor.to_lowercase();
  let mut stmt = match conn.prepare(
    "SELECT vendor_pattern, category FROM vendor_category_rules WHERE household_id = ?1 ORDER BY id",
  ) {
    Ok(stmt) => stmt,
    Err(_) => return "Uncategorized".to_string(),
  };
  let rules = stmt
    .query_map(params![household_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
    .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
    .unwrap_or_default();
  rules
    .into_iter()
    .find_map(|(pattern, category)| normalized.contains(&pattern.to_lowercase()).then_some(category))
    .unwrap_or_else(|| "Uncategorized".to_string())
}

pub fn parse_csv_statement(conn: &Connection, household_id: i64, path: &std::path::Path) -> Result<Vec<ImportRow>, String> {
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
  let amount_idx = find_column(&headers, &["amount", "debit", "withdrawal", "charge"]);
  let description_idx = find_column(&headers, &["description", "memo", "details", "transaction", "payee", "merchant"]);
  let category_idx = find_column(&headers, &["category"]);

  let mut rows = Vec::new();
  for record in reader.records() {
    let record = record.map_err(|err| format!("Bad CSV row: {err}"))?;
    let date = date_idx.and_then(|idx| record.get(idx)).map(normalize_date).unwrap_or_else(|| "2026-01-01".to_string());
    let description = description_idx.and_then(|idx| record.get(idx)).unwrap_or("").trim().to_string();
    let amount_cents = amount_idx.and_then(|idx| record.get(idx)).and_then(parse_amount_cents).unwrap_or(0);
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
    rows.push(ImportRow {
      date,
      amount_input: money(amount_cents).replace('$', ""),
      amount_cents,
      member: load_default_member_name(conn, household_id).unwrap_or_default(),
      category,
      vendor,
      description,
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
          "SELECT COUNT(*) FROM expenses WHERE household_id = ?1 AND date = ?2 AND amount_cents = ?3 AND lower(COALESCE(vendor, '')) = lower(?4) AND lower(description) = lower(?5)",
          params![household_id, row.date, row.amount_cents, row.vendor, row.description],
          |count_row| count_row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0
    })
    .count()
}
