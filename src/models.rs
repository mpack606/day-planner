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

impl TaskRecord {
    pub fn start_mins_from_8am(&self) -> i32 {
        let parts: Vec<&str> = self.start_time.split_whitespace().collect();
        if parts.len() < 2 {
            return 0;
        }

        let time_parts: Vec<&str> = parts[0].split(':').collect();
        if time_parts.len() < 2 {
            return 0;
        }

        let mut hour: i32 = time_parts[0].parse().unwrap_or(0);
        let min: i32 = time_parts[1].parse().unwrap_or(0);
        let ampm = parts[1].to_lowercase();

        if ampm == "pm" && hour < 12 {
            hour += 12;
        } else if ampm == "am" && hour == 12 {
            hour = 0;
        }

        // Minutes since midnight
        let total_mins = hour * 60 + min;
        // Minutes since 8:00 AM
        total_mins - (8 * 60)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_mins_from_8am() {
        let task = TaskRecord {
            id: "1".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "08:00 AM".to_string(),
        };
        assert_eq!(task.start_mins_from_8am(), 0);

        let task2 = TaskRecord {
            id: "2".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "10:30 AM".to_string(),
        };
        assert_eq!(task2.start_mins_from_8am(), 150);

        let task3 = TaskRecord {
            id: "3".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "01:00 PM".to_string(),
        };
        assert_eq!(task3.start_mins_from_8am(), 300); // 13:00 - 8:00 = 5 hours = 300 mins

        let task4 = TaskRecord {
            id: "4".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "12:00 PM".to_string(),
        };
        assert_eq!(task4.start_mins_from_8am(), 240); // 12:00 PM is noon, 4 hours after 8 AM
    }
}
