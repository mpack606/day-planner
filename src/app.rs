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

    pub fn save(&self) {
        save_data(&self.data);
    }

    pub fn enter_edit_mode(&mut self) {
        let date_str = self.current_date.to_string();
        if let Some(tasks) = self.data.tasks.get_mut(&date_str) {
            if !tasks.is_empty() {
                tasks.sort_by_key(|t| t.start_mins_from_8am());
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

        let re = Regex::new(r"^(?P<name>.+?)\s+(?:(?P<h>\d+)h)?\s*(?:(?P<m>\d+)m)?\s*(?P<start>\d{1,2}:\d{2}\s*(?:AM|PM|am|pm|Am|Pm))$").unwrap();
        
        if let Some(caps) = re.captures(input_val) {
            let name = caps.name("name").map_or("", |m| m.as_str()).to_string();
            let hours: u32 = caps.name("h").map_or("0", |m| m.as_str()).parse().unwrap_or(0);
            let mins: u32 = caps.name("m").map_or("0", |m| m.as_str()).parse().unwrap_or(0);
            let start_time = caps.name("start").map_or("", |m| m.as_str()).to_string();
            
            let total_mins = hours * 60 + mins;
            if total_mins > 0 {
                let date_str = self.current_date.to_string();
                
                if self.edit_mode {
                    if let Some(index) = self.selected_task_index {
                        if let Some(tasks) = self.data.tasks.get_mut(&date_str) {
                            tasks.sort_by_key(|t| t.start_mins_from_8am());
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
}
