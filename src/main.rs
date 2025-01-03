use std::sync::Arc;
use std::env;
use tokio::runtime::Runtime;
use anyhow::Result;
use dotenv::dotenv;

mod app;
mod config;
mod db;
mod logger;
mod metrics;
mod monitor;
mod scroll;
mod supabase;
mod menubar;
mod tasks;

use crate::app::AppState;
use crate::config::Config;
use crate::tasks::metrics::{collect_metrics, save_metrics_with_updates};
use crate::tasks::monitor::refresh_monitors_periodically;
use crate::supabase::SupabaseClient;


fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    log::info!("Starting keyboard logger...");

    let config = Config::load()?;
    let rt = Runtime::new()?;

    log::info!("SUPABASE_URL: {}", env::var("SUPABASE_URL").unwrap_or_else(|_| "not set".to_string()));
    log::info!("SUPABASE_ANON_KEY: {}", env::var("SUPABASE_ANON_KEY").map(|k| "is set".to_string()).unwrap_or_else(|_| "not set".to_string()));

    let state = rt.block_on(AppState::initialize())?;

    let supabase = if config.has_supabase_config() {
        Some(Arc::new(SupabaseClient::new(
            config.supabase.url.as_ref().unwrap(),
            config.supabase.api_key.as_ref().unwrap(),
        )?))
    } else {
        log::warn!("Supabase configuration not found, skipping...");
        None
    };


    rt.spawn(collect_metrics(Arc::clone(&state)));
    rt.spawn(save_metrics_with_updates(
        Arc::clone(&state),
        supabase.clone(),
    ));
    rt.spawn(refresh_monitors_periodically(Arc::clone(&state)));

    rt.block_on(async {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    Ok(())
}
