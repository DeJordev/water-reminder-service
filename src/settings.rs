use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

const APP_DIR: &str = "water-reminder";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("no se pudo crear el directorio de configuracion {path}: {source}")]
    CreateDir { path: PathBuf, source: io::Error },
    #[error("no se pudo escribir la configuracion temporal {path}: {source}")]
    WriteTemp { path: PathBuf, source: io::Error },
    #[error("no se pudo reemplazar la configuracion {path}: {source}")]
    Replace { path: PathBuf, source: io::Error },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub interval_minutes: u32,
    pub daily_goal: u32,
    pub active_start_hour: u32,
    pub active_end_hour: u32,
    pub sound_enabled: bool,
    pub fullscreen_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_minutes: 45,
            daily_goal: 8,
            active_start_hour: 9,
            active_end_hour: 22,
            sound_enabled: true,
            fullscreen_mode: true,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        if !(1..=240).contains(&self.interval_minutes) {
            self.interval_minutes = 45;
        }
        if !(1..=20).contains(&self.daily_goal) {
            self.daily_goal = 8;
        }
        if self.active_start_hour > 23 {
            self.active_start_hour = 9;
        }
        if !(1..=24).contains(&self.active_end_hour) {
            self.active_end_hour = 22;
        }
        if self.active_end_hour <= self.active_start_hour {
            self.active_start_hour = 9;
            self.active_end_hour = 22;
        }
    }
}

pub fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join(APP_DIR)
}

pub fn config_file() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

pub fn load_settings() -> Settings {
    load_settings_from_path(&config_file())
}

pub fn load_settings_from_path(path: &Path) -> Settings {
    let mut settings = Settings::default();

    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(Value::Object(data)) = serde_json::from_str::<Value>(&text) {
            if let Some(value) = data.get("interval_minutes") {
                settings.interval_minutes = int_from_value(value, settings.interval_minutes);
            }
            if let Some(value) = data.get("daily_goal") {
                settings.daily_goal = int_from_value(value, settings.daily_goal);
            }
            if let Some(value) = data.get("active_start_hour") {
                settings.active_start_hour = int_from_value(value, settings.active_start_hour);
            }
            if let Some(value) = data.get("active_end_hour") {
                settings.active_end_hour = int_from_value(value, settings.active_end_hour);
            }
            if let Some(value) = data.get("sound_enabled") {
                settings.sound_enabled = bool_from_value(value, settings.sound_enabled);
            }
            if let Some(value) = data.get("fullscreen_mode") {
                settings.fullscreen_mode = bool_from_value(value, settings.fullscreen_mode);
            }
        }
    }

    if let Some(interval) = env_int("WATER_INTERVAL") {
        settings.interval_minutes = interval;
    }
    if let Some(goal) = env_int("WATER_DAILY_GOAL") {
        settings.daily_goal = goal;
    }

    settings.normalize();
    settings
}

pub fn save_settings(settings: &Settings) -> Result<(), SettingsError> {
    save_settings_to_path(settings, &config_file())
}

pub fn save_settings_to_path(settings: &Settings, path: &Path) -> Result<(), SettingsError> {
    let mut normalized = settings.clone();
    normalized.normalize();

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).map_err(|source| SettingsError::CreateDir {
        path: dir.to_path_buf(),
        source,
    })?;

    let tmp_path = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(&normalized).expect("serializar settings no falla");
    fs::write(&tmp_path, text).map_err(|source| SettingsError::WriteTemp {
        path: tmp_path.clone(),
        source,
    })?;
    fs::rename(&tmp_path, path).map_err(|source| SettingsError::Replace {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn env_int(name: &str) -> Option<u32> {
    env::var(name).ok()?.parse::<u32>().ok()
}

fn int_from_value(value: &Value, default: u32) -> u32 {
    match value {
        Value::Bool(_) => default,
        Value::Number(number) => number
            .as_i64()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(default),
        Value::String(text) => text.parse::<u32>().unwrap_or(default),
        _ => default,
    }
}

fn bool_from_value(value: &Value, default: bool) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn invalid_config_values_fall_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(
            &path,
            json!({
                "interval_minutes": "bad",
                "daily_goal": 0,
                "active_start_hour": 23,
                "active_end_hour": 9,
                "sound_enabled": "false",
                "fullscreen_mode": "off"
            })
            .to_string(),
        )
        .unwrap();

        let loaded = load_settings_from_path(&path);

        assert_eq!(loaded.interval_minutes, 45);
        assert_eq!(loaded.daily_goal, 8);
        assert_eq!(loaded.active_start_hour, 9);
        assert_eq!(loaded.active_end_hour, 22);
        assert!(!loaded.sound_enabled);
        assert!(!loaded.fullscreen_mode);
    }

    #[test]
    fn save_settings_writes_python_compatible_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("water-reminder").join("config.json");
        let settings = Settings {
            interval_minutes: 30,
            daily_goal: 10,
            active_start_hour: 8,
            active_end_hour: 20,
            sound_enabled: false,
            fullscreen_mode: false,
        };

        save_settings_to_path(&settings, &path).unwrap();

        let text = fs::read_to_string(path).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["interval_minutes"], 30);
    }
}
