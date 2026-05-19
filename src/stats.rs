use chrono::Local;
use serde_json::{Map, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

const APP_DIR: &str = "water-reminder";
const STATS_FILE: &str = "stats.json";

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("no se pudo crear el directorio de datos {path}: {source}")]
    CreateDir { path: PathBuf, source: io::Error },
    #[error("no se pudo escribir stats temporal {path}: {source}")]
    WriteTemp { path: PathBuf, source: io::Error },
    #[error("no se pudo reemplazar stats {path}: {source}")]
    Replace { path: PathBuf, source: io::Error },
}

pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
        })
        .join(APP_DIR)
}

pub fn stats_file() -> PathBuf {
    data_dir().join(STATS_FILE)
}

pub fn today_key() -> String {
    Local::now().date_naive().to_string()
}

pub fn get_today_count() -> u32 {
    get_today_count_from_path(&stats_file(), &today_key())
}

pub fn increment_today() -> Result<u32, StatsError> {
    increment_day(&stats_file(), &today_key())
}

pub fn reset_today() -> Result<(), StatsError> {
    reset_day(&stats_file(), &today_key())
}

pub fn get_today_count_from_path(path: &Path, key: &str) -> u32 {
    let stats = load_map(path);
    count_from_value(stats.get(key).unwrap_or(&Value::Null))
}

pub fn increment_day(path: &Path, key: &str) -> Result<u32, StatsError> {
    let mut stats = load_map(path);
    let next = count_from_value(stats.get(key).unwrap_or(&Value::Null)) + 1;
    stats.insert(key.to_owned(), Value::from(next));
    save_map(path, &stats)?;
    Ok(next)
}

pub fn reset_day(path: &Path, key: &str) -> Result<(), StatsError> {
    let mut stats = load_map(path);
    stats.insert(key.to_owned(), Value::from(0));
    save_map(path, &stats)
}

fn load_map(path: &Path) -> Map<String, Value> {
    let Ok(text) = fs::read_to_string(path) else {
        return Map::new();
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn save_map(path: &Path, stats: &Map<String, Value>) -> Result<(), StatsError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir).map_err(|source| StatsError::CreateDir {
        path: dir.to_path_buf(),
        source,
    })?;

    let tmp_path = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(stats).expect("serializar stats no falla");
    fs::write(&tmp_path, text).map_err(|source| StatsError::WriteTemp {
        path: tmp_path.clone(),
        source,
    })?;
    fs::rename(&tmp_path, path).map_err(|source| StatsError::Replace {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn count_from_value(value: &Value) -> u32 {
    match value {
        Value::Bool(_) => 0,
        Value::Number(number) => number
            .as_i64()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0),
        Value::String(text) => text
            .parse::<i64>()
            .ok()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn invalid_stats_file_starts_at_zero() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("stats.json");
        fs::write(&path, "[]").unwrap();

        assert_eq!(get_today_count_from_path(&path, "2026-05-19"), 0);
        assert_eq!(increment_day(&path, "2026-05-19").unwrap(), 1);
    }

    #[test]
    fn non_numeric_today_count_is_recovered() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("water-reminder").join("stats.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, json!({"2026-05-19": "bad"}).to_string()).unwrap();

        assert_eq!(get_today_count_from_path(&path, "2026-05-19"), 0);
        assert_eq!(increment_day(&path, "2026-05-19").unwrap(), 1);
    }
}
