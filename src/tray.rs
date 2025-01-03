use anyhow::Result;
use std::sync::Arc;
use std::path::PathBuf;
use directories::ProjectDirs;
use tokio::time::Duration;
use tray_item::TrayItem;
use crate::AppState;

fn get_icon_path() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "kweeb-logger", "logger") {
        let data_dir = proj_dirs.data_dir();
        let icon_path = data_dir.join("keyboard.png");
        log::info!("Tray icon path: {:?}", icon_path);
        if !icon_path.exists() {
            std::fs::create_dir_all(data_dir).ok()?;
            std::fs::write(&icon_path, include_bytes!("../assets/icon.png")).ok()?;
        }
        println!("Icon path: {:?}", icon_path);
        Some(icon_path)
    } else {
        None
    }
}

pub fn setup_tray(tray: &mut TrayItem, state: Arc<AppState>) -> Result<()> {
    if let Some(icon_path) = get_icon_path() {
        if let Err(e) = tray.set_icon(icon_path.to_str().unwrap_or("")) {
            log::error!("Failed to set tray icon: {}", e);
        } else {
            log::info!("Tray icon set: {:?}", icon_path);
        }
    } else {
        log::error!("Tray icon path not resolved");
    }

    // Add menu items
    tray.add_menu_item("Kweeb Logger", Box::new(|| {
        log::info!("Kweeb Logger menu item clicked");
    }))
    .expect("Failed to add 'Kweeb Logger' menu item");

    tray.add_menu_item("Quit", Box::new(move || {
        log::info!("Quit menu item clicked");
        std::process::exit(0);
    }))
    .expect("Failed to add 'Quit' menu item");

    Ok(())
}
