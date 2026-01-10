use chrono::{Datelike, Local, NaiveTime, Timelike, Weekday};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmConfig {
    pub alarms: Vec<Alarm>,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub spotify: SpotifyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebConfig {
    #[serde(default = "default_web_enabled")]
    pub enabled: bool,
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub password_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyConfig {
    /// ALSA device name (e.g., "default", "hw:0,0", "hw:Headphones")
    /// Use "aplay -L" to list available devices
    #[serde(default = "default_audio_device")]
    pub audio_device: String,
}

impl Default for SpotifyConfig {
    fn default() -> Self {
        Self {
            audio_device: default_audio_device(),
        }
    }
}

fn default_audio_device() -> String {
    "default".to_string()
}

fn default_web_enabled() -> bool {
    false
}

fn default_bind_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alarm {
    /// Alarm name/description
    pub name: String,
    /// Time in 24-hour format (HH:MM)
    pub time: String,
    /// Days of week to play alarm (Mon, Tue, Wed, Thu, Fri, Sat, Sun)
    /// If None or empty, alarm plays every day
    #[serde(default)]
    pub days: Vec<String>,
    /// Whether this alarm is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl Alarm {
    /// Parse the time string (HH:MM) into a NaiveTime
    pub fn parse_time(&self) -> Result<NaiveTime, String> {
        let parts: Vec<&str> = self.time.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid time format: {}", self.time));
        }

        let hour = parts[0]
            .parse::<u32>()
            .map_err(|_| format!("Invalid hour: {}", parts[0]))?;
        let minute = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid minute: {}", parts[1]))?;

        NaiveTime::from_hms_opt(hour, minute, 0)
            .ok_or_else(|| format!("Invalid time: {}:{}", hour, minute))
    }

    /// Check if alarm should play on the given weekday
    fn should_play_on(&self, weekday: Weekday) -> bool {
        if self.days.is_empty() {
            return true; // Play every day if no days specified
        }

        let weekday_str = format!("{:?}", weekday); // "Mon", "Tue", etc.
        self.days.iter().any(|d| {
            d.eq_ignore_ascii_case(&weekday_str) || d.eq_ignore_ascii_case(&weekday_str[..3])
        })
    }
}

impl AlarmConfig {
    /// Load alarm configuration from a JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: AlarmConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save alarm configuration to a JSON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Calculate seconds until the next occurrence of the given time
#[allow(dead_code)]
fn seconds_until_time(target_time: NaiveTime) -> u64 {
    let now = Local::now();
    let today = now.date_naive();
    let target_today = today.and_time(target_time);

    // If target time has passed today, schedule for tomorrow
    let target_datetime = if now.naive_local() >= target_today {
        (today + chrono::Days::new(1)).and_time(target_time)
    } else {
        target_today
    };

    let duration = target_datetime
        .and_local_timezone(Local)
        .unwrap()
        .signed_duration_since(now);
    duration.num_seconds() as u64
}

/// Run the alarm scheduler
pub async fn run_scheduler(
    state: crate::state::SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Print initial alarm list
    {
        let state_guard = state.read().await;
        println!(
            "Starting alarm scheduler with {} alarms",
            state_guard.config.alarms.len()
        );

        for alarm in &state_guard.config.alarms {
            if alarm.enabled {
                println!(
                    "  - {}: {} (days: {:?})",
                    alarm.name, alarm.time, alarm.days
                );
            } else {
                println!("  - {}: {} [DISABLED]", alarm.name, alarm.time);
            }
        }
    }

    // Keep track of the last minute we checked to avoid duplicate triggers
    let mut last_played_hour_minute: Option<(u32, u32)> = None;

    loop {
        let now = Local::now();
        let current_weekday = now.weekday();
        let current_time = now.time();
        let current_hour_minute = (current_time.hour(), current_time.minute());

        // Only check alarms once per minute
        if last_played_hour_minute == Some(current_hour_minute) {
            sleep(Duration::from_secs(1)).await;
            continue;
        }
        println!("checking {}", current_time);

        // Read current alarms and audio device from shared state
        let (alarms, audio_device) = {
            let state_guard = state.read().await;
            (
                state_guard.config.alarms.clone(),
                Some(state_guard.config.spotify.audio_device.clone()),
            )
        };

        for alarm in &alarms {
            if !alarm.enabled {
                continue;
            }

            // Check if alarm should play today
            if !alarm.should_play_on(current_weekday) {
                continue;
            }

            // Parse alarm time
            let alarm_time = match alarm.parse_time() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error parsing alarm time for '{}': {}", alarm.name, e);
                    continue;
                }
            };

            // Check if it's time to play
            let hour_match = current_time.hour() == alarm_time.hour();
            let minute_match = current_time.minute() == alarm_time.minute();

            if hour_match && minute_match {
                println!("\n🔔 Alarm triggered: {} at {}", alarm.name, alarm.time);

                // Play the alarm (spirc is Arc<Mutex<>> now, so it's not consumed)
                match crate::spotify::play(audio_device.clone()).await {
                    Ok(_) => {
                        println!(
                            "✓ Alarm '{}' played successfully... Will start checking for the next alarm at the start of the next minute",
                            alarm.name
                        );
                        // Update last trigger time in state
                        if let Ok(mut state_guard) = state.try_write() {
                            state_guard.last_alarm_trigger = Some((alarm.name.clone(), now));
                        }
                    }
                    Err(e) => {
                        eprintln!("✗ Error playing alarm '{}': {}", alarm.name, e);
                        eprintln!("   Scheduler will continue running for next alarm");
                        eprintln!("   If this persists, you may need to restart the program");
                    }
                }

                last_played_hour_minute = Some((current_time.hour(), current_time.minute()));
                break; // Break inner loop but continue outer loop
            }
        }

        // Check every 1 second
        sleep(Duration::from_secs(1)).await;
    }
}
