use dirs::home_dir;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskRecord {
    pub id: String,
    pub name: String,
    pub duration_mins: u32,
    pub start_time: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppData {
    pub tasks: HashMap<String, Vec<TaskRecord>>,
}

pub fn get_data_path() -> PathBuf {
    let mut path = home_dir().expect("Could not find home directory");
    path.push(".jira-time-reporter.json");
    path
}

pub fn load_data() -> AppData {
    let path = get_data_path();
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str(&contents) {
                return data;
            }
        }
    }
    AppData::default()
}

pub fn save_data(data: &AppData) {
    let path = get_data_path();
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, json);
    }
}
