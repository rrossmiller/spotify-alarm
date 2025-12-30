mod alarm;
mod auth;
mod spotify;
mod state;
mod web;

use librespot::core::Error;
use log::LevelFilter;
use state::{AppState, SharedState};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::builder()
        .filter_module("librespot", LevelFilter::Info)
        .init();

    // Handle hash-password command
    if let Some(cmd) = env::args().nth(1) {
        if cmd == "hash-password" {
            return handle_hash_password();
        }
    }

    // Get config file path from command line args or use default
    let config_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "alarms.json".to_string());

    // Load alarm configuration
    let config = match alarm::AlarmConfig::load(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading alarm config from '{}': {}", config_path, e);
            eprintln!("Please create an alarms.json file with your alarm configuration.");
            eprintln!("Example format:");
            eprintln!(
                r#"{{
  "web": {{
    "enabled": true,
    "bind_addr": "0.0.0.0",
    "port": 8080,
    "password_hash": "run 'cargo run -- hash-password' to generate"
  }},
  "alarms": [
    {{
      "name": "Weekday Morning Alarm",
      "time": "07:00",
      "days": ["Mon", "Tue", "Wed", "Thu", "Fri"],
      "enabled": true
    }}
  ]
}}"#
            );
            std::process::exit(1);
        }
    };

    // Check if web server is enabled and password is configured
    if config.web.enabled && config.web.password_hash.is_none() {
        eprintln!("⚠️  Web server is enabled but no password is configured!");
        eprintln!("Run 'cargo run -- hash-password' to generate a password hash,");
        eprintln!("then add it to alarms.json under web.password_hash");
        std::process::exit(1);
    }

    // Create shared state
    let state: SharedState = Arc::new(RwLock::new(AppState {
        config: config.clone(),
        config_path: PathBuf::from(&config_path),
        // session: session.clone(),
        // spirc: spirc.clone(),
        last_alarm_trigger: None,
    }));

    // Spawn alarm scheduler task
    println!("\n🎵 Spotify Alarm started");
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        loop {
            match alarm::run_scheduler(scheduler_state.clone()).await {
                Ok(_) => {
                    // Scheduler should never normally exit, but if it does, restart it
                    eprintln!(
                        "⚠️  Alarm scheduler exited unexpectedly, restarting in 5 seconds..."
                    );
                }
                Err(e) => {
                    eprintln!("❌ Scheduler error: {}", e);
                    eprintln!("   Restarting scheduler in 5 seconds...");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    // Run web server if enabled
    if config.web.enabled {
        let bind_addr = format!("{}:{}", config.web.bind_addr, config.web.port);
        web::run_server(state, &bind_addr)
            .await
            .map_err(|e| Error::unavailable(e.to_string()))?;
    } else {
        println!("ℹ️  Web interface is disabled. Press Ctrl+C to stop.");
        // Keep running if web is disabled
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| Error::unavailable(e.to_string()))?;
    }

    Ok(())
}

fn handle_hash_password() -> Result<(), Error> {
    println!("🔐 Password Hash Generator");
    println!();

    print!("Enter password: ");
    io::stdout().flush().unwrap();

    let mut password = String::new();
    io::stdin()
        .read_line(&mut password)
        .map_err(|e| Error::unavailable(e.to_string()))?;

    let password = password.trim();

    if password.is_empty() {
        eprintln!("Error: Password cannot be empty");
        std::process::exit(1);
    }

    match auth::hash_password(password) {
        Ok(hash) => {
            println!();
            println!("✓ Password hashed successfully!");
            println!();
            println!("Add this to your alarms.json under web.password_hash:");
            println!("{}", hash);
            println!();
            Ok(())
        }
        Err(e) => {
            eprintln!("Error hashing password: {}", e);
            std::process::exit(1);
        }
    }
}
