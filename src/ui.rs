use crate::app::{App, CopyStage};
use crate::models::mins_to_time_string;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main
            Constraint::Length(3), // Input box
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    // Header
    let total_mins = app.get_total_duration_mins();
    let h = total_mins / 60;
    let m = total_mins % 60;
    let date_str = format!("{} | {}h {}m", app.current_date.format("%a %d %b %Y"), h, m);
    let header = Paragraph::new(date_str)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Day Planner"));
    f.render_widget(header, chunks[0]);

    // Main
    let main_backdrop = Block::default().borders(Borders::ALL);
    f.render_widget(main_backdrop, chunks[1]);

    let main_area = chunks[1].inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(main_area);

    // Tasks and Gaps on the Right
    let date_str_key = app.current_date.to_string();
    let mut tasks = app
        .data
        .tasks
        .get(&date_str_key)
        .cloned()
        .unwrap_or_default();

    // Sort tasks by start time
    let start_mins = app.data.get_start_mins(&date_str_key);
    let end_mins = app.data.get_end_mins(&date_str_key);
    tasks.sort_by_key(|t| t.start_mins_relative_to(start_mins));

    let total_height = main_chunks[1].height;

    // We'll build a list of "DisplayItems" which are either Tasks or Gaps
    #[derive(Clone)]
    enum DisplayItem {
        Task(crate::models::TaskRecord),
        Gap { start_mins: i32, duration_mins: u32 },
    }

    let mut display_items = Vec::new();
    let mut current_mins = start_mins;

    for task in &tasks {
        let task_start = task.start_mins_since_midnight();
        if task_start > current_mins {
            display_items.push(DisplayItem::Gap {
                start_mins: current_mins,
                duration_mins: (task_start - current_mins) as u32,
            });
        }
        display_items.push(DisplayItem::Task(task.clone()));
        current_mins = current_mins.max(task_start + task.duration_mins as i32);
    }

    if current_mins < end_mins {
        display_items.push(DisplayItem::Gap {
            start_mins: current_mins,
            duration_mins: (end_mins - current_mins) as u32,
        });
    }

    let mut constraints = Vec::new();
    let total_dur = (end_mins - start_mins) as u32;
    for item in &display_items {
        let duration = match item {
            DisplayItem::Task(t) => t.duration_mins,
            DisplayItem::Gap { duration_mins, .. } => *duration_mins,
        };
        let h = (duration as f64 / total_dur as f64 * total_height as f64).round() as u16;
        let constraint = match item {
            DisplayItem::Task(_) => Constraint::Min(h.max(1)),
            DisplayItem::Gap { .. } => Constraint::Min(h),
        };
        constraints.push(constraint);
    }

    // Split for both timeline labels and tasks to ensure they align
    let task_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.clone())
        .split(main_chunks[1]);

    let timeline_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(main_chunks[0]);

    // Render Timeline and Tasks
    let mut last_label_y = 0;
    let mut gap_index = 0;
    let timeline_end_y = main_chunks[0].y + main_chunks[0].height.saturating_sub(1);

    for (i, item) in display_items.iter().enumerate() {
        let chunk = timeline_chunks[i];
        let task_chunk = task_chunks[i];

        // Render Label on the Left
        if chunk.height > 0 && chunk.y < timeline_end_y && (i == 0 || chunk.y >= last_label_y + 1) {
            let item_start_mins = match item {
                DisplayItem::Task(task) => task.start_mins_since_midnight(),
                DisplayItem::Gap { start_mins, .. } => *start_mins,
            };
            let time_label = format!(" {}", mins_to_time_string(item_start_mins));
            let p = Paragraph::new(time_label).style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, chunk);
            last_label_y = chunk.y;
        }

        // Render Content on the Right
        match item {
            DisplayItem::Task(task) => {
                let content_chunk = task_chunk;
                if content_chunk.height > 0 {
                    let h = task.duration_mins / 60;
                    let m = task.duration_mins % 60;
                    let title = format!(" [{}] {} ({}h {}m) ", task.start_time, task.name, h, m);

                    let task_index = tasks.iter().position(|t| t.id == task.id);
                    let is_selected = (app.edit_mode || app.copy_stage == CopyStage::SelectSource)
                        && app.selected_task_index == task_index;
                    let is_delete_selected =
                        app.delete_mode && app.selected_task_index == task_index;
                    let is_copy_source = app.copy_stage == CopyStage::SelectDestination
                        && app.copy_source_task_id.as_deref() == Some(task.id.as_str());

                    let block = if content_chunk.height == 1 {
                        Block::default().borders(Borders::TOP).title(title).style(
                            if is_delete_selected {
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                            } else if is_selected {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else if is_copy_source {
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(Color::Cyan)
                            },
                        )
                    } else {
                        let mut block = Block::default()
                            .borders(Borders::ALL)
                            .title(title)
                            .style(Style::default().bg(Color::Rgb(40, 40, 60)));

                        if is_delete_selected {
                            block = block.border_style(
                                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                            );
                        } else if is_selected {
                            block = block.border_style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            );
                        } else if is_copy_source {
                            block = block.border_style(
                                Style::default()
                                    .fg(Color::Green)
                                    .add_modifier(Modifier::BOLD),
                            );
                        }
                        block
                    };
                    f.render_widget(block, content_chunk);
                }
            }
            DisplayItem::Gap { duration_mins, .. } => {
                if task_chunk.height > 0 {
                    let h = duration_mins / 60;
                    let m = duration_mins % 60;
                    let title = format!(" GAP: {}h {}m ", h, m);
                    let is_selected = app.copy_stage == CopyStage::SelectDestination
                        && app.selected_gap_index == Some(gap_index);
                    let gap_block = Block::default()
                        .borders(Borders::ALL)
                        .style(if is_selected {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::DIM)
                        })
                        .title(title);
                    f.render_widget(gap_block, task_chunk);
                }
                gap_index += 1;
            }
        }
    }

    if main_chunks[0].height > 0 {
        let end_label = Paragraph::new(format!(" {}", mins_to_time_string(end_mins)))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(
            end_label,
            Rect::new(main_chunks[0].x, timeline_end_y, main_chunks[0].width, 1),
        );
    }

    // Input Box
    let input_title = if app.delete_mode {
        "Delete: select a record with arrows, then press Enter (Esc to cancel)"
    } else if app.copy_stage == CopyStage::SelectSource {
        "Copy: select a record with arrows, then press Enter (Esc to cancel)"
    } else if app.copy_stage == CopyStage::SelectDestination {
        if app.selected_gap_index.is_some() {
            "Copy: select an empty time slot with arrows, then press Enter"
        } else {
            "Copy: no empty time slots available (Esc to cancel)"
        }
    } else if app.edit_mode {
        "Edit record (Arrows to cycle, Enter to save, Esc to cancel)"
    } else if app.fill_mode {
        "Fill all gaps with task name (Enter to confirm, Esc to cancel)"
    } else if app.input_mode {
        "Enter record (e.g. Task 1h 30m 15:00 or 03:00 PM) - Press Esc to cancel"
    } else {
        "Press Enter to add | Press E to edit | Press C to copy | Press D to delete | Press F to fill gaps"
    };

    let input_paragraph = Paragraph::new(app.input.value())
        .style(match app.input_mode || app.edit_mode || app.fill_mode {
            true => Style::default().fg(Color::Yellow),
            false => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title(input_title));
    f.render_widget(input_paragraph, chunks[2]);
    if app.input_mode || app.edit_mode || app.fill_mode {
        f.set_cursor_position((
            chunks[2].x + app.input.visual_cursor() as u16 + 1,
            chunks[2].y + 1,
        ));
    }

    // Footer
    let footer_text = "<Left/Right>: Date | <Enter>: Add | <E>: Edit | <C>: Copy | <D>: Delete | <F>: Fill Gaps | <Q>: Quit";
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskRecord;
    use chrono::NaiveDate;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn timeline_displays_earliest_start_and_extended_end() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let date_key = date.to_string();
        let mut app = App::with_path(PathBuf::from("unused-test-data.json"));
        app.current_date = date;
        app.data.tasks = HashMap::from([(
            date_key,
            vec![TaskRecord {
                id: "long-day".to_string(),
                name: "Long day".to_string(),
                duration_mins: 8 * 60 + 30,
                start_time: "07:30 AM".to_string(),
            }],
        )]);

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, &app)).unwrap();

        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(output.contains("07:30 AM"));
        assert!(output.contains("04:00 PM"));
        assert!(output.contains("Long day"));
        assert!(!output.contains("[ / ]"));
    }
}
