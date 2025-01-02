use crate::AppState;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::Duration;
use tray_item::TrayItem;

pub struct TrayMenu {
    tray: TrayItem,
    state: Arc<AppState>,
}

impl TrayMenu {
    pub fn new(state: Arc<AppState>) -> Result<Self> {
        let mut tray = TrayItem::new("kweeb-logger", "kweeb-logger-tray")?;
        
        if let Ok(icon_data) = std::fs::read("./keyboard.ico") {
            tray.set_icon_from_buffer(&icon_data)?;
        }

        Ok(Self { tray, state })
    }

    pub fn setup(&mut self) -> Result<()> {
        let keypresses = self.tray.add_menu_item("Keypresses: 0", || {})?;
        let mouse_clicks = self.tray.add_menu_item("Mouse Clicks: 0", || {})?;
        let mouse_distance = self.tray.add_menu_item("Mouse Travel: 0.0", || {})?;
        let scroll_steps = self.tray.add_menu_item("Scroll Steps: 0", || {})?;

        self.tray.add_separator()?;
        
        self.tray.add_menu_item("Quit", move || {
            std::process::exit(0);
        })?;

        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                
                let total = state.total_metrics.lock().await;
                let _ = keypresses.set_title(&format!(
                    "Keypresses: {}", 
                    total.total_keypresses
                ));
                let _ = mouse_clicks.set_title(&format!(
                    "Mouse Clicks: {}", 
                    total.total_mouse_clicks
                ));
                let _ = mouse_distance.set_title(&format!(
                    "Mouse Travel (in) {:.2} / (mi) {:.2}", 
                    total.total_mouse_distance_in,
                    total.total_mouse_distance_mi
                ));
                let _ = scroll_steps.set_title(&format!(
                    "Scroll Steps: {}", 
                    total.total_scroll_steps
                ));
            }
        });

        Ok(())
    }
}