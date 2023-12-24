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
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

use crate::state::State;
use crate::string::StringUtils;

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
    let mut selected_line = 0;
    let (file1_lines, file2_lines) = if let Some(diff) = &state.first_diff {
        selected_line = diff.line_index;

        state.get_lines_around_line(diff.line_index, 20)
    } else {
        state.get_lines_around_line(0, 20)
    };

    let longest_line_length = longest_line_length(&file1_lines, &file2_lines);

    let (mut file1_list_lines, mut file2_list_lines) = build_lists(&file1_lines, &file2_lines, 0);

    let mut ui_state = UIState {
        list_state: ListState::default(),
        horizontal_offset: 0,
    };

    ui_state.list_state.select(Some(selected_line));

    loop {
        terminal.draw(|f| {
            // let size = f.size();
            // let block = Block::default().title("Block").borders(Borders::ALL);
            // f.render_widget(block, size);

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let list1 = List::new(file1_list_lines.clone())
                .block(Block::default().borders(Borders::ALL).title("File 1"))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list1, chunks[0], &mut ui_state.list_state);

            let list2 = List::new(file2_list_lines.clone())
                .block(Block::default().borders(Borders::ALL).title("File 2"))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_widget(list2, chunks[1]);
        })?;

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Right => {
                        // TODO: Check upper bound
                        let min_line_length = if longest_line_length > 10 {
                            longest_line_length - 10
                        } else {
                            0
                        };

                        if ui_state.horizontal_offset < min_line_length {
                            ui_state.horizontal_offset += 1;

                            (file1_list_lines, file2_list_lines) =
                                build_lists(&file1_lines, &file2_lines, ui_state.horizontal_offset);
                        }
                    }
                    KeyCode::Left => {
                        if ui_state.horizontal_offset != 0 {
                            ui_state.horizontal_offset -= 1;

                            (file1_list_lines, file2_list_lines) =
                                build_lists(&file1_lines, &file2_lines, ui_state.horizontal_offset);
                        }
                    }
                    KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn build_lists<'a>(
    file1_lines: &Vec<String>,
    file2_lines: &Vec<String>,
    horizontal_offset: usize,
) -> (Vec<ListItem<'a>>, Vec<ListItem<'a>>) {
    let mapper = |l: &String| -> ListItem<'a> {
        let string = if horizontal_offset >= l.len() && l.len() > 0 {
            // String would be offscreen and out of range. Indicate it's offscreen
            "<==".to_string()
        } else {
            l.as_str().slice(horizontal_offset..).to_string()
        };

        ListItem::new(string)
    };

    let file1_lines: Vec<ListItem<'_>> = file1_lines.iter().map(mapper).collect();
    let file2_lines: Vec<ListItem<'_>> = file2_lines.iter().map(mapper).collect();

    (file1_lines, file2_lines)
}

fn longest_line_length(file1_lines: &Vec<String>, file2_lines: &Vec<String>) -> usize {
    let mut longest_length = 0;

    for line in file1_lines.iter().chain(file2_lines.iter()) {
        if line.len() > longest_length {
            longest_length = line.len();
        }
    }

    longest_length
}
