use std::{
    io::{self, stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListState},
    Terminal,
};

use crate::state::State;

struct UIState {
    list_state: ListState,
    horizontal_offset: usize,
}

pub fn build_app(state: State) -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    run_app(&mut terminal, state)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut state: State) -> Result<(), io::Error> {
    let mut selected_line = state.selected_line;

    let mut ui_state = UIState {
        list_state: ListState::default(),
        horizontal_offset: 0,
    };

    ui_state.list_state.select(Some(selected_line));

    let mut last_keycode: Option<KeyCode> = None;
    let mut key_repeat_count = 0;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let list1 = List::new(state.file1_list_lines.clone())
                .block(Block::default().borders(Borders::ALL).title("File 1"))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list1, chunks[0], &mut ui_state.list_state);

            let list2 = List::new(state.file2_list_lines.clone())
                .block(Block::default().borders(Borders::ALL).title("File 2"))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list2, chunks[1], &mut ui_state.list_state);
        })?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let mut repeat = false;

                if let Some(code) = last_keycode {
                    if code == key.code {
                        if key_repeat_count < 5 {
                            key_repeat_count += 1;
                        } else {
                            repeat = true;
                        }
                    }
                }

                let horizontal_step_size = if repeat { 5 } else { 1 };

                match key.code {
                    KeyCode::Right => {
                        let min_line_length = if state.longest_line_length > 10 {
                            state.longest_line_length - 10
                        } else {
                            0
                        };

                        if ui_state.horizontal_offset + horizontal_step_size < min_line_length {
                            ui_state.horizontal_offset += horizontal_step_size;

                            state.build_lines(ui_state.horizontal_offset);
                        }
                    }
                    KeyCode::Left => {
                        if ui_state.horizontal_offset >= horizontal_step_size {
                            ui_state.horizontal_offset -= horizontal_step_size;
                        } else {
                            ui_state.horizontal_offset = 0;
                        }

                        state.build_lines(ui_state.horizontal_offset);
                    }
                    KeyCode::Down => {}
                    KeyCode::Esc => break,
                    _ => {}
                }

                last_keycode = Some(key.code.clone());
            } else {
                last_keycode = None;
                key_repeat_count = 0;
            }
        } else {
            // No event, kill repeat
            last_keycode = None;
            key_repeat_count = 0;
        }
    }

    Ok(())
}
