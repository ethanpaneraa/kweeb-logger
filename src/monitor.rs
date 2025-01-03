use anyhow::{Result, anyhow};
use core_graphics::display::CGDisplay;
use std::error::Error;
use std::fmt;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
pub enum MonitorOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
pub struct Monitor {
    pub x_pos: i32,
    pub y_pos: i32,
    pub width_px: i32,
    pub height_px: i32,
    pub ppi: f64,
    pub primary: bool,
    pub display_id: u32,
    pub orientation: MonitorOrientation,
}

#[derive(Debug)]
pub enum MonitorError {
    NoMonitorsFound,
    InvalidCoordinates,
    SystemError(String),
}

// A* pathfinding structures
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Point {
    x: i32,
    y: i32,
    f_score: i32,  
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_score.cmp(&self.f_score)
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Error for MonitorError {}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MonitorError::NoMonitorsFound => write!(f, "No monitors found"),
            MonitorError::InvalidCoordinates => write!(f, "Invalid coordinates"),
            MonitorError::SystemError(msg) => write!(f, "System error: {}", msg),
        }
    }
}

impl Monitor {
    fn new(display: CGDisplay) -> Result<Self, MonitorError> {
        let bounds = display.bounds();
        
        let width = bounds.size.width as i32;
        let height = bounds.size.height as i32;
        let orientation = if height > width {
            MonitorOrientation::Vertical
        } else {
            MonitorOrientation::Horizontal
        };
        
        let scale_factor = display.pixels_high() as f64 / bounds.size.height;
        let ppi = 72.0 * scale_factor;
        
        Ok(Monitor {
            x_pos: bounds.origin.x as i32,
            y_pos: bounds.origin.y as i32,
            width_px: width,
            height_px: height,
            ppi,
            primary: display.is_main(),
            display_id: display.unit_number(),
            orientation,
        })
    }

    pub fn get_edge_points(&self) -> Vec<(i32, i32)> {
        let mut points = Vec::new();
        
        points.push((self.x_pos, self.y_pos));
        points.push((self.x_pos + self.width_px, self.y_pos));
        points.push((self.x_pos, self.y_pos + self.height_px));
        points.push((self.x_pos + self.width_px, self.y_pos + self.height_px));
        
        points.push((self.x_pos + self.width_px / 2, self.y_pos));
        points.push((self.x_pos + self.width_px / 2, self.y_pos + self.height_px));
        points.push((self.x_pos, self.y_pos + self.height_px / 2));
        points.push((self.x_pos + self.width_px, self.y_pos + self.height_px / 2));
        
        points
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x_pos 
            && x < self.x_pos + self.width_px 
            && y >= self.y_pos 
            && y < self.y_pos + self.height_px
    }
}

fn manhattan_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    (x2 - x1).abs() + (y2 - y1).abs()
}

fn are_monitors_adjacent(m1: &Monitor, m2: &Monitor) -> bool {
    const ALIGNMENT_TOLERANCE: i32 = 5; 

    let x_overlap = m1.x_pos - ALIGNMENT_TOLERANCE < m2.x_pos + m2.width_px 
        && m2.x_pos - ALIGNMENT_TOLERANCE < m1.x_pos + m1.width_px;
    let y_overlap = m1.y_pos - ALIGNMENT_TOLERANCE < m2.y_pos + m2.height_px 
        && m2.y_pos - ALIGNMENT_TOLERANCE < m1.y_pos + m1.height_px;
    
    (x_overlap && (
        (m1.y_pos - m2.y_pos - m2.height_px).abs() <= ALIGNMENT_TOLERANCE ||
        (m2.y_pos - m1.y_pos - m1.height_px).abs() <= ALIGNMENT_TOLERANCE
    )) ||
    (y_overlap && (
        (m1.x_pos - m2.x_pos - m2.width_px).abs() <= ALIGNMENT_TOLERANCE ||
        (m2.x_pos - m1.x_pos - m1.width_px).abs() <= ALIGNMENT_TOLERANCE
    ))
}

fn find_optimal_path(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    monitors: &[Monitor],
) -> Vec<(i32, i32)> {
    let mut open_set = BinaryHeap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    
    open_set.push(Point {
        x: start_x,
        y: start_y,
        f_score: manhattan_distance(start_x, start_y, end_x, end_y),
    });
    g_score.insert((start_x, start_y), 0);

    let mut transition_points = Vec::new();
    for m in monitors {
        transition_points.extend(m.get_edge_points());
    }
    
    while let Some(current) = open_set.pop() {
        if current.x == end_x && current.y == end_y {
            return reconstruct_path(&came_from, (end_x, end_y));
        }

        let current_g = *g_score.get(&(current.x, current.y)).unwrap_or(&i32::MAX);

        // Generate neighbors (transition points that are visible from current point)
        for &(next_x, next_y) in &transition_points {
            if is_valid_movement(current.x, current.y, next_x, next_y, monitors) {
                let tentative_g = current_g + manhattan_distance(current.x, current.y, next_x, next_y);
                
                if tentative_g < *g_score.get(&(next_x, next_y)).unwrap_or(&i32::MAX) {
                    came_from.insert((next_x, next_y), (current.x, current.y));
                    g_score.insert((next_x, next_y), tentative_g);
                    
                    open_set.push(Point {
                        x: next_x,
                        y: next_y,
                        f_score: tentative_g + manhattan_distance(next_x, next_y, end_x, end_y),
                    });
                }
            }
        }
    }

    vec![(start_x, start_y), (end_x, end_y)]
}

fn is_valid_movement(x1: i32, y1: i32, x2: i32, y2: i32, monitors: &[Monitor]) -> bool {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let steps = std::cmp::max(dx.abs(), dy.abs()) as f64;
    
    if steps == 0.0 {
        return true;
    }

    let x_step = dx as f64 / steps;
    let y_step = dy as f64 / steps;

    for i in 0..=steps as i32 {
        let x = x1 as f64 + (x_step * i as f64);
        let y = y1 as f64 + (y_step * i as f64);
        if !monitors.iter().any(|m| m.contains_point(x as i32, y as i32)) {
            return false;
        }
    }

    true
}

fn reconstruct_path(came_from: &HashMap<(i32, i32), (i32, i32)>, end: (i32, i32)) -> Vec<(i32, i32)> {
    let mut path = vec![end];
    let mut current = end;

    while let Some(&prev) = came_from.get(&current) {
        path.push(prev);
        current = prev;
    }

    path.reverse();
    path
}

pub fn calculate_multi_monitor_distance(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    monitors: &[Monitor],
) -> Result<f64, MonitorError> {
    if monitors.is_empty() {
        return Err(MonitorError::NoMonitorsFound);
    }

    let m1 = get_monitor_for_coordinates(x1, y1, monitors)?;
    let m2 = get_monitor_for_coordinates(x2, y2, monitors)?;

    if std::ptr::eq(m1, m2) {
        return Ok(calculate_distance(x1, y1, x2, y2) / m1.ppi);
    }

    let path = find_optimal_path(x1, y1, x2, y2, monitors);
    let mut total_distance = 0.0;

    for i in 0..path.len() - 1 {
        let (px1, py1) = path[i];
        let (px2, py2) = path[i + 1];
        
        if let Ok(monitor) = get_monitor_for_coordinates(px1, py1, monitors) {
            total_distance += calculate_distance(px1, py1, px2, py2) / monitor.ppi;
        }
    }

    Ok(total_distance)
}

pub fn get_monitors() -> Result<Vec<Monitor>> {
    let displays = CGDisplay::active_displays()
        .map_err(|e| anyhow!("Failed to get displays: {:?}", e))?;
        
    if displays.is_empty() {
        return Err(MonitorError::NoMonitorsFound.into());
    }

    let mut monitors = Vec::new();
    for display_id in displays {
        let display = CGDisplay::new(display_id);
        if let Ok(monitor) = Monitor::new(display) {
            monitors.push(monitor);
        }
    }

    if monitors.is_empty() {
        return Err(MonitorError::NoMonitorsFound.into());
    }

    Ok(monitors)
}

pub fn get_monitor_for_coordinates(x: i32, y: i32, monitors: &[Monitor]) -> Result<&Monitor, MonitorError> {
    if monitors.is_empty() {
        return Err(MonitorError::NoMonitorsFound);
    }

    monitors
        .iter()
        .find(|m| {
            x >= m.x_pos
                && x < (m.x_pos + m.width_px)
                && y >= m.y_pos
                && y < (m.y_pos + m.height_px)
        })
        .ok_or(MonitorError::InvalidCoordinates)
}

pub fn calculate_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    (dx * dx + dy * dy).sqrt()
}