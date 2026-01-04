use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pentacle {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_url: Option<String>,
    pub icon_url: Option<String>,
    pub default_quality: i32,
    pub article_count: i64,
    pub unread_count: i64,
}

#[tauri::command]
pub fn get_pentacles(state: State<AppState>) -> Result<Vec<Pentacle>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn()
        .prepare(
            r#"
            SELECT
                p.id,
                p.url,
                p.title,
                p.description,
                p.site_url,
                p.icon_url,
                p.default_quality,
                COUNT(f.id) as article_count,
                COUNT(CASE WHEN f.status = 'concealed' THEN 1 END) as unread_count
            FROM pentacles p
            LEFT JOIN fnords f ON f.pentacle_id = p.id
            GROUP BY p.id
            ORDER BY p.title COLLATE NOCASE
            "#,
        )
        .map_err(|e| e.to_string())?;

    let pentacles = stmt
        .query_map([], |row| {
            Ok(Pentacle {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                site_url: row.get(4)?,
                icon_url: row.get(5)?,
                default_quality: row.get(6)?,
                article_count: row.get(7)?,
                unread_count: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(pentacles)
}

#[tauri::command]
pub fn add_pentacle(state: State<AppState>, url: String, title: Option<String>) -> Result<Pentacle, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute(
            "INSERT INTO pentacles (url, title) VALUES (?1, ?2)",
            (&url, &title),
        )
        .map_err(|e| e.to_string())?;

    let id = db.conn().last_insert_rowid();

    Ok(Pentacle {
        id,
        url,
        title,
        description: None,
        site_url: None,
        icon_url: None,
        default_quality: 3,
        article_count: 0,
        unread_count: 0,
    })
}

#[tauri::command]
pub fn delete_pentacle(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn()
        .execute("DELETE FROM pentacles WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}
