use anyhow::Result;
use std::sync::Arc;
use tokio::time::Duration;
use tray_item::TrayItem;
use crate::AppState;

pub fn setup_tray(tray: &mut TrayItem, state: Arc<AppState>) -> Result<()> {
    tray.add_menu_item("Kweeb Logger", Box::new(|| ()))
        .expect("Failed to add menu item");
    tray.add_menu_item("", Box::new(|| ()))
        .expect("Failed to add separator");
    tray.add_menu_item("Quit", Box::new(|| {
        println!("Quitting...");
        std::process::exit(0);
    }))
    .expect("Failed to add quit menu item");

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