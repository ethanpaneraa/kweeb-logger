use sqlx::sqlite::SqlitePool;
use anyhow::Result;
use std::path::PathBuf;
use directories::ProjectDirs;

pub async fn initialize_db(db_path: Option<PathBuf>) -> Result<SqlitePool> {
    let db_path = db_path.unwrap_or_else(|| {
        let proj_dirs = ProjectDirs::from("com", "kweeb-logger", "logger")
            .expect("Failed to get project directories");
        proj_dirs.data_dir().join("kweeb-logger.db")
    });

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite:{}", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS metrics (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            keypresses INTEGER,
            mouse_clicks INTEGER,
            mouse_distance_in REAL,
            mouse_distance_mi REAL,
            scroll_steps INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
