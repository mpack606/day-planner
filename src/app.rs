use crate::models::{AppData, TaskRecord, load_data, save_data};
use chrono::{Local, NaiveDate};
use tui_input::Input;
use uuid::Uuid;
use regex::Regex;

pub struct App {
    pub data: AppData,
    pub current_date: NaiveDate,
    pub input: Input,
    pub input_mode: bool,
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
            should_quit: false,
        }
    }

    pub fn next_day(&mut self) {
        self.current_date = self.current_date.succ_opt().unwrap_or(self.current_date);
    }

    pub fn previous_day(&mut self) {
        self.current_date = self.current_date.pred_opt().unwrap_or(self.current_date);
    }

    pub fn save(&self) {
        save_data(&self.data);
    }

    pub fn handle_submit(&mut self) {
        let input_val = self.input.value().trim();
        if input_val.is_empty() {
            return;
        }

        // Match format like "WMI-1234 3h 25m 10:00 AM" or "Task 1h 02:30 PM"
        let re = Regex::new(r"^(?P<name>.+?)\s+(?:(?P<h>\d+)h)?\s*(?:(?P<m>\d+)m)?\s*(?P<start>\d{1,2}:\d{2}\s*(?:AM|PM|am|pm))$").unwrap();
        
        if let Some(caps) = re.captures(input_val) {
            let name = caps.name("name").map_or("", |m| m.as_str()).to_string();
            let hours: u32 = caps.name("h").map_or("0", |m| m.as_str()).parse().unwrap_or(0);
            let mins: u32 = caps.name("m").map_or("0", |m| m.as_str()).parse().unwrap_or(0);
            let start_time = caps.name("start").map_or("", |m| m.as_str()).to_string();
            
            let total_mins = hours * 60 + mins;
            if total_mins > 0 {
                let rec = TaskRecord {
                    id: Uuid::new_v4().to_string(),
                    name,
                    duration_mins: total_mins,
                    start_time,
                };
                let date_str = self.current_date.to_string();
                self.data.tasks.entry(date_str).or_insert_with(Vec::new).push(rec);
                self.save();
                self.input.reset();
                self.input_mode = false;
            }
        }
    }
}
