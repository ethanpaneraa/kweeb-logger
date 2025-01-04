use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, ACCEPT};
use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Metrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,  
    #[serde(skip_serializing_if = "Option::is_none")]  
    pub created_at: Option<String>,
    pub keypresses: i32,
    pub mouse_clicks: i32,
    pub mouse_distance_in: f64,
    pub mouse_distance_mi: f64,
    pub scroll_steps: i32,
    pub device_id: String,
}

pub struct SupabaseClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl SupabaseClient {
    pub fn initialize_supabase() -> Result<Option<Arc<SupabaseClient>>> {
        let supabase_url = env::var("SUPABASE_URL")
            .context("SUPABASE_URL not set")?;
        let supabase_key = env::var("SUPABASE_ANON_KEY")
            .context("SUPABASE_ANON_KEY not set")?;
    
        let supabase = SupabaseClient::new(&supabase_url, &supabase_key)?;
        Ok(Some(Arc::new(supabase)))
    }

    pub fn new(supabase_url: &str, api_key: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "apikey",
            HeaderValue::from_str(api_key)?
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(SupabaseClient {
            client,
            base_url: supabase_url.to_string(),
            api_key: api_key.to_string(),
        })
    }

    pub async fn upsert_metrics(&self, metrics: &Metrics) -> Result<()> {
        let url = format!("{}/rest/v1/rpc/upsert_metrics", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "p_device_id": metrics.device_id,
                "p_keypresses": metrics.keypresses,
                "p_mouse_clicks": metrics.mouse_clicks,
                "p_mouse_distance_in": metrics.mouse_distance_in,
                "p_mouse_distance_mi": metrics.mouse_distance_mi,
                "p_scroll_steps": metrics.scroll_steps
            }))
            .send()
            .await?;
    
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            log::error!("Failed to upsert metrics. Status: {}, Error: {}", 
                status, error_text);
            anyhow::bail!("Supabase request failed: {}", error_text);
        }
    
        Ok(())
    }

    pub async fn update_metrics(&self, device_id: &str, metrics: &Metrics) -> Result<()> {
        let url = format!("{}/rest/v1/kweeb_logger_metrics", self.base_url);
        
        self.client
            .patch(&url)
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .query(&[("device_id", "eq.".to_string() + device_id)])
            .json(&metrics)
            .send()
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_total_metrics(&self, device_id: &str) -> Result<Metrics> {
        let url = format!(
            "{}/rest/v1/rpc/get_total_metrics",
            self.base_url
        );

        let response = self.client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "p_device_id": device_id
            }))
            .send()
            .await?;

        let metrics = response.json::<Metrics>().await?;
        Ok(metrics)
    }
}