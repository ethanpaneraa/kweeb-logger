use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;
use device_query::{DeviceState, DeviceQuery};
use objc::class;
use objc::{msg_send, sel, sel_impl};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use cocoa::appkit::{
    NSStatusBar, NSMenuItem, NSApplication, 
    NSApplicationActivationPolicy
};
use cocoa::base::{id, nil, YES};
use tokio::sync::mpsc;
use std::thread;
use objc::rc::autoreleasepool;

mod config;
mod db;
mod metrics;
mod monitor;
mod scroll;
mod logger;

use crate::{
    db::Database,
    metrics::{Metrics, TotalMetrics},
    monitor::{get_monitors, Monitor, calculate_distance},
};

pub struct AppState {
    metrics: Mutex<Metrics>,
    total_metrics: Mutex<TotalMetrics>,
    monitors: Mutex<Vec<Monitor>>,
    last_mouse_pos: Mutex<(i32, i32)>,
    db: Arc<Database>,
}

struct StatusBar {
    status_item: ObjcId,
    menu: ObjcId,
    button: ObjcId, 
    keystroke_item: ObjcId,
    clicks_item: ObjcId,
    distance_item: ObjcId,
    scroll_item: ObjcId,
    update_sender: mpsc::UnboundedSender<StatusBarMessage>,
}


#[derive(Clone)]
struct StatusBarMessage {
    total_keypresses: i32,
    total_mouse_clicks: i32,
    total_mouse_distance_in: f64,
    total_mouse_distance_mi: f64,
    total_scroll_steps: i32,
}

struct ObjcId(*mut objc::runtime::Object);
unsafe impl Send for ObjcId {}
unsafe impl Sync for ObjcId {}

impl StatusBar {
    unsafe fn new() -> (Arc<Self>, mpsc::UnboundedReceiver<StatusBarMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        
        autoreleasepool(|| {
            println!("Creating system status bar...");
            let status_bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            
            println!("Creating status item...");
            let status_item: id = msg_send![status_bar, statusItemWithLength:-1.0];
            
            let _: () = msg_send![status_item, retain];
            
            println!("Getting button...");
            let button: id = msg_send![status_item, button];
            let _: () = msg_send![button, retain];
            
            println!("Setting button properties...");
            let title = NSString::alloc(nil).init_str("📊");
            let _: () = msg_send![button, setTitle:title];
            
            let _: () = msg_send![button, setEnabled:YES];

            let _: () = msg_send![status_item, setVisible:YES];
            
            println!("Creating menu...");
            let menu: id = msg_send![class!(NSMenu), new];
            let _: () = msg_send![menu, retain];
            
            // Create menu items
            println!("Adding menu items...");
            let keystroke_item = Self::create_menu_item("Keypresses: 0");
            let _: () = msg_send![menu, addItem:keystroke_item];
            
            let clicks_item = Self::create_menu_item("Mouse Clicks: 0");
            let _: () = msg_send![menu, addItem:clicks_item];
            
            let distance_item = Self::create_menu_item("Mouse Travel: 0 in / 0 mi");
            let _: () = msg_send![menu, addItem:distance_item];
            
            let scroll_item = Self::create_menu_item("Scroll Steps: 0");
            let _: () = msg_send![menu, addItem:scroll_item];
            
            // Add separator
            let separator: id = msg_send![class!(NSMenuItem), separatorItem];
            let _: () = msg_send![menu, addItem:separator];
            
            // Add quit item with action
            println!("Adding quit item...");
            let quit_title = NSString::alloc(nil).init_str("Quit");
            let quit_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(
                    quit_title,
                    sel!(terminate:),
                    NSString::alloc(nil).init_str("q")
                );
            let _: () = msg_send![quit_item, setTarget:class!(NSApplication)];
            let _: () = msg_send![menu, addItem:quit_item];
            
            // Set the menu
            println!("Setting menu...");
            let _: () = msg_send![status_item, setMenu:menu];
            
            println!("Status bar setup complete");
            
            let status_bar = Arc::new(StatusBar {
                status_item: ObjcId(status_item),
                menu: ObjcId(menu),
                button: ObjcId(button),
                keystroke_item: ObjcId(keystroke_item),
                clicks_item: ObjcId(clicks_item),
                distance_item: ObjcId(distance_item),
                scroll_item: ObjcId(scroll_item),
                update_sender: tx,
            });
            
            (status_bar, rx)
        })
    }

    fn send_update(&self, metrics: &TotalMetrics) {
        let msg = StatusBarMessage {
            total_keypresses: metrics.total_keypresses,
            total_mouse_clicks: metrics.total_mouse_clicks,
            total_mouse_distance_in: metrics.total_mouse_distance_in,
            total_mouse_distance_mi: metrics.total_mouse_distance_mi,
            total_scroll_steps: metrics.total_scroll_steps,
        };
        
        let _ = self.update_sender.send(msg);
    }

    unsafe fn create_menu_item(title: &str) -> id {
        autoreleasepool(|| {
            let title = NSString::alloc(nil).init_str(title);
            NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                title,
                sel!(doNothing:),
                NSString::alloc(nil).init_str("")
            )
        })
    }

    unsafe fn update_menu_items(&self, msg: &StatusBarMessage) {
        autoreleasepool(|| {
            let keystroke_text = NSString::alloc(nil)
                .init_str(&format!("Keypresses: {}", msg.total_keypresses));
            let _: () = msg_send![self.keystroke_item.0, setTitle:keystroke_text];

            let clicks_text = NSString::alloc(nil)
                .init_str(&format!("Mouse Clicks: {}", msg.total_mouse_clicks));
            let _: () = msg_send![self.clicks_item.0, setTitle:clicks_text];

            let distance_text = NSString::alloc(nil)
                .init_str(&format!(
                    "Mouse Travel: {:.1} in / {:.2} mi",
                    msg.total_mouse_distance_in,
                    msg.total_mouse_distance_mi
                ));
            let _: () = msg_send![self.distance_item.0, setTitle:distance_text];

            let scroll_text = NSString::alloc(nil)
                .init_str(&format!("Scroll Steps: {}", msg.total_scroll_steps));
            let _: () = msg_send![self.scroll_item.0, setTitle:scroll_text];
        });
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    println!("Starting application...");
    
    let rt = Runtime::new().unwrap();
    
    let state = rt.block_on(async {
        println!("Initializing database and state...");
        let db = Arc::new(Database::new().await?);
        let total_metrics = db.get_total_metrics().await?;
        
        let app_state = Arc::new(AppState {
            metrics: Mutex::new(Metrics::default()),
            total_metrics: Mutex::new(total_metrics),
            monitors: Mutex::new(get_monitors()?),
            last_mouse_pos: Mutex::new((0, 0)),
            db,
        });
        
        Ok::<_, anyhow::Error>(app_state)
    })?;
    
    println!("State initialized");
    
    unsafe {
        autoreleasepool(|| {
            println!("Creating autorelease pool and application...");
            let app = NSApplication::sharedApplication(nil);
            
            // Set up application properties
            app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
            
            // Create and retain the application delegate
            let _: () = msg_send![app, setDelegate:app];
            
            // Initialize the application
            let _: () = msg_send![app, finishLaunching];
            
            // Create our status bar
            println!("Creating status bar...");
            let (status_bar, mut rx) = StatusBar::new();
            
            // Handle UI updates in the main thread
            let status_bar_ref = Arc::clone(&status_bar);
            thread::spawn(move || {
                println!("UI update thread started");
                while let Some(msg) = rx.blocking_recv() {
                    unsafe {
                        status_bar_ref.update_menu_items(&msg);
                    }
                }
            });
            
            // Start the metrics collection
            let metrics_state = Arc::clone(&state);
            rt.spawn(async move {
                println!("Starting metrics collection...");
                collect_metrics(metrics_state).await;
            });
            
            // Start the metrics saving
            let save_state = Arc::clone(&state);
            let save_status_bar = status_bar;
            rt.spawn(async move {
                println!("Starting metrics saving...");
                save_metrics_with_updates(save_state, save_status_bar).await;
            });
            
            // Start the monitor refresh
            let monitor_state = Arc::clone(&state);
            rt.spawn(async move {
                println!("Starting monitor refresh...");
                refresh_monitors_periodically(monitor_state).await;
            });
            
            println!("Starting run loop...");
            let _: () = msg_send![app, activateIgnoringOtherApps:YES];
            app.run();
        })
    }
    
    Ok(())
}


async fn save_metrics_with_updates(state: Arc<AppState>, status_bar: Arc<StatusBar>) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        let mut metrics = state.metrics.lock().await;

        log::debug!(
            "keypresses: {}, mouse_clicks: {}, mouse_distance_in: {}, mouse_distance_mi: {}, scroll_steps: {}",
            metrics.keypresses,
            metrics.mouse_clicks,
            metrics.mouse_distance_in,
            metrics.mouse_distance_mi,
            metrics.scroll_steps,
        );

        match state.db
            .insert_metrics(
                metrics.keypresses,
                metrics.mouse_clicks,
                metrics.mouse_distance_in,
                metrics.mouse_distance_mi,
                metrics.scroll_steps,
            )
            .await
        {
            Ok(_) => {
                if let Ok(new_total) = state.db.get_total_metrics().await {
                    if let Ok(mut total) = state.total_metrics.try_lock() {
                        *total = new_total;
                        status_bar.send_update(&*total);
                    }
                }
                metrics.reset();
            }
            Err(e) => {
                log::error!("Failed to save metrics: {}", e);
            }
        }
    }
}

// Your existing metrics collection functions remain largely the same
async fn collect_metrics(state: Arc<AppState>) {
    let device_state = DeviceState::new();
    let mut last_mouse = device_state.get_mouse();
    let mut last_keys = device_state.get_keys();
    
    loop {
        let current_mouse = device_state.get_mouse();
        let current_keys = device_state.get_keys();
        
        let new_keys: Vec<_> = current_keys.iter()
            .filter(|k| !last_keys.contains(k))
            .collect();
        
        // Calculate mouse movement first to avoid multiple calculations
        let distance = calculate_distance(
            last_mouse.coords.0, last_mouse.coords.1,
            current_mouse.coords.0, current_mouse.coords.1
        );

        // Update current metrics
        {
            let mut metrics = state.metrics.lock().await;
            metrics.keypresses += new_keys.len() as i32;
            
            if current_mouse.button_pressed.len() > last_mouse.button_pressed.len() {
                metrics.mouse_clicks += 1;
            }
            
            metrics.mouse_distance_in += distance;
            metrics.mouse_distance_mi += distance / 63360.0;
        }
        
        // Update total metrics in a separate block
        {
            let mut total = state.total_metrics.lock().await;
            total.total_keypresses += new_keys.len() as i32;
            if current_mouse.button_pressed.len() > last_mouse.button_pressed.len() {
                total.total_mouse_clicks += 1;
            }
            total.total_mouse_distance_in += distance;
            total.total_mouse_distance_mi += distance / 63360.0;
        }
        
        // Update last mouse position
        *state.last_mouse_pos.lock().await = current_mouse.coords;
        
        last_mouse = current_mouse;
        last_keys = current_keys;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

async fn save_metrics_periodically(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let mut metrics = state.metrics.lock().await;

        log::debug!(
            "keypresses: {}, mouse_clicks: {}, mouse_distance_in: {}, mouse_distance_mi: {}, scroll_steps: {}",
            metrics.keypresses,
            metrics.mouse_clicks,
            metrics.mouse_distance_in,
            metrics.mouse_distance_mi,
            metrics.scroll_steps,
        );

        match state.db
            .insert_metrics(
                metrics.keypresses,
                metrics.mouse_clicks,
                metrics.mouse_distance_in,
                metrics.mouse_distance_mi,
                metrics.scroll_steps,
            )
            .await
        {
            Ok(_) => {
                if let Ok(new_total) = state.db.get_total_metrics().await {
                    if let Ok(mut total) = state.total_metrics.try_lock() {
                        *total = new_total;
                    }
                }
                metrics.reset();
            }
            Err(e) => {
                log::error!("Failed to save metrics: {}", e);
            }
        }
    }
}

async fn refresh_monitors_periodically(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        if let Ok(new_monitors) = get_monitors() {
            *state.monitors.lock().await = new_monitors;
        }
    }
}
