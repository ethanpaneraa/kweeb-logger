use anyhow::Context;
use directories::ProjectDirs;
use std::fs::OpenOptions;

pub fn setup_logging() -> anyhow::Result<()> {
    let proj_dirs = ProjectDirs::from("com", "kweeb-logger", "logger")
        .context("Failed to get project directories")?;
    
    let log_dir = proj_dirs.data_dir();
    println!("Creating log directory at: {}", log_dir.display());
    std::fs::create_dir_all(&log_dir)?;
    
    let log_file = log_dir.join("kweeb-logger.log");
    println!("Log file will be at: {}", log_file.display());
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)?;

    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    log::info!("Logging initialized at {}", log_file.display());
    Ok(())
}