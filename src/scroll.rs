use device_query::{DeviceQuery, DeviceState};

pub struct ScrollTracker {
    device_state: DeviceState,
    last_mouse_y: i32,
}

impl ScrollTracker {
    pub fn new() -> Self {
        let device_state = DeviceState::new();
        let last_mouse_y = device_state.get_mouse().coords.1;
        Self {
            device_state,
            last_mouse_y,
        }
    }

    pub fn get_scroll_delta(&mut self) -> i32 {
        let current_y = self.device_state.get_mouse().coords.1;
        let delta = (current_y - self.last_mouse_y).abs();
        self.last_mouse_y = current_y;
        
        if delta > 15 { 
            1
        } else {
            0  
        }
    }
}