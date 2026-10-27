use dirs::home_dir;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
pub const DEFAULT_START_MINS: i32 = 480; // 8:00 AM
pub const WORK_DAY_DURATION_MINS: u32 = 480; // 8 hours

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskRecord {
    pub id: String,
    pub name: String,
    pub duration_mins: u32,
    pub start_time: String,
}
impl TaskRecord {
    pub fn to_input_string(&self) -> String {
        let h = self.duration_mins / 60;
        let m = self.duration_mins % 60;
        let mut parts = vec![self.name.clone()];
        if h > 0 {
            parts.push(format!("{}h", h));
        }
        if m > 0 {
            parts.push(format!("{}m", m));
        }
        parts.push(self.start_time.clone());
        parts.join(" ")
    }

    pub fn start_mins_since_midnight(&self) -> i32 {
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

        hour * 60 + min
    }

    pub fn start_mins_relative_to(&self, start_mins: i32) -> i32 {
        self.start_mins_since_midnight() - start_mins
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppData {
    pub tasks: HashMap<String, Vec<TaskRecord>>,
}

impl AppData {
    pub fn get_start_mins(&self, date: &str) -> i32 {
        self.tasks
            .get(date)
            .and_then(|tasks| {
                tasks
                    .iter()
                    .map(TaskRecord::start_mins_since_midnight)
                    .min()
            })
            .unwrap_or(DEFAULT_START_MINS)
    }

    pub fn get_end_mins(&self, date: &str) -> i32 {
        let start_mins = self.get_start_mins(date);
        let minimum_end_mins = start_mins + WORK_DAY_DURATION_MINS as i32;
        self.tasks
            .get(date)
            .and_then(|tasks| {
                tasks
                    .iter()
                    .map(|task| task.start_mins_since_midnight() + task.duration_mins as i32)
                    .max()
            })
            .map_or(minimum_end_mins, |latest_task_end| {
                minimum_end_mins.max(latest_task_end)
            })
    }
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }
}

pub fn get_data_path() -> PathBuf {
    let mut path = home_dir().expect("Could not find home directory");
    path.push(".jira-time-reporter.json");
    path
}

pub fn load_data(path: &Path) -> AppData {
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str(&contents) {
                return data;
            }
        }
    }
    AppData::default()
}

pub fn save_data(data: &AppData, path: &Path) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, json);
    }
}

pub fn mins_to_time_string(mins: i32) -> String {
    let mut h = (mins / 60) % 24;
    let m = mins % 60;
    let ampm = if h >= 12 { "PM" } else { "AM" };
    if h > 12 {
        h -= 12;
    } else if h == 0 {
        h = 12;
    }
    format!("{:02}:{:02} {}", h, m, ampm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_mins_relative_to() {
        let task = TaskRecord {
            id: "1".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "08:00 AM".to_string(),
        };
        assert_eq!(task.start_mins_relative_to(DEFAULT_START_MINS), 0);
        assert_eq!(task.start_mins_relative_to(420), 60);

        let task2 = TaskRecord {
            id: "2".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "10:30 AM".to_string(),
        };
        assert_eq!(task2.start_mins_relative_to(DEFAULT_START_MINS), 150);

        let task3 = TaskRecord {
            id: "3".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "01:00 PM".to_string(),
        };
        assert_eq!(task3.start_mins_relative_to(DEFAULT_START_MINS), 300); // 13:00 - 8:00 = 5 hours = 300 mins

        let task4 = TaskRecord {
            id: "4".to_string(),
            name: "Test".to_string(),
            duration_mins: 60,
            start_time: "12:00 PM".to_string(),
        };
        assert_eq!(task4.start_mins_relative_to(DEFAULT_START_MINS), 240); // 12:00 PM is noon, 4 hours after 8 AM
    }

    #[test]
    fn test_timeline_start_uses_earliest_task_for_each_date() {
        let mut tasks = HashMap::new();
        tasks.insert(
            "2026-10-27".to_string(),
            vec![
                TaskRecord {
                    id: "later".to_string(),
                    name: "Later".to_string(),
                    duration_mins: 30,
                    start_time: "09:00 AM".to_string(),
                },
                TaskRecord {
                    id: "earlier".to_string(),
                    name: "Earlier".to_string(),
                    duration_mins: 30,
                    start_time: "07:30 AM".to_string(),
                },
            ],
        );
        tasks.insert(
            "2026-10-28".to_string(),
            vec![TaskRecord {
                id: "another-day".to_string(),
                name: "Another day".to_string(),
                duration_mins: 30,
                start_time: "10:00 AM".to_string(),
            }],
        );
        let data = AppData { tasks };

        assert_eq!(data.get_start_mins("2026-10-27"), 450);
        assert_eq!(data.get_start_mins("2026-10-28"), 600);
        assert_eq!(data.get_start_mins("2026-10-29"), DEFAULT_START_MINS);
    }

    #[test]
    fn test_timeline_end_expands_for_long_days() {
        let data = AppData {
            tasks: HashMap::from([(
                "2026-10-27".to_string(),
                vec![TaskRecord {
                    id: "long-day".to_string(),
                    name: "Long day".to_string(),
                    duration_mins: 8 * 60 + 30,
                    start_time: "07:30 AM".to_string(),
                }],
            )]),
        };

        assert_eq!(data.get_start_mins("2026-10-27"), 450);
        assert_eq!(data.get_end_mins("2026-10-27"), 960);
    }

    #[test]
    fn test_legacy_start_offsets_are_ignored_when_loading() {
        let data: AppData = serde_json::from_str(
            r#"{
                "tasks": {
                    "2026-10-27": [{
                        "id": "first",
                        "name": "First",
                        "duration_mins": 30,
                        "start_time": "07:30 AM"
                    }]
                },
                "start_mins": 600,
                "daily_start_mins": {"2026-10-27": 660}
            }"#,
        )
        .unwrap();

        assert_eq!(data.get_start_mins("2026-10-27"), 450);
    }
}
