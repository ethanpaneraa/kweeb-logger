#[derive(Debug, Clone)]
pub struct Monitor {
    pub x_pos: i32,
    pub y_pos: i32,
    pub width_px: i32,
    pub height_px: i32,
    pub width_in: i32,
    pub height_in: i32,
    pub ppi: i32,
}

impl Monitor {
    pub fn get_side_coordinates(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> (i32, i32) {
        if x2 < self.x_pos {
            (self.x_pos, y1)
        } else if x2 >= self.x_pos + self.width_px {
            (self.x_pos + self.width_px - 1, y1)
        } else if y2 < self.y_pos {
            (x1, self.y_pos)
        } else if y2 >= self.y_pos + self.height_px {
            (x1, self.y_pos + self.height_px - 1)
        } else {
            (x2, y2)
        }
    }
}

pub fn get_monitor_for_coordinates(x: i32, y: i32, monitors: &[Monitor]) -> &Monitor {
    monitors
        .iter()
        .find(|m| {
            x >= m.x_pos
                && x < (m.x_pos + m.width_px)
                && y >= m.y_pos
                && y < (m.y_pos + m.height_px)
        })
        .unwrap_or(&monitors[0])
}

pub fn calculate_multi_monitor_distance(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    monitors: &[Monitor],
) -> f64 {
    let m1 = get_monitor_for_coordinates(x1, y1, monitors);
    let m2 = get_monitor_for_coordinates(x2, y2, monitors);

    if std::ptr::eq(m1, m2) {
        return calculate_distance(x1, y1, x2, y2) / m1.ppi as f64;
    }

    let (sx1, sy1) = m1.get_side_coordinates(x1, y1, x2, y2);
    let d1 = calculate_distance(x1, y1, sx1, sy1) / m1.ppi as f64;

    let (sx2, sy2) = m2.get_side_coordinates(x1, y1, x2, y2);
    let d2 = calculate_distance(sx1, sy1, sx2, sy2) / m2.ppi as f64;

    d1 + d2
}

pub fn calculate_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    (dx * dx + dy * dy).sqrt()
}