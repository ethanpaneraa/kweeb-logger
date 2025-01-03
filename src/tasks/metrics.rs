use std::sync::Arc;
use tokio::time::Duration;
use device_query::{DeviceQuery, DeviceState};
use crate::menubar::MenuMetrics;
use crate::monitor::calculate_multi_monitor_distance;
use crate::scroll::ScrollTracker;
use crate::app::AppState;
use crate::supabase::SupabaseClient;
use crate::supabase;
use std::collections::HashSet;

pub async fn save_metrics_with_updates(
    state: Arc<AppState>,
    supabase: Option<Arc<SupabaseClient>>
) {
    // Generate a device ID once at startup
    let device_id = get_or_create_device_id();
    log::info!("Starting metrics save loop with device_id: {}", device_id);
    
    let mut last_ui_update = std::time::Instant::now();
    let min_ui_update_interval = std::time::Duration::from_secs(1);
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        let metrics = match tokio::time::timeout(
            tokio::time::Duration::from_secs(1),
            state.metrics.lock()
        ).await {
            Ok(guard) => guard,
            Err(_) => {
                log::warn!("Timeout while acquiring metrics lock");
                continue;
            }
        };

        let metrics_data = metrics.clone();
        drop(metrics);

        if let Ok(_) = state.db.insert_metrics(
            metrics_data.keypresses,
            metrics_data.mouse_clicks,
            metrics_data.mouse_distance_in,
            metrics_data.mouse_distance_mi,
            metrics_data.scroll_steps,
        ).await {
            log::debug!("Successfully saved metrics to local database");
            
            if let Some(supabase_client) = &supabase {
                let supabase_metrics = supabase::Metrics {
                    id: None,
                    created_at: None,
                    keypresses: metrics_data.keypresses,
                    mouse_clicks: metrics_data.mouse_clicks,
                    mouse_distance_in: metrics_data.mouse_distance_in,
                    mouse_distance_mi: metrics_data.mouse_distance_mi,
                    scroll_steps: metrics_data.scroll_steps,
                    device_id: device_id.clone(),
                };

                log::debug!("Attempting to save metrics to Supabase: {:?}", supabase_metrics);
                if let Err(e) = supabase_client.upsert_metrics(&supabase_metrics).await {
                    log::error!("Failed to save metrics to Supabase: {}", e);
                } else {
                    log::debug!("Successfully saved metrics to Supabase");
                }
            } else {
                log::debug!("Supabase client not configured, skipping remote save");
            }

            let now = std::time::Instant::now();
            if now.duration_since(last_ui_update) >= min_ui_update_interval {
                if let Ok(new_total) = state.db.get_total_metrics().await {
                    if let Ok(mut total) = state.total_metrics.try_lock() {
                        *total = new_total.clone();
                        
                        if let Ok(mut menu_bar) = state.menu_bar.try_lock() {
                            let menu_metrics = MenuMetrics::new(
                                new_total.total_keypresses,
                                new_total.total_mouse_clicks,
                                new_total.total_mouse_distance_in,
                                new_total.total_mouse_distance_mi,
                                new_total.total_scroll_steps,
                            );
                            
                            if let Err(e) = menu_bar.update_metrics(&menu_metrics) {
                                log::error!("Failed to update menu metrics: {}", e);
                            }
                        }
                        
                        last_ui_update = now;
                    }
                }
            }

            if let Ok(mut metrics) = state.metrics.try_lock() {
                metrics.reset();
            }
        }
    }
}


fn get_or_create_device_id() -> String {
    let app_dirs = directories::ProjectDirs::from("com", "kweeb-logger", "logger")
        .expect("Failed to get project directories");
    let data_dir = app_dirs.data_dir();
    let device_id_path = data_dir.join("device_id");

    if let Ok(existing_id) = std::fs::read_to_string(&device_id_path) {
        existing_id
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        std::fs::create_dir_all(data_dir).expect("Failed to create data directory");
        std::fs::write(device_id_path, &new_id).expect("Failed to save device ID");
        new_id
    }
}

pub async fn collect_metrics(state: Arc<AppState>) {
    let device_state = DeviceState::new();
    let mut last_mouse = device_state.get_mouse();
    let mut last_keys = device_state.get_keys();
    let mut scroll_tracker = ScrollTracker::new();

    let mut previously_pressed: HashSet<bool> = last_mouse.button_pressed
        .iter()
        .copied()
        .collect();

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let current_mouse = device_state.get_mouse();
        let current_keys = device_state.get_keys();
        let scroll_delta = scroll_tracker.get_scroll_delta();

        let distance = calculate_multi_monitor_distance(
            last_mouse.coords.0,
            last_mouse.coords.1,
            current_mouse.coords.0,
            current_mouse.coords.1,
            &state.monitors.lock().await,
        ).unwrap_or(0.0);

        let mut click_count = 0;
        for (prev, curr) in last_mouse.button_pressed.iter().zip(current_mouse.button_pressed.iter()) {
            if !prev && *curr {
                click_count += 1;
            }
        }

        if let Ok(mut metrics) = state.metrics.try_lock() {
            metrics.keypresses += current_keys.iter()
                .filter(|k| !last_keys.contains(k))
                .count() as i32;

            metrics.mouse_clicks += click_count;
            metrics.mouse_distance_in += distance;
            metrics.mouse_distance_mi += distance / 63360.0;
            metrics.scroll_steps += scroll_delta;
        }

        if let Ok(mut total) = state.total_metrics.try_lock() {
            total.total_keypresses += current_keys.iter()
                .filter(|k| !last_keys.contains(k))
                .count() as i32;

            total.total_mouse_clicks += click_count;
            total.total_scroll_steps += scroll_delta;
        }

        last_mouse = current_mouse;
        last_keys = current_keys;
    }
}