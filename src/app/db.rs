use rusqlite::{Connection, Result, params, OptionalExtension, ToSql, Row};
use rusqlite::types::Value;
use crate::app::model::SoftwareEntry;
use chrono::{DateTime, Local};
use serde_json;
use crate::app::model::TagGroup;

// 所有字段名（用于 SELECT）
const ALL_FIELDS: &[&str] = &[
    "id", "name", "alias", "repo_url", "current_version", "latest_version",
    "asset_name", "install_path", "executable_path", "notes", "tags",
    "created_at", "updated_at", "linked_folders"
];

// 字段索引常量（必须与 ALL_FIELDS 顺序一致）
const IDX_ID: usize = 0;
const IDX_NAME: usize = 1;
const IDX_ALIAS: usize = 2;
const IDX_REPO_URL: usize = 3;
const IDX_CURRENT_VERSION: usize = 4;
const IDX_LATEST_VERSION: usize = 5;
const IDX_ASSET_NAME: usize = 6;
const IDX_INSTALL_PATH: usize = 7;
const IDX_EXECUTABLE_PATH: usize = 8;
const IDX_NOTES: usize = 9;
const IDX_TAGS: usize = 10;
const IDX_CREATED_AT: usize = 11;
const IDX_UPDATED_AT: usize = 12;
const IDX_LINKED_FOLDERS: usize = 13;

// 插入时使用的字段（不含 id）
const INSERT_FIELDS: &[&str] = &[
    "name", "alias", "repo_url", "current_version", "latest_version",
    "asset_name", "install_path", "executable_path", "notes", "tags",
    "created_at", "updated_at", "linked_folders"
];

// 更新时使用的字段（不含 id 和 created_at）
const UPDATE_FIELDS: &[&str] = &[
    "name", "alias", "repo_url", "current_version", "latest_version",
    "asset_name", "install_path", "executable_path", "notes", "tags",
    "updated_at", "linked_folders"
];

// 从数据库行解析为 SoftwareEntry
fn row_to_entry(row: &Row) -> Result<SoftwareEntry> {
    let tags_json: String = row.get(IDX_TAGS)?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let linked_folders_json: Option<String> = row.get(IDX_LINKED_FOLDERS)?;
    let linked_folders: Vec<crate::app::model::LinkedFolder> = linked_folders_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    Ok(SoftwareEntry {
        id: row.get(IDX_ID)?,
        name: row.get(IDX_NAME)?,
        alias: row.get(IDX_ALIAS)?,
        repo_url: row.get(IDX_REPO_URL)?,
        current_version: row.get(IDX_CURRENT_VERSION)?,
        latest_version: row.get(IDX_LATEST_VERSION)?,
        asset_name: row.get(IDX_ASSET_NAME)?,
        install_path: row.get(IDX_INSTALL_PATH)?,
        executable_path: row.get(IDX_EXECUTABLE_PATH)?,
        notes: row.get(IDX_NOTES)?,
        tags,
        linked_folders,
        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(IDX_CREATED_AT)?)
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(|_| Local::now()),
        updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(IDX_UPDATED_AT)?)
            .map(|dt| dt.with_timezone(&Local))
            .unwrap_or_else(|_| Local::now()),
    })
}

// 将 SoftwareEntry 转换为插入时的参数值列表（顺序与 INSERT_FIELDS 一致）
fn entry_to_insert_values(entry: &SoftwareEntry) -> Vec<Value> {
    vec![
        entry.name.clone().into(),
        entry.alias.clone().into(),
        entry.repo_url.clone().into(),
        entry.current_version.clone().into(),
        entry.latest_version.clone().into(),
        entry.asset_name.clone().into(),
        entry.install_path.clone().into(),
        entry.executable_path.clone().into(),
        entry.notes.clone().into(),
        serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string()).into(),
        entry.created_at.to_rfc3339().into(),
        entry.updated_at.to_rfc3339().into(),
        serde_json::to_string(&entry.linked_folders).unwrap_or_else(|_| "[]".to_string()).into(),
    ]
}

// 将 SoftwareEntry 转换为更新时的参数值列表（顺序与 UPDATE_FIELDS 一致，不含 id）
fn entry_to_update_values(entry: &SoftwareEntry) -> Vec<Value> {
    vec![
        entry.name.clone().into(),
        entry.alias.clone().into(),
        entry.repo_url.clone().into(),
        entry.current_version.clone().into(),
        entry.latest_version.clone().into(),
        entry.asset_name.clone().into(),
        entry.install_path.clone().into(),
        entry.executable_path.clone().into(),
        entry.notes.clone().into(),
        serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string()).into(),
        entry.updated_at.to_rfc3339().into(),
        serde_json::to_string(&entry.linked_folders).unwrap_or_else(|_| "[]".to_string()).into(),
    ]
}

pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("data.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS software (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            alias TEXT NOT NULL,
            repo_url TEXT NOT NULL,
            current_version TEXT NOT NULL,
            latest_version TEXT,
            asset_name TEXT NOT NULL,
            install_path TEXT,
            executable_path TEXT,
            notes TEXT,
            tags TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    // 新建 tag_groups 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tag_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            tags TEXT NOT NULL  -- JSON 数组
        )",
        [],
    )?;

    // 为旧数据库添加 linked_folders 列（如果不存在）
    let _ = conn.execute("ALTER TABLE software ADD COLUMN linked_folders TEXT", []);

    Ok(conn)
}

pub fn get_all_software(conn: &Connection) -> Result<Vec<SoftwareEntry>> {
    let fields = ALL_FIELDS.join(", ");
    let sql = format!("SELECT {} FROM software ORDER BY updated_at DESC", fields);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_entry)?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn get_software_by_id(conn: &Connection, id: i64) -> Result<Option<SoftwareEntry>> {
    let fields = ALL_FIELDS.join(", ");
    let sql = format!("SELECT {} FROM software WHERE id = ?", fields);
    let mut stmt = conn.prepare(&sql)?;
    let result = stmt.query_row([id], row_to_entry).optional()?;
    Ok(result)
}

pub fn insert_software(conn: &Connection, entry: &SoftwareEntry) -> Result<i64> {
    let fields = INSERT_FIELDS.join(", ");
    let placeholders = (0..INSERT_FIELDS.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!("INSERT INTO software ({}) VALUES ({})", fields, placeholders);
    let values = entry_to_insert_values(entry);
    let params: Vec<&dyn ToSql> = values.iter().map(|v| v as &dyn ToSql).collect();
    conn.execute(&sql, params.as_slice())?;
    Ok(conn.last_insert_rowid())
}

pub fn update_software(conn: &Connection, entry: &SoftwareEntry) -> Result<usize> {
    let set_clause = UPDATE_FIELDS.iter()
        .map(|f| format!("{} = ?", f))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("UPDATE software SET {} WHERE id = ?", set_clause);
    let mut values = entry_to_update_values(entry);
    values.push(entry.id.expect("更新时 ID 不能为空").into());
    let params: Vec<&dyn ToSql> = values.iter().map(|v| v as &dyn ToSql).collect();
    conn.execute(&sql, params.as_slice())
}

pub fn delete_software(conn: &Connection, id: i64) -> Result<usize> {
    conn.execute("DELETE FROM software WHERE id = ?", [id])
}

pub fn search_by_tag(conn: &Connection, tag: &str) -> Result<Vec<SoftwareEntry>> {
    let all = get_all_software(conn)?;
    Ok(all.into_iter().filter(|e| e.tags.iter().any(|t| t.contains(tag))).collect())
}

pub fn save_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        [key, value],
    )?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
    let result = stmt.query_row([key], |row| row.get(0)).optional()?;
    Ok(result)
}

// 获取所有标签组
pub fn get_all_tag_groups(conn: &Connection) -> Result<Vec<TagGroup>> {
    let mut stmt = conn.prepare("SELECT id, name, tags FROM tag_groups ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        let tags_json: String = row.get(2)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        Ok(TagGroup {
            id: row.get(0)?,
            name: row.get(1)?,
            tags,
        })
    })?;
    let mut groups = Vec::new();
    for row in rows {
        groups.push(row?);
    }
    Ok(groups)
}

// 保存标签组（插入或更新）
pub fn save_tag_group(conn: &Connection, group: &TagGroup) -> Result<i64> {
    if let Some(id) = group.id {
        // 更新
        conn.execute(
            "UPDATE tag_groups SET name = ?1, tags = ?2 WHERE id = ?3",
            params![
                group.name,
                serde_json::to_string(&group.tags).unwrap_or_else(|_| "[]".to_string()),
                id
            ],
        )?;
        Ok(id)
    } else {
        // 插入
        conn.execute(
            "INSERT INTO tag_groups (name, tags) VALUES (?1, ?2)",
            params![
                group.name,
                serde_json::to_string(&group.tags).unwrap_or_else(|_| "[]".to_string())
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

// 删除标签组
pub fn delete_tag_group(conn: &Connection, id: i64) -> Result<usize> {
    conn.execute("DELETE FROM tag_groups WHERE id = ?", [id])
}
