use std::sync::Arc;
use tokio::time::{self, Duration};

use crate::{app::AppState, monitor::get_monitors};

pub async fn refresh_monitors_periodically(state: Arc<AppState>) {
    loop {
        time::sleep(Duration::from_secs(30)).await;
        if let Ok(new_monitors) = get_monitors() {
            *state.monitors.lock().await = new_monitors;
        }
    }
}