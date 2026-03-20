use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
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
    let date_str = app.current_date.format("%a %d %b %Y").to_string();
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
    tasks.sort_by_key(|t| t.start_mins_from_8am());

    let total_height = main_chunks[1].height;
    
    // We'll build a list of "DisplayItems" which are either Tasks or Gaps
    #[derive(Clone)]
    enum DisplayItem {
        Task(crate::models::TaskRecord),
        Gap(u32),
    }

    let mut display_items = Vec::new();
    let mut current_min: i32 = 0; // Starts at 8:00 AM

    for task in tasks {
        let start_min = task.start_mins_from_8am();
        if start_min > current_min {
            display_items.push(DisplayItem::Gap((start_min - current_min) as u32));
            current_min = start_min;
        }
        display_items.push(DisplayItem::Task(task.clone()));
        current_min += task.duration_mins as i32;
    }

    // Add trailing gap to reach 4:00 PM (480 mins) if necessary
    if current_min < 480 {
        display_items.push(DisplayItem::Gap((480 - current_min) as u32));
    }

    let mut constraints = Vec::new();
    for item in &display_items {
        let duration = match item {
            DisplayItem::Task(t) => t.duration_mins,
            DisplayItem::Gap(g) => *g,
        };
        constraints.push(Constraint::Ratio(duration, 480.max(current_min as u32)));
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
    let mut running_min = 0;
    let mut last_label_y = 0;

    for (i, item) in display_items.iter().enumerate() {
        let chunk = timeline_chunks[i];
        let task_chunk = task_chunks[i];

        // Render Label on the Left
        if chunk.height > 0 && (i == 0 || chunk.y >= last_label_y + 1) {
            let h = 8 + running_min / 60;
            let m = running_min % 60;
            let time_label = format!(" {:02}:{:02}", h, m);
            let p = Paragraph::new(time_label).style(Style::default().fg(Color::DarkGray));
            f.render_widget(p, chunk);
            last_label_y = chunk.y;
        }

        // Render Content on the Right
        match item {
            DisplayItem::Task(task) => {
                let mut content_chunk = task_chunk;
                if content_chunk.height == 0 && total_height > 0 {
                    content_chunk.height = 1;
                }

                if content_chunk.height > 0 {
                    let h = task.duration_mins / 60;
                    let m = task.duration_mins % 60;
                    let title = format!(" [{}] {} ({}h {}m) ", task.start_time, task.name, h, m);

                    let block = if content_chunk.height == 1 {
                        Block::default()
                            .borders(Borders::TOP)
                            .title(title)
                            .style(Style::default().fg(Color::Cyan))
                    } else {
                        Block::default()
                            .borders(Borders::ALL)
                            .title(title)
                            .style(Style::default().bg(Color::Rgb(40, 40, 60)))
                    };
                    f.render_widget(block, content_chunk);
                }
                running_min += task.duration_mins as i32;
            }
            DisplayItem::Gap(gap) => {
                if task_chunk.height > 0 {
                    let h = gap / 60;
                    let m = gap % 60;
                    let title = format!(" GAP: {}h {}m ", h, m);
                    let gap_block = Block::default()
                        .borders(Borders::ALL)
                        .style(
                            Style::default()
                                .fg(Color::DarkGray)
                                .add_modifier(Modifier::DIM),
                        )
                        .title(title);
                    f.render_widget(gap_block, task_chunk);
                }
                running_min += *gap as i32;
            }
        }
    }

    // Input Box
    let input_title = if app.input_mode {
        "Enter record (e.g. Task 1h 30m 10:00 AM) - Press Esc to cancel"
    } else {
        "Press Enter to add record"
    };

    let input_paragraph = Paragraph::new(app.input.value())
        .style(match app.input_mode {
            true => Style::default().fg(Color::Yellow),
            false => Style::default(),
        })
        .block(Block::default().borders(Borders::ALL).title(input_title));
    f.render_widget(input_paragraph, chunks[2]);
    if app.input_mode {
        f.set_cursor_position((
            chunks[2].x + app.input.visual_cursor() as u16 + 1,
            chunks[2].y + 1,
        ));
    }

    // Footer
    let footer_text = "<Left/Right>: Change Date | <Enter>: Add Task | <Q>: Quit";
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}
