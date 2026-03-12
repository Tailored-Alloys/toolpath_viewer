//! Toolpath Viewer - A high-performance toolpath visualization application
//!
//! This application uses Clean Architecture with the following layers:
//! - Domain: Core business entities and logic
//! - Application: Use cases and application services
//! - Infrastructure: External dependencies (file I/O, OpenGL rendering)
//! - Presentation: User interface and event handling

mod domain;
mod application;
mod infrastructure;
mod presentation;

use anyhow::Result;
use log::info;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("Starting Toolpath Viewer");

    // Create and run the application
    let app = presentation::App::new()?;
    app.run()
}
