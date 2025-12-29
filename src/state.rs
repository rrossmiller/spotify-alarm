use crate::alarm::{Alarm, AlarmConfig};
use chrono::{DateTime, Local};
use librespot::connect::Spirc;
use librespot::core::session::Session;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub config: AlarmConfig,
    pub config_path: PathBuf,
    pub session: Session,
    pub spirc: Arc<Mutex<Spirc>>,
    pub last_alarm_trigger: Option<(String, DateTime<Local>)>,
}

pub type SharedState = Arc<tokio::sync::RwLock<AppState>>;

impl AppState {
    /// Save the current configuration to disk
    pub fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.config.save(&self.config_path)?;
        Ok(())
    }

    /// Reload configuration from disk
    pub fn load_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config = AlarmConfig::load(&self.config_path)?;
        Ok(())
    }

    /// Get all alarms
    pub fn get_alarms(&self) -> Vec<Alarm> {
        self.config.alarms.clone()
    }

    /// Get a specific alarm by index
    pub fn get_alarm(&self, index: usize) -> Option<Alarm> {
        self.config.alarms.get(index).cloned()
    }

    /// Update an alarm at a specific index
    pub fn update_alarm(&mut self, index: usize, alarm: Alarm) -> Result<(), String> {
        if index >= self.config.alarms.len() {
            return Err(format!("Index {} out of bounds", index));
        }
        self.config.alarms[index] = alarm;
        Ok(())
    }

    /// Add a new alarm
    pub fn add_alarm(&mut self, alarm: Alarm) {
        self.config.alarms.push(alarm);
    }

    /// Delete an alarm by index
    pub fn delete_alarm(&mut self, index: usize) -> Result<(), String> {
        if index >= self.config.alarms.len() {
            return Err(format!("Index {} out of bounds", index));
        }
        self.config.alarms.remove(index);
        Ok(())
    }

    /// Toggle an alarm's enabled state
    pub fn toggle_alarm(&mut self, index: usize) -> Result<Alarm, String> {
        if index >= self.config.alarms.len() {
            return Err(format!("Index {} out of bounds", index));
        }
        self.config.alarms[index].enabled = !self.config.alarms[index].enabled;
        Ok(self.config.alarms[index].clone())
    }
}
