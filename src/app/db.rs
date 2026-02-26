use rusqlite::{Connection, Result, params, OptionalExtension};
use crate::app::model::SoftwareEntry;
use chrono::{Local, DateTime};
use serde_json;

/// 初始化数据库表
pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("data.db")?;
    
    // 创建软件条目表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS software (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            repo_owner TEXT NOT NULL,
            repo_name TEXT NOT NULL,
            current_version TEXT NOT NULL,
            latest_version TEXT,
            install_path TEXT,
            executable_path TEXT,
            notes TEXT,
            tags TEXT,  -- JSON 数组存储
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;
    
    // 创建设置表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;
    
    Ok(conn)
}

/// 插入新软件条目
pub fn insert_software(conn: &Connection, entry: &SoftwareEntry) -> Result<i64> {
    conn.execute(
        "INSERT INTO software (
            name, repo_owner, repo_name, current_version, latest_version,
            install_path, executable_path, notes, tags, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            entry.name,
            entry.repo_owner,
            entry.repo_name,
            entry.current_version,
            entry.latest_version,
            entry.install_path,
            entry.executable_path,
            entry.notes,
            serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string()),
            entry.created_at.to_rfc3339(),
            entry.updated_at.to_rfc3339()
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// 查询所有软件条目
pub fn get_all_software(conn: &Connection) -> Result<Vec<SoftwareEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, repo_owner, repo_name, current_version, latest_version,
                install_path, executable_path, notes, tags, created_at, updated_at
         FROM software ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        let tags_json: String = row.get(9)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        Ok(SoftwareEntry {
            id: row.get(0)?,
            name: row.get(1)?,
            repo_owner: row.get(2)?,
            repo_name: row.get(3)?,
            current_version: row.get(4)?,
            latest_version: row.get(5)?,
            install_path: row.get(6)?,
            executable_path: row.get(7)?,
            notes: row.get(8)?,
            tags,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now()),
        })
    })?;
    
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// 根据ID查询单个软件
pub fn get_software_by_id(conn: &Connection, id: i64) -> Result<Option<SoftwareEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, repo_owner, repo_name, current_version, latest_version,
                install_path, executable_path, notes, tags, created_at, updated_at
         FROM software WHERE id = ?"
    )?;
    let result = stmt.query_row(params![id], |row| {
        let tags_json: String = row.get(9)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        Ok(SoftwareEntry {
            id: row.get(0)?,
            name: row.get(1)?,
            repo_owner: row.get(2)?,
            repo_name: row.get(3)?,
            current_version: row.get(4)?,
            latest_version: row.get(5)?,
            install_path: row.get(6)?,
            executable_path: row.get(7)?,
            notes: row.get(8)?,
            tags,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now()),
        })
    }).optional()?;
    Ok(result)
}

/// 更新软件条目
pub fn update_software(conn: &Connection, entry: &SoftwareEntry) -> Result<usize> {
    let id = entry.id.expect("更新时 ID 不能为空");
    conn.execute(
        "UPDATE software SET
            name = ?1,
            repo_owner = ?2,
            repo_name = ?3,
            current_version = ?4,
            latest_version = ?5,
            install_path = ?6,
            executable_path = ?7,
            notes = ?8,
            tags = ?9,
            updated_at = ?10
         WHERE id = ?11",
        params![
            entry.name,
            entry.repo_owner,
            entry.repo_name,
            entry.current_version,
            entry.latest_version,
            entry.install_path,
            entry.executable_path,
            entry.notes,
            serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string()),
            Local::now().to_rfc3339(),
            id
        ],
    )
}

/// 删除软件条目
pub fn delete_software(conn: &Connection, id: i64) -> Result<usize> {
    conn.execute("DELETE FROM software WHERE id = ?", params![id])
}

/// 根据标签搜索软件（返回包含任一标签的条目）
pub fn search_by_tag(conn: &Connection, tag: &str) -> Result<Vec<SoftwareEntry>> {
    let all = get_all_software(conn)?;
    Ok(all.into_iter().filter(|e| e.tags.iter().any(|t| t.contains(tag))).collect())
}

/// 保存设置项
pub fn save_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

/// 读取设置项
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
    let result = stmt.query_row(params![key], |row| row.get(0)).optional()?;
    Ok(result)
}
