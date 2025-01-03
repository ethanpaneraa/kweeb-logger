use anyhow::{Context, Result};
use directories::ProjectDirs;
use sqlx::{sqlite::SqlitePool, Row};  
use std::path::PathBuf;
use crate::metrics::TotalMetrics;

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

    pub async fn get_total_metrics(&self) -> Result<TotalMetrics> {
        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(keypresses), 0) as total_keypresses,
                COALESCE(SUM(mouse_clicks), 0) as total_mouse_clicks,
                COALESCE(SUM(mouse_distance_in), 0.0) as total_mouse_distance_in,
                COALESCE(SUM(mouse_distance_mi), 0.0) as total_mouse_distance_mi,
                COALESCE(SUM(scroll_steps), 0) as total_scroll_steps
            FROM metrics
            "#
        )
        .fetch_one(self.pool())
        .await
        .context("Failed to fetch total metrics")?;

        Ok(TotalMetrics {
            total_keypresses: row.try_get(0)
                .context("Failed to get total_keypresses")?,
            total_mouse_clicks: row.try_get(1)
                .context("Failed to get total_mouse_clicks")?,
            total_mouse_distance_in: row.try_get(2)
                .context("Failed to get total_mouse_distance_in")?,
            total_mouse_distance_mi: row.try_get(3)
                .context("Failed to get total_mouse_distance_mi")?,
            total_scroll_steps: row.try_get(4)
                .context("Failed to get total_scroll_steps")?,
        })
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

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
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