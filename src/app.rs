use crate::models::{
    AppData, TaskRecord, WORK_DAY_DURATION_MINS, get_data_path, load_data, mins_to_time_string,
    save_data,
};
use chrono::{Local, NaiveDate};
use regex::Regex;
use std::path::PathBuf;
use tui_input::Input;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyStage {
    Inactive,
    SelectSource,
    SelectDestination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptySlot {
    pub start_mins: i32,
    pub duration_mins: u32,
}

pub struct App {
    pub data: AppData,
    pub storage_path: PathBuf,
    pub current_date: NaiveDate,
    pub input: Input,
    pub input_mode: bool,
    pub edit_mode: bool,
    pub fill_mode: bool,
    pub selected_task_index: Option<usize>,
    pub copy_stage: CopyStage,
    pub copy_source_task_id: Option<String>,
    pub selected_gap_index: Option<usize>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let path = get_data_path();
        let data = load_data(&path);
        let current_date = Local::now().date_naive();
        Self {
            data,
            storage_path: path,
            current_date,
            input: Input::default(),
            input_mode: false,
            edit_mode: false,
            fill_mode: false,
            selected_task_index: None,
            copy_stage: CopyStage::Inactive,
            copy_source_task_id: None,
            selected_gap_index: None,
            should_quit: false,
        }
    }

    pub fn with_path(path: PathBuf) -> Self {
        let data = load_data(&path);
        let current_date = Local::now().date_naive();
        Self {
            data,
            storage_path: path,
            current_date,
            input: Input::default(),
            input_mode: false,
            edit_mode: false,
            fill_mode: false,
            selected_task_index: None,
            copy_stage: CopyStage::Inactive,
            copy_source_task_id: None,
            selected_gap_index: None,
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
        self.data
            .tasks
            .get(&date_str)
            .map(|tasks| tasks.iter().map(|t| t.duration_mins).sum())
            .unwrap_or(0)
    }

    pub fn save(&self) {
        save_data(&self.data, &self.storage_path);
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

    pub fn enter_copy_mode(&mut self) {
        let date_str = self.current_date.to_string();
        let start_mins = self.data.get_start_mins(&date_str);
        if let Some(tasks) = self.data.tasks.get_mut(&date_str) {
            if !tasks.is_empty() {
                tasks.sort_by_key(|task| task.start_mins_relative_to(start_mins));
                self.input_mode = false;
                self.edit_mode = false;
                self.fill_mode = false;
                self.copy_stage = CopyStage::SelectSource;
                self.copy_source_task_id = None;
                self.selected_task_index = Some(0);
                self.selected_gap_index = None;
            }
        }
    }

    pub fn cancel_copy_mode(&mut self) {
        self.copy_stage = CopyStage::Inactive;
        self.copy_source_task_id = None;
        self.selected_task_index = None;
        self.selected_gap_index = None;
    }

    pub fn is_copy_mode(&self) -> bool {
        self.copy_stage != CopyStage::Inactive
    }

    pub fn empty_slots(&self) -> Vec<EmptySlot> {
        let date_str = self.current_date.to_string();
        let start_mins = self.data.get_start_mins(&date_str);
        let mut tasks = self.data.tasks.get(&date_str).cloned().unwrap_or_default();
        tasks.sort_by_key(|task| task.start_mins_relative_to(start_mins));

        let mut slots = Vec::new();
        let mut current_min = 0;

        for task in tasks {
            let task_start = task.start_mins_relative_to(start_mins);
            if task_start > current_min {
                slots.push(EmptySlot {
                    start_mins: start_mins + current_min,
                    duration_mins: (task_start - current_min) as u32,
                });
            }
            current_min = current_min.max(task_start + task.duration_mins as i32);
        }

        if current_min < WORK_DAY_DURATION_MINS as i32 {
            slots.push(EmptySlot {
                start_mins: start_mins + current_min,
                duration_mins: (WORK_DAY_DURATION_MINS as i32 - current_min) as u32,
            });
        }

        slots
    }

    pub fn move_copy_selection_previous(&mut self) {
        match self.copy_stage {
            CopyStage::SelectSource => {
                if let Some(index) = self.selected_task_index {
                    self.selected_task_index = Some(index.saturating_sub(1));
                }
            }
            CopyStage::SelectDestination => {
                if let Some(index) = self.selected_gap_index {
                    self.selected_gap_index = Some(index.saturating_sub(1));
                }
            }
            CopyStage::Inactive => {}
        }
    }

    pub fn move_copy_selection_next(&mut self) {
        match self.copy_stage {
            CopyStage::SelectSource => {
                let task_count = self
                    .data
                    .tasks
                    .get(&self.current_date.to_string())
                    .map_or(0, Vec::len);
                if let Some(index) = self.selected_task_index {
                    if index + 1 < task_count {
                        self.selected_task_index = Some(index + 1);
                    }
                }
            }
            CopyStage::SelectDestination => {
                let gap_count = self.empty_slots().len();
                if let Some(index) = self.selected_gap_index {
                    if index + 1 < gap_count {
                        self.selected_gap_index = Some(index + 1);
                    }
                }
            }
            CopyStage::Inactive => {}
        }
    }

    pub fn confirm_copy_selection(&mut self) {
        match self.copy_stage {
            CopyStage::SelectSource => {
                let date_str = self.current_date.to_string();
                let source_id = self
                    .selected_task_index
                    .and_then(|index| self.data.tasks.get(&date_str)?.get(index))
                    .map(|task| task.id.clone());

                if let Some(source_id) = source_id {
                    self.copy_source_task_id = Some(source_id);
                    self.copy_stage = CopyStage::SelectDestination;
                    self.selected_gap_index = (!self.empty_slots().is_empty()).then_some(0);
                }
            }
            CopyStage::SelectDestination => {
                let Some(slot) = self
                    .selected_gap_index
                    .and_then(|index| self.empty_slots().get(index).copied())
                else {
                    return;
                };
                let date_str = self.current_date.to_string();
                let Some(mut copied_task) =
                    self.copy_source_task_id.as_ref().and_then(|source_id| {
                        self.data
                            .tasks
                            .get(&date_str)?
                            .iter()
                            .find(|task| task.id == *source_id)
                            .cloned()
                    })
                else {
                    self.cancel_copy_mode();
                    return;
                };

                copied_task.id = Uuid::new_v4().to_string();
                copied_task.duration_mins = slot.duration_mins;
                copied_task.start_time = mins_to_time_string(slot.start_mins);
                self.data
                    .tasks
                    .entry(date_str)
                    .or_default()
                    .push(copied_task);
                self.save();
                self.cancel_copy_mode();
            }
            CopyStage::Inactive => {}
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

        if self.fill_mode {
            self.fill_gaps(input_val.to_string());
            self.fill_mode = false;
            self.input.reset();
            return;
        }

        let re = Regex::new(r"^(?P<name>.+?)\s+(?:(?P<h>\d+)h)?\s*(?:(?P<m>\d+)m)?(?:\s+(?P<start>\d{1,2}:\d{2}\s*(?:AM|PM|am|pm|Am|Pm)))?$").unwrap();

        if let Some(caps) = re.captures(input_val) {
            let name = caps.name("name").map_or("", |m| m.as_str()).to_string();
            let hours: u32 = caps
                .name("h")
                .map_or("0", |m| m.as_str())
                .parse()
                .unwrap_or(0);
            let mins: u32 = caps
                .name("m")
                .map_or("0", |m| m.as_str())
                .parse()
                .unwrap_or(0);

            let total_mins = hours * 60 + mins;
            if total_mins > 0 {
                let date_str = self.current_date.to_string();

                let start_time = if let Some(start_cap) = caps.name("start") {
                    start_cap.as_str().to_string()
                } else {
                    let day_start = self.data.get_start_mins(&date_str);
                    self.data
                        .tasks
                        .get(&date_str)
                        .and_then(|tasks| {
                            tasks.iter().max_by_key(|t| t.start_mins_since_midnight())
                        })
                        .map(|last_task| {
                            mins_to_time_string(
                                last_task.start_mins_since_midnight()
                                    + last_task.duration_mins as i32,
                            )
                        })
                        .unwrap_or_else(|| mins_to_time_string(day_start))
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
                    self.data
                        .tasks
                        .entry(date_str)
                        .or_insert_with(Vec::new)
                        .push(rec);
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

    pub fn fill_gaps(&mut self, task_name: String) {
        let date_str = self.current_date.to_string();
        let start_mins = self.data.get_start_mins(&date_str);

        let mut tasks = self.data.tasks.get(&date_str).cloned().unwrap_or_default();
        tasks.sort_by_key(|t| t.start_mins_relative_to(start_mins));

        let mut new_tasks = Vec::new();
        let mut current_min = 0; // Relative to start_mins

        for task in tasks {
            let task_start_relative = task.start_mins_relative_to(start_mins);
            if task_start_relative > current_min {
                // There is a gap
                let gap_duration = (task_start_relative - current_min) as u32;
                new_tasks.push(TaskRecord {
                    id: Uuid::new_v4().to_string(),
                    name: task_name.clone(),
                    duration_mins: gap_duration,
                    start_time: mins_to_time_string(start_mins + current_min),
                });
            }
            new_tasks.push(task.clone());
            current_min = task_start_relative + task.duration_mins as i32;
        }

        if current_min < WORK_DAY_DURATION_MINS as i32 {
            let gap_duration = (WORK_DAY_DURATION_MINS as i32 - current_min) as u32;
            new_tasks.push(TaskRecord {
                id: Uuid::new_v4().to_string(),
                name: task_name.clone(),
                duration_mins: gap_duration,
                start_time: mins_to_time_string(start_mins + current_min),
            });
        }

        self.data.tasks.insert(date_str, new_tasks);
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DEFAULT_START_MINS, TaskRecord};
    use std::collections::HashMap;

    #[test]
    fn test_get_total_duration_mins() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let date_str = date.to_string();

        let mut tasks = HashMap::new();
        tasks.insert(
            date_str,
            vec![
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
            ],
        );

        let app = App {
            data: AppData {
                tasks,
                start_mins: DEFAULT_START_MINS,
                daily_start_mins: HashMap::new(),
            },
            storage_path: PathBuf::from("test_data.json"),
            current_date: date,
            input: Input::default(),
            input_mode: false,
            edit_mode: false,
            fill_mode: false,
            selected_task_index: None,
            copy_stage: CopyStage::Inactive,
            copy_source_task_id: None,
            selected_gap_index: None,
            should_quit: false,
        };

        assert_eq!(app.get_total_duration_mins(), 90);
    }

    #[test]
    fn test_daily_start_time_shift() {
        let date1 = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2023, 10, 28).unwrap();

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_daily_start_time_shift.json");

        let mut app = App::with_path(path.clone());
        app.current_date = date1;

        // Default should be DEFAULT_START_MINS (8:00 AM)
        assert_eq!(
            app.data.get_start_mins(&date1.to_string()),
            DEFAULT_START_MINS
        );
        assert_eq!(
            app.data.get_start_mins(&date2.to_string()),
            DEFAULT_START_MINS
        );

        // Shift date1 by 30 mins
        app.shift_start_time(30);
        assert_eq!(app.data.get_start_mins(&date1.to_string()), 510);
        assert_eq!(app.data.get_start_mins(&date2.to_string()), 480);

        // Switch to date2 and shift by -60 mins
        app.current_date = date2;
        app.shift_start_time(-60);
        assert_eq!(app.data.get_start_mins(&date1.to_string()), 510);
        assert_eq!(app.data.get_start_mins(&date2.to_string()), 420);

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_handle_submit_optional_start_time() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_handle_submit_optional_start_time.json");

        let mut app = App::with_path(path.clone());
        app.current_date = date;
        app.input_mode = true;

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

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_handle_submit_with_shifted_day_start() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_handle_submit_with_shifted_day_start.json");

        let mut app = App::with_path(path.clone());
        app.current_date = date;
        app.input_mode = true;

        // Shift day start to 7:00 AM
        app.shift_start_time(-60);

        app.input = Input::new("Task 1 30m".to_string());
        app.handle_submit();

        let date_str = date.to_string();
        let tasks = app.data.tasks.get(&date_str).unwrap();
        assert_eq!(tasks[0].start_time, "07:00 AM");

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_fill_gaps() {
        let date = NaiveDate::from_ymd_opt(2023, 10, 27).unwrap();
        let date_str = date.to_string();
        let mut tasks = HashMap::new();
        tasks.insert(
            date_str.clone(),
            vec![
                TaskRecord {
                    id: "1".to_string(),
                    name: "Task 1".to_string(),
                    duration_mins: 10,
                    start_time: "08:00 AM".to_string(),
                },
                TaskRecord {
                    id: "2".to_string(),
                    name: "Task 2".to_string(),
                    duration_mins: 60,
                    start_time: "08:10 AM".to_string(),
                },
                TaskRecord {
                    id: "3".to_string(),
                    name: "Task 3".to_string(),
                    duration_mins: 10,
                    start_time: "01:30 PM".to_string(),
                },
                TaskRecord {
                    id: "4".to_string(),
                    name: "Task 4".to_string(),
                    duration_mins: 10,
                    start_time: "03:50 PM".to_string(),
                },
            ],
        );

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_fill_gaps.json");

        let mut app = App::with_path(path.clone());
        app.data.tasks = tasks;
        app.current_date = date;

        app.fill_gaps("Special task".to_string());

        let tasks = app.data.tasks.get(&date_str).unwrap();

        // Expected order:
        // 08:00 Task 1 10m
        // 08:10 Task 2 1h (ends 09:10)
        // 09:10 Special task 4h 20m (13:30 - 09:10 = 260 mins = 4h 20m)
        // 01:30 PM Task 3 10m (ends 01:40 PM)
        // 01:40 PM Special task 2h 10m (03:50 PM - 01:40 PM = 130 mins = 2h 10m)
        // 03:50 PM Task 4 10m

        assert_eq!(tasks.len(), 6);

        assert_eq!(tasks[2].name, "Special task");
        assert_eq!(tasks[2].start_time, "09:10 AM");
        assert_eq!(tasks[2].duration_mins, 260);

        assert_eq!(tasks[4].name, "Special task");
        assert_eq!(tasks[4].start_time, "01:40 PM");
        assert_eq!(tasks[4].duration_mins, 130);

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_fill_gaps_trailing() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let date_str = date.to_string();
        let mut tasks = HashMap::new();
        tasks.insert(
            date_str.clone(),
            vec![TaskRecord {
                id: "1".to_string(),
                name: "Task 1".to_string(),
                duration_mins: 60,
                start_time: "08:00 AM".to_string(),
            }],
        );

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_fill_gaps_trailing.json");

        let mut app = App::with_path(path.clone());
        app.data.tasks = tasks;
        app.current_date = date;

        app.fill_gaps("Gap Filler".to_string());

        let tasks = app.data.tasks.get(&date_str).unwrap();

        // Expected:
        // 08:00 AM Task 1 60m (ends 09:00 AM)
        // 09:00 AM Gap Filler 7h (420 mins) (ends 04:00 PM)

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[1].name, "Gap Filler");
        assert_eq!(tasks[1].start_time, "09:00 AM");
        assert_eq!(tasks[1].duration_mins, 420);

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_fill_gaps_empty_day() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let date_str = date.to_string();

        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_fill_gaps_empty_day.json");

        let mut app = App::with_path(path.clone());
        app.current_date = date;

        app.fill_gaps("Whole Day".to_string());

        let tasks = app.data.tasks.get(&date_str).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "Whole Day");
        assert_eq!(tasks[0].start_time, "08:00 AM");
        assert_eq!(tasks[0].duration_mins, WORK_DAY_DURATION_MINS);

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_copy_task_fills_selected_gap() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        let date_str = date.to_string();
        let path = std::env::temp_dir().join("test_copy_task_fills_selected_gap.json");
        let mut app = App::with_path(path.clone());
        app.current_date = date;
        app.data.tasks.insert(
            date_str.clone(),
            vec![
                TaskRecord {
                    id: "source".to_string(),
                    name: "Project work".to_string(),
                    duration_mins: 60,
                    start_time: "08:00 AM".to_string(),
                },
                TaskRecord {
                    id: "later".to_string(),
                    name: "Meeting".to_string(),
                    duration_mins: 60,
                    start_time: "10:00 AM".to_string(),
                },
            ],
        );

        app.enter_copy_mode();
        assert_eq!(app.copy_stage, CopyStage::SelectSource);
        app.confirm_copy_selection();
        assert_eq!(app.copy_stage, CopyStage::SelectDestination);
        assert_eq!(app.empty_slots()[0].duration_mins, 60);
        app.confirm_copy_selection();

        let tasks = app.data.tasks.get(&date_str).unwrap();
        let copied = tasks
            .iter()
            .find(|task| task.id != "source" && task.name == "Project work")
            .unwrap();
        assert_eq!(copied.start_time, "09:00 AM");
        assert_eq!(copied.duration_mins, 60);
        assert_ne!(copied.id, "source");
        assert_eq!(app.copy_stage, CopyStage::Inactive);

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn test_copy_task_can_select_later_source_and_gap() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 6).unwrap();
        let date_str = date.to_string();
        let path = std::env::temp_dir().join("test_copy_task_can_select_later_source_and_gap.json");
        let mut app = App::with_path(path.clone());
        app.current_date = date;
        app.data.tasks.insert(
            date_str.clone(),
            vec![
                TaskRecord {
                    id: "first".to_string(),
                    name: "First".to_string(),
                    duration_mins: 30,
                    start_time: "08:30 AM".to_string(),
                },
                TaskRecord {
                    id: "second".to_string(),
                    name: "Second".to_string(),
                    duration_mins: 30,
                    start_time: "10:00 AM".to_string(),
                },
            ],
        );

        app.enter_copy_mode();
        app.move_copy_selection_next();
        app.confirm_copy_selection();
        app.move_copy_selection_next();
        app.confirm_copy_selection();

        let copied = app
            .data
            .tasks
            .get(&date_str)
            .unwrap()
            .iter()
            .find(|task| task.id != "second" && task.name == "Second")
            .unwrap();
        assert_eq!(copied.start_time, "09:00 AM");
        assert_eq!(copied.duration_mins, 60);

        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }
}
