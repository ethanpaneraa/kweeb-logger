// db.rs
use anyhow::{Context, Result};
use directories::ProjectDirs;
use sqlx::sqlite::SqlitePool;
use std::path::PathBuf;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let db_path = get_database_path()?;
        let pool = initialize_database(&db_path).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn insert_metrics(
        &self,
        keypresses: i32,
        mouse_clicks: i32,
        mouse_distance_in: f64,
        mouse_distance_mi: f64,
        scroll_steps: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO metrics 
            (keypresses, mouse_clicks, mouse_distance_in, mouse_distance_mi, scroll_steps)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(keypresses)
        .bind(mouse_clicks)
        .bind(mouse_distance_in)
        .bind(mouse_distance_mi)
        .bind(scroll_steps)
        .execute(self.pool())
        .await
        .context("Failed to insert metrics")?;

        Ok(())
    }
}

fn get_database_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "kweeb-logger", "logger")
        .context("Failed to get project directories")?;

    let data_dir = proj_dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;
    
    Ok(data_dir.join("kweeb-logger.db"))
}

async fn initialize_database(db_path: &PathBuf) -> Result<SqlitePool> {
    if !db_path.exists() {
        std::fs::File::create(db_path)?;
        log::info!("Created new database file at {}", db_path.display());
    }

    let db_url = format!("sqlite:{}", db_path.display());
    let pool = SqlitePool::connect(&db_url)
        .await
        .context("Failed to connect to database")?;

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
        );
        "#,
    )
    .execute(&pool)
    .await
    .context("Failed to create metrics table")?;

    Ok(pool)
}