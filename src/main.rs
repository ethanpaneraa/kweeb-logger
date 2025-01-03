use anyhow::Context;
use device_query::{DeviceQuery, DeviceState};
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
mod scroll;
mod logger;
mod tray;

use crate::{
    config::Config,
    db::Database,
    metrics::{Metrics, TotalMetrics},
    monitor::calculate_distance,
    scroll::ScrollTracker,
};

pub struct AppState {
    metrics: Mutex<Metrics>,
    total_metrics: Mutex<TotalMetrics>,
    last_mouse_pos: Mutex<(i32, i32)>,
    db: Arc<Database>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logger::setup_logging()?;
    log::info!("Starting kweeb-logger...");

    let _config = Config::load().context("Failed to load configuration")?;
    log::info!("Configuration loaded");

    let database = Database::new().await?;
    let db = Arc::new(database);
    log::info!("Database initialized");

    let state = Arc::new(AppState {
        metrics: Mutex::new(Metrics::default()),
        total_metrics: Mutex::new(TotalMetrics::default()),
        last_mouse_pos: Mutex::new((0, 0)),
        db,
    });

    let mut tray = TrayItem::new("kweeb-logger", "kweeb-logger-tray")?;
    tray::setup_tray(&mut tray, Arc::clone(&state))?;
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

async fn collect_metrics(
    state: Arc<AppState>,
    device_state: DeviceState,
    running: Arc<AtomicBool>,
) {
    let mut last_keys = device_state.get_keys();
    let mut last_mouse = device_state.get_mouse();
    let mut scroll_tracker = ScrollTracker::new();

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

        let scroll_delta = scroll_tracker.get_scroll_delta();
        if scroll_delta > 0 {
            let mut metrics = state.metrics.lock().await;
            let mut total = state.total_metrics.lock().await;
            metrics.scroll_steps += scroll_delta;
            total.total_scroll_steps += scroll_delta;
        }

        if current_mouse.coords != last_mouse.coords {
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

        // Handle mouse clicks
        if current_mouse.button_pressed.iter().zip(last_mouse.button_pressed.iter())
            .any(|(&current, &last)| current && !last) {
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

        let metrics = state.metrics.lock().await;
        
        if let Err(e) = state.db.insert_metrics(
            metrics.keypresses,
            metrics.mouse_clicks,
            metrics.mouse_distance_in,
            metrics.mouse_distance_mi,
            metrics.scroll_steps
        ).await {
            log::error!("Failed to save metrics: {}", e);
            continue;
        }

        drop(metrics); 
        state.metrics.lock().await.reset();
    }
}