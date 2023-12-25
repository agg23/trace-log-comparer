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
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

use crate::state::{DiffSection, State};
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

    let diffs = state.calculate_diffs(&file1_lines, &file2_lines);

    let (file1_spans, file2_spans) = build_spans(&diffs, 0);
    let (mut file1_lines, mut file2_lines) = build_lines(&file1_spans, &file2_spans, 0);

    let mut ui_state = UIState {
        list_state: ListState::default(),
        horizontal_offset: 0,
    };

    ui_state.list_state.select(Some(selected_line));

    let mut last_keycode: Option<KeyCode> = None;
    let mut key_repeat_count = 0;

    loop {
        terminal.draw(|f| {
            // let size = f.size();
            // let block = Block::default().title("Block").borders(Borders::ALL);
            // f.render_widget(block, size);

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let list1 = List::new(file1_lines.clone())
                .block(Block::default().borders(Borders::ALL).title("File 1"))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(list1, chunks[0], &mut ui_state.list_state);

            let list2 = List::new(file2_lines.clone())
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
                        let min_line_length = if longest_line_length > 10 {
                            longest_line_length - 10
                        } else {
                            0
                        };

                        if ui_state.horizontal_offset + horizontal_step_size < min_line_length {
                            ui_state.horizontal_offset += horizontal_step_size;

                            (file1_lines, file2_lines) =
                                build_lines(&file1_spans, &file2_spans, ui_state.horizontal_offset);
                        }
                    }
                    KeyCode::Left => {
                        if ui_state.horizontal_offset >= horizontal_step_size {
                            ui_state.horizontal_offset -= horizontal_step_size;
                        } else {
                            ui_state.horizontal_offset = 0;
                        }

                        (file1_lines, file2_lines) =
                            build_lines(&file1_spans, &file2_spans, ui_state.horizontal_offset);
                    }
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

fn build_spans<'a>(
    diffs: &'a Vec<Vec<DiffSection>>,
    horizontal_offset: usize,
) -> (Vec<Spans<'_>>, Vec<Spans<'_>>) {
    let mapper = |l: &String| -> ListItem<'a> {
        let string = if horizontal_offset >= l.len() && l.len() > 0 {
            // String would be offscreen and out of range. Indicate it's offscreen
            "<==".to_string()
        } else {
            l.as_str().slice(horizontal_offset..).to_string()
        };

        ListItem::new(string)
    };

    let (file1_spans, file2_spans): (Vec<Spans<'_>>, Vec<Spans<'_>>) = diffs
        .iter()
        .map(|line_diffs| {
            let mut line1 = Spans::default();
            let mut line2 = Spans::default();

            for diff in line_diffs.iter() {
                match diff {
                    DiffSection::Added(string) => line2
                        .0
                        .push(Span::styled(string, Style::default().bg(Color::Green))),
                    DiffSection::Modified { left, right } => {
                        line1.0.push(Span::styled(
                            left,
                            Style::default().add_modifier(Modifier::BOLD),
                        ));

                        line2.0.push(Span::styled(
                            right,
                            Style::default().add_modifier(Modifier::BOLD),
                        ));
                    }
                    DiffSection::Same(string) => {
                        let span = Span::raw(string);

                        line1.0.push(span.clone());
                        line2.0.push(span);
                    }
                    DiffSection::Removed(string) => line1
                        .0
                        .push(Span::styled(string, Style::default().bg(Color::Red))),
                }
            }

            (line1, line2)
        })
        .unzip();

    (file1_spans, file2_spans)
}

fn build_lines<'a>(
    file1_spans: &Vec<Spans<'a>>,
    file2_spans: &Vec<Spans<'a>>,
    horizontal_offset: usize,
) -> (Vec<ListItem<'a>>, Vec<ListItem<'a>>) {
    let add_left_placeholder = |spans: Spans<'a>| -> ListItem<'a> {
        let mut string = spans;

        if string.width() == 0 {
            string = Spans::from(Span::styled(
                "<==",
                Style::default().add_modifier(Modifier::DIM),
            ));
        }

        ListItem::new(string)
    };

    let file1_lines = file1_spans
        .iter()
        .map(|spans| add_left_placeholder(spans_substring(spans.clone(), horizontal_offset)))
        .collect();

    let file2_lines = file2_spans
        .iter()
        .map(|spans| add_left_placeholder(spans_substring(spans.clone(), horizontal_offset)))
        .collect();

    (file1_lines, file2_lines)
}

fn spans_substring<'a>(spans: Spans<'a>, horizontal_offset: usize) -> Spans<'a> {
    let mut required_offset = horizontal_offset;

    let spans: Vec<Span<'_>> = spans
        .0
        .into_iter()
        .filter_map(|span| {
            if required_offset == 0 {
                // Consume this span
                Some(span)
            } else if required_offset < span.width() {
                // Offset is within this span
                let text = span.content.slice(required_offset..).to_string().clone();
                required_offset = 0;
                Some(Span::styled(text, span.style.clone()))
            } else {
                // Offset is not within this span. Skip it
                required_offset -= span.width();
                None
            }
        })
        .collect();

    Spans::from(spans)
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
