#[derive(Default)]
pub struct Metrics {
    pub keypresses: i32,
    pub mouse_clicks: i32,
    pub mouse_distance_in: f64,
    pub mouse_distance_mi: f64,
    pub scroll_steps: i32,
}

impl Metrics {
    pub fn reset(&mut self) {
        self.keypresses = 0;
        self.mouse_clicks = 0;
        self.mouse_distance_in = 0.0;
        self.mouse_distance_mi = 0.0;
        self.scroll_steps = 0;
    }
}

#[derive(Default)]
pub struct TotalMetrics {
    pub total_keypresses: i32,
    pub total_mouse_clicks: i32,
    pub total_mouse_distance_in: f64,
    pub total_mouse_distance_mi: f64,
    pub total_scroll_steps: i32,
}