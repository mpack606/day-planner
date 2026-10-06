mod app;
mod models;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{error::Error, io, time::Duration};
use tui_input::backend::crossterm::EventHandler;

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn Error>>
where
    B::Error: 'static,
{
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.is_copy_mode() {
                        match key.code {
                            KeyCode::Enter => app.confirm_copy_selection(),
                            KeyCode::Esc => app.cancel_copy_mode(),
                            KeyCode::Up | KeyCode::Left => app.move_copy_selection_previous(),
                            KeyCode::Down | KeyCode::Right => app.move_copy_selection_next(),
                            _ => {}
                        }
                    } else if app.input_mode || app.edit_mode || app.fill_mode {
                        match key.code {
                            KeyCode::Enter => {
                                app.handle_submit();
                            }
                            KeyCode::Esc => {
                                app.input_mode = false;
                                app.edit_mode = false;
                                app.fill_mode = false;
                                app.selected_task_index = None;
                                app.input.reset();
                            }
                            KeyCode::Up if app.edit_mode => {
                                app.move_selection_up();
                            }
                            KeyCode::Down if app.edit_mode => {
                                app.move_selection_down();
                            }
                            _ => {
                                app.input.handle_event(&Event::Key(key));
                            }
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                app.should_quit = true;
                            }
                            KeyCode::Left => {
                                app.previous_day();
                            }
                            KeyCode::Right => {
                                app.next_day();
                            }
                            KeyCode::Enter => {
                                app.input_mode = true;
                            }
                            KeyCode::Char('e') | KeyCode::Char('E') => {
                                app.enter_edit_mode();
                            }
                            KeyCode::Char('[') => {
                                app.shift_start_time(-30);
                            }
                            KeyCode::Char(']') => {
                                app.shift_start_time(30);
                            }
                            KeyCode::Char('f') | KeyCode::Char('F') => {
                                app.fill_mode = true;
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                app.enter_copy_mode();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
