use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::time::Duration;
use std::thread;
use serde::Serialize;
use anyhow::{Result, Context};

const MAX_RETRIES: u32 = 20;
const RETRY_DELAY: Duration = Duration::from_millis(250);
const SOCKET_PATH: &str = "/tmp/kawaiilogger.sock";

#[derive(Debug, Serialize)]
pub struct MenuMetrics {
    pub keypresses: i32,
    pub mouse_clicks: i32,
    pub mouse_distance_in: f64,
    pub mouse_distance_mi: f64,
    pub scroll_steps: i32,
}

impl MenuMetrics {
    pub fn new(
        keypresses: i32,
        mouse_clicks: i32,
        mouse_distance_in: f64,
        mouse_distance_mi: f64,
        scroll_steps: i32,
    ) -> Self {
        Self {
            keypresses,
            mouse_clicks,
            mouse_distance_in,
            mouse_distance_mi,
            scroll_steps,
        }
    }
}

pub struct MenuBar {
    socket: UnixStream,
    go_process: std::process::Child,
}

impl MenuBar {
    pub fn new() -> Result<Self> {
        println!("Starting Go menubar process...");
        
        // Get current directory
        let current_dir = std::env::current_dir()?;
        println!("Current directory: {}", current_dir.display());
        
        // Use the correct binary name
        let menubar_path = current_dir.join("menubar-app");
        println!("Looking for menubar at: {}", menubar_path.display());
        
        // Start the Go process with explicit path
        let go_process = Command::new(menubar_path)
            .spawn()
            .context("Failed to start menubar process")?;

        println!("Go process started with PID: {}", go_process.id());

        // Try to connect with retries
        let socket = Self::connect_with_retry()?;

        Ok(MenuBar {
            socket,
            go_process,
        })
    }


    fn connect_with_retry() -> Result<UnixStream> {
        for i in 0..MAX_RETRIES {
            println!("Attempting to connect to socket (attempt {}/{})", i + 1, MAX_RETRIES);
            
            match UnixStream::connect(SOCKET_PATH) {
                Ok(socket) => {
                    println!("Successfully connected to menubar socket");
                    return Ok(socket);
                }
                Err(e) => {
                    println!("Connection attempt {} failed: {}", i + 1, e);
                    if i == MAX_RETRIES - 1 {
                        return Err(e).context("Failed to connect to menubar socket after maximum retries");
                    }
                    thread::sleep(RETRY_DELAY);
                }
            }
        }
        unreachable!()
    }

    pub fn update_metrics(&mut self, metrics: &MenuMetrics) -> Result<()> {
        let json = serde_json::to_string(metrics)?;
        println!("Sending metrics update: {}", json);
        self.socket.write_all(json.as_bytes())?;
        Ok(())
    }
}

impl Drop for MenuBar {
    fn drop(&mut self) {
        println!("Cleaning up MenuBar...");
        if let Err(e) = self.go_process.kill() {
            println!("Error killing Go process: {}", e);
        }
    }
}