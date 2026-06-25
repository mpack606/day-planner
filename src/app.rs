use crate::models::{AppData, TaskRecord, load_data, save_data, mins_to_time_string};
use chrono::{Local, NaiveDate};
use tui_input::Input;
use uuid::Uuid;
use regex::Regex;

pub struct App {
    pub data: AppData,
    pub current_date: NaiveDate,
    pub input: Input,
    pub input_mode: bool,
    pub edit_mode: bool,
    pub selected_task_index: Option<usize>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let current_date = Local::now().date_naive();
        let data = load_data();
        Self {
            data,
            current_date,
            input: Input::default(),
            input_mode: false,
            edit_mode: false,
            selected_task_index: None,
            should_quit: false,
        }
    }

    pub fn next_day(&mut self) {
        self.current_date = self.current_date.succ_opt().unwrap_or(self.current_date);
    }

    pub fn previous_day(&mut self) {
        self.current_date = self.current_date.pred_opt().unwrap_or(self.current_date);
    }

    pub fn get_total_duration_mins(&self) -> u32 {
        let date_str = self.current_date.to_string();
        self.data.tasks.get(&date_str)
            .map(|tasks| tasks.iter().map(|t| t.duration_mins).sum())
            .unwrap_or(0)
    }

    pub fn save(&self) {
        save_data(&self.data);
    }

    pub fn enter_edit_mode(&mut self) {
        let date_str = self.current_date.to_string();
        let start_mins = self.data.get_start_mins(&date_str);
        if let Some(tasks) = self.data.tasks.get_mut(&date_str) {
            if !tasks.is_empty() {
                tasks.sort_by_key(|t| t.start_mins_relative_to(start_mins));
                self.edit_mode = true;
                self.input_mode = false;
                self.selected_task_index = Some(0);
                self.update_input_with_selected();
            }
        }
    }

    pub fn move_selection_up(&mut self) {
        if let Some(index) = self.selected_task_index {
            if index > 0 {
                self.selected_task_index = Some(index - 1);
                self.update_input_with_selected();
            }
        }
    }

    pub fn move_selection_down(&mut self) {
        let date_str = self.current_date.to_string();
        if let Some(tasks) = self.data.tasks.get(&date_str) {
            if let Some(index) = self.selected_task_index {
                if index < tasks.len() - 1 {
                    self.selected_task_index = Some(index + 1);
                    self.update_input_with_selected();
                }
            }
        }
    }

    fn update_input_with_selected(&mut self) {
        let date_str = self.current_date.to_string();
        if let Some(tasks) = self.data.tasks.get(&date_str) {
            if let Some(index) = self.selected_task_index {
                if let Some(task) = tasks.get(index) {
                    let input_str = task.to_input_string();
                    self.input = Input::new(input_str);
                }
            }
        }
    }

    pub fn handle_submit(&mut self) {
        let input_val = self.input.value().trim();
        if input_val.is_empty() {
            return;
        }

        let re = Regex::new(r"^(?P<name>.+?)\s+(?:(?P<h>\d+)h)?\s*(?:(?P<m>\d+)m)?(?:\s+(?P<start>\d{1,2}:\d{2}\s*(?:AM|PM|am|pm|Am|Pm)))?$").unwrap();
        
        if let Some(caps) = re.captures(input_val) {
            let name = caps.name("name").map_or("", |m| m.as_str()).to_string();
            let hours: u32 = caps.name("h").map_or("0", |m| m.as_str()).parse().unwrap_or(0);
            let mins: u32 = caps.name("m").map_or("0", |m| m.as_str()).parse().unwrap_or(0);
            
            let total_mins = hours * 60 + mins;
            if total_mins > 0 {
                let date_str = self.current_date.to_string();
                
                let start_time = if let Some(start_cap) = caps.name("start") {
                    start_cap.as_str().to_string()
                } else {
                    let day_start = self.data.get_start_mins(&date_str);
                    self.data.tasks.get(&date_str).and_then(|tasks| {
                        tasks.iter().max_by_key(|t| t.start_mins_since_midnight())
                    }).map(|last_task| {
                        mins_to_time_string(last_task.start_mins_since_midnight() + last_task.duration_mins as i32)
                    }).unwrap_or_else(|| mins_to_time_string(day_start))
                };

                if self.edit_mode {
                    if let Some(index) = self.selected_task_index {
                        let start_mins = self.data.get_start_mins(&date_str);
                        if let Some(tasks) = self.data.tasks.get_mut(&date_str) {
                            tasks.sort_by_key(|t| t.start_mins_relative_to(start_mins));
                            if let Some(task) = tasks.get_mut(index) {
                                task.name = name;
                                task.duration_mins = total_mins;
                                task.start_time = start_time;
                            }
                        }
                    }
                    self.edit_mode = false;
                    self.selected_task_index = None;
                } else {
                    let rec = TaskRecord {
                        id: Uuid::new_v4().to_string(),
                        name,
                        duration_mins: total_mins,
                        start_time,
                    };
                    self.data.tasks.entry(date_str).or_insert_with(Vec::new).push(rec);
                }
                
                self.save();
                self.input.reset();
                self.input_mode = false;
            }
        }
    }

    pub fn shift_start_time(&mut self, delta_mins: i32) {
        let date_str = self.current_date.to_string();
        let current_start = self.data.get_start_mins(&date_str);
        let new_start = (current_start + delta_mins).clamp(0, 1410); // 1410 = 23:30
        self.data.daily_start_mins.insert(date_str, new_start);
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskRecord;
    use std::collections::HashMap;

    #[test]
    fn test_get_total_duration_mins() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let date_str = date.to_string();
        
        let mut tasks = HashMap::new();
        tasks.insert(date_str, vec![
            TaskRecord {
                id: "1".to_string(),
                name: "Task 1".to_string(),
                duration_mins: 60,
                start_time: "08:00 AM".to_string(),
            },
            TaskRecord {
                id: "2".to_string(),
                name: "Task 2".to_string(),
                duration_mins: 30,
                start_time: "09:00 AM".to_string(),
            },
        ]);

        let app = App {
            data: AppData {
                tasks,
                start_mins: 480,
                daily_start_mins: HashMap::new(),
            },
            current_date: date,
            input: Input::default(),
            input_mode: false,
            edit_mode: false,
            selected_task_index: None,
            should_quit: false,
        };

        assert_eq!(app.get_total_duration_mins(), 90);
    }

    #[test]
    fn test_daily_start_time_shift() {
        let date1 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2023, 10, 28).unwrap();
        
        let mut app = App {
            data: AppData::default(),
            current_date: date1,
            input: Input::default(),
            input_mode: false,
            edit_mode: false,
            selected_task_index: None,
            should_quit: false,
        };

        // Default should be 480 (8:00 AM)
        assert_eq!(app.data.get_start_mins(&date1.to_string()), 480);
        assert_eq!(app.data.get_start_mins(&date2.to_string()), 480);

        // Shift date1 by 30 mins
        app.shift_start_time(30);
        assert_eq!(app.data.get_start_mins(&date1.to_string()), 510);
        assert_eq!(app.data.get_start_mins(&date2.to_string()), 480);

        // Switch to date2 and shift by -60 mins
        app.current_date = date2;
        app.shift_start_time(-60);
        assert_eq!(app.data.get_start_mins(&date1.to_string()), 510);
        assert_eq!(app.data.get_start_mins(&date2.to_string()), 420);
    }

    #[test]
    fn test_handle_submit_optional_start_time() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let mut app = App {
            data: AppData {
                tasks: HashMap::new(),
                start_mins: 480, // 8:00 AM
                daily_start_mins: HashMap::new(),
            },
            current_date: date,
            input: Input::default(),
            input_mode: true,
            edit_mode: false,
            selected_task_index: None,
            should_quit: false,
        };

        // 1. Add first task without start time -> should be 08:00 AM
        app.input = Input::new("Task 1 30m".to_string());
        app.handle_submit();
        
        let date_str = date.to_string();
        let tasks = app.data.tasks.get(&date_str).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "Task 1");
        assert_eq!(tasks[0].duration_mins, 30);
        assert_eq!(tasks[0].start_time, "08:00 AM");

        // 2. Add second task without start time -> should be 08:30 AM (08:00 + 30m)
        app.input = Input::new("Task 2 1h".to_string());
        app.handle_submit();
        
        let tasks = app.data.tasks.get(&date_str).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].name, "Task 2");
        assert_eq!(tasks[1].duration_mins, 60);
        assert_eq!(tasks[1].start_time, "08:30 AM");

        // 3. Add third task with explicit start time
        app.input = Input::new("Task 3 15m 10:00 AM".to_string());
        app.handle_submit();
        
        let tasks = app.data.tasks.get(&date_str).unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[2].name, "Task 3");
        assert_eq!(tasks[2].start_time, "10:00 AM");

        // 4. Add fourth task without start time -> should be 10:15 AM (10:00 + 15m)
        app.input = Input::new("Task 4 10m".to_string());
        app.handle_submit();
        
        let tasks = app.data.tasks.get(&date_str).unwrap();
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[3].name, "Task 4");
        assert_eq!(tasks[3].start_time, "10:15 AM");
    }

    #[test]
    fn test_handle_submit_with_shifted_day_start() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let mut app = App {
            data: AppData {
                tasks: HashMap::new(),
                start_mins: 480, // 8:00 AM
                daily_start_mins: HashMap::new(),
            },
            current_date: date,
            input: Input::default(),
            input_mode: true,
            edit_mode: false,
            selected_task_index: None,
            should_quit: false,
        };

        // Shift day start to 7:00 AM
        app.shift_start_time(-60);
        
        app.input = Input::new("Task 1 30m".to_string());
        app.handle_submit();
        
        let date_str = date.to_string();
        let tasks = app.data.tasks.get(&date_str).unwrap();
        assert_eq!(tasks[0].start_time, "07:00 AM");
    }
}
