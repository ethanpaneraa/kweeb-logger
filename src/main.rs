use anyhow::{Context};
use device_query::{DeviceQuery, DeviceState};
use directories::ProjectDirs;
use sqlx::sqlite::SqlitePool;
use std::{
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    time::Duration,
};
use tokio::sync::Mutex;
use tray_item::TrayItem;

mod config;
mod db;
mod metrics;
mod monitor;

use crate::{
    config::Config,
    metrics::{Metrics, TotalMetrics},
    monitor::calculate_distance,
};

pub struct AppState {
    metrics: Mutex<Metrics>,
    total_metrics: Mutex<TotalMetrics>,
    last_mouse_pos: Mutex<(i32, i32)>,
    db_pool: SqlitePool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging()?;
    log::info!("Starting kweeb-logger...");

    let _config = Config::load().context("Failed to load configuration")?;
    log::info!("Configuration loaded");

    let db_pool = setup_database().await?;
    log::info!("Database initialized");

    let state = Arc::new(AppState {
        metrics: Mutex::new(Metrics::default()),
        total_metrics: Mutex::new(TotalMetrics::default()),
        last_mouse_pos: Mutex::new((0, 0)),
        db_pool,
    });

    let mut tray = TrayItem::new("kweeb-logger", "kweeb-logger-tray")?;
    setup_tray(&mut tray, Arc::clone(&state))?;
    log::info!("System tray initialized");

    let device_state = DeviceState::new();
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    let metrics_state = Arc::clone(&state);
    tokio::spawn(async move {
        collect_metrics(metrics_state, device_state, running_clone).await;
    });

    let save_state = Arc::clone(&state);
    tokio::spawn(async move {
        save_metrics_periodically(save_state).await;
    });

    log::info!("Metrics collection started");

    tokio::signal::ctrl_c().await?;
    running.store(false, Ordering::SeqCst);
    log::info!("Shutting down kweeb-logger...");

    Ok(())
}

fn setup_logging() -> anyhow::Result<()> {
    let proj_dirs = ProjectDirs::from("com", "kweeb-logger", "logger")
        .context("Failed to get project directories")?;
    
    let log_dir = proj_dirs.data_dir();
    println!("Creating log directory at: {}", log_dir.display());
    std::fs::create_dir_all(&log_dir)?;
    
    let log_file = log_dir.join("kweeb-logger.log");
    println!("Log file will be at: {}", log_file.display());
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .filter_level(log::LevelFilter::Info)
        .init();
    
    log::info!("Logging initialized at {}", log_file.display());
    Ok(())
}


async fn setup_database() -> anyhow::Result<SqlitePool> {
    let proj_dirs = ProjectDirs::from("com", "kweeb-logger", "logger")
        .context("Failed to get project directories")?;

    let data_dir = proj_dirs.data_dir();
    println!("Creating data directory at: {}", data_dir.display());
    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("kweeb-logger.db");
    println!("Database will be at: {}", db_path.display());

    if !db_path.exists() {
        std::fs::File::create(&db_path)?;
        println!("Created new database file");
    }

    let db_url = format!("sqlite:{}", db_path.display());
    println!("Connecting to database at: {}", db_url);

    let pool = SqlitePool::connect(&db_url)
        .await
        .context("Failed to connect to database")?;

    println!("Successfully connected to database");

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

    println!("Database schema initialized");
    Ok(pool)
}
fn setup_tray(tray: &mut TrayItem, state: Arc<AppState>) -> anyhow::Result<()> {
    tray.add_menu_item("Kweeb Logger", Box::new(|| ()))?;
    tray.add_menu_item("", Box::new(|| ()))?; 
    tray.add_menu_item("Quit", Box::new(|| {
        println!("Quitting...");
        std::process::exit(0);
    }))?;

    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let total = state_clone.total_metrics.lock().await;
            
            log::info!(
                "Stats - Keypresses: {}, Clicks: {}, Distance: {:.2}mi, Scrolls: {}", 
                total.total_keypresses,
                total.total_mouse_clicks,
                total.total_mouse_distance_mi,
                total.total_scroll_steps
            );
        }
    });

    Ok(())
}

async fn collect_metrics(
    state: Arc<AppState>,
    device_state: DeviceState,
    running: Arc<AtomicBool>,
) {
    let mut last_keys = device_state.get_keys();
    let mut last_mouse = device_state.get_mouse();

    while running.load(Ordering::SeqCst) {
        let current_keys = device_state.get_keys();
        let new_keys: Vec<_> = current_keys
            .iter()
            .filter(|k| !last_keys.contains(k))
            .collect();

        if !new_keys.is_empty() {
            let mut metrics = state.metrics.lock().await;
            let mut total = state.total_metrics.lock().await;
            metrics.keypresses += new_keys.len() as i32;
            total.total_keypresses += new_keys.len() as i32;
        }

        let current_mouse = device_state.get_mouse();
        if current_mouse != last_mouse {
            let mut metrics = state.metrics.lock().await;
            let mut total = state.total_metrics.lock().await;
            let mut last_pos = state.last_mouse_pos.lock().await;

            let distance = calculate_distance(
                last_pos.0,
                last_pos.1,
                current_mouse.coords.0,
                current_mouse.coords.1,
            );

            metrics.mouse_distance_in += distance;
            metrics.mouse_distance_mi += distance / 63360.0;
            total.total_mouse_distance_in += distance;
            total.total_mouse_distance_mi += distance / 63360.0;

            *last_pos = current_mouse.coords;
        }

        if current_mouse.button_pressed != last_mouse.button_pressed {
            let mut metrics = state.metrics.lock().await;
            let mut total = state.total_metrics.lock().await;
            metrics.mouse_clicks += 1;
            total.total_mouse_clicks += 1;
        }

        last_keys = current_keys;
        last_mouse = current_mouse;

        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn save_metrics_periodically(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        let mut metrics = state.metrics.lock().await;
        
        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO metrics 
            (keypresses, mouse_clicks, mouse_distance_in, mouse_distance_mi, scroll_steps)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(metrics.keypresses)
        .bind(metrics.mouse_clicks)
        .bind(metrics.mouse_distance_in)
        .bind(metrics.mouse_distance_mi)
        .bind(metrics.scroll_steps)
        .execute(&state.db_pool)
        .await
        {
            log::error!("Failed to save metrics: {}", e);
            continue;
        }

        metrics.reset();
    }
}






