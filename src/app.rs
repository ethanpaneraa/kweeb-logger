use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    db::Database,
    menubar::MenuBar,
    metrics::{Metrics, TotalMetrics},
    monitor::get_monitors,
    monitor::Monitor,
};

pub struct AppState {
    pub metrics: Mutex<Metrics>,
    pub total_metrics: Mutex<TotalMetrics>,
    pub monitors: Mutex<Vec<Monitor>>,
    pub db: Arc<Database>,
    pub menu_bar: Arc<Mutex<MenuBar>>,
}

impl AppState {
    pub async fn initialize() -> anyhow::Result<Arc<Self>> {
        let db = Arc::new(Database::new().await?);
        let total_metrics = db.get_total_metrics().await?;
        let menu_bar = MenuBar::new()?;
        let monitors = get_monitors()?;

        Ok(Arc::new(Self {
            metrics: Mutex::new(Metrics::default()),
            total_metrics: Mutex::new(total_metrics),
            monitors: Mutex::new(monitors),
            db,
            menu_bar: Arc::new(Mutex::new(menu_bar)),
        }))
    }
}