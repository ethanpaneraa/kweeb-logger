use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, ACCEPT};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Metrics {
    pub id: Option<i32>,
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
    pub fn new(supabase_url: &str, api_key: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
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

    pub async fn insert_metrics(&self, metrics: &Metrics) -> Result<()> {
        let url = format!("{}/rest/v1/metrics", self.base_url);
        
        self.client
            .post(&url)
            .header("apikey", &self.api_key)
            .header("Content-Type", "application/json")
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