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
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(chunks[1].inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        }));

    let main_backdrop = Block::default().borders(Borders::ALL);
    f.render_widget(main_backdrop, chunks[1]);

    // Timeline on the Left
    let timeline_text = " 08:00\n\n\n 09:00\n\n\n 10:00\n\n\n 11:00\n\n\n 12:00\n\n\n 13:00\n\n\n 14:00\n\n\n 15:00\n\n\n 16:00";
    let timeline_p = Paragraph::new(timeline_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(timeline_p, main_chunks[0]);

    // Tasks and Gaps on the Right
    let date_str_key = app.current_date.to_string();
    let tasks = app
        .data
        .tasks
        .get(&date_str_key)
        .cloned()
        .unwrap_or_default();
    let total_spent: u32 = tasks.iter().map(|t| t.duration_mins).sum();

    // 8h to 16h = 8 * 60 = 480 mins
    let mut constraints: Vec<Constraint> = tasks
        .iter()
        .map(|t| Constraint::Ratio(t.duration_mins, 480))
        .collect();

    let gap = 480_u32.saturating_sub(total_spent);
    if gap > 0 {
        constraints.push(Constraint::Ratio(gap, 480));
    }

    let task_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(main_chunks[1]);

    for (i, task) in tasks.iter().enumerate() {
        let h = task.duration_mins / 60;
        let m = task.duration_mins % 60;
        let title = format!(" [{}] {} ({}h {}m) ", task.start_time, task.name, h, m);
        let block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Rgb(40, 40, 60)))
            .title(title);
        f.render_widget(block, task_chunks[i]);
    }

    if gap > 0 {
        let h = gap / 60;
        let m = gap % 60;
        let title = format!(" GAP: Time left {}h {}m ", h, m);
        let gap_block = Block::default()
            .borders(Borders::ALL)
            .style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            )
            .title(title);
        f.render_widget(gap_block, task_chunks[tasks.len()]);
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
