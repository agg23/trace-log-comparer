use std::{
    fs::File,
    io::{self, BufRead, BufReader, Seek, SeekFrom},
};

use itertools::{Diff, EitherOrBoth, Itertools};
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::ListItem,
};

use crate::string::StringUtils;

pub struct State<'a> {
    first_diff: Option<DiffPosition>,

    file1_line_positions: Vec<usize>,
    file2_line_positions: Vec<usize>,

    file1_reader: BufReader<File>,
    file2_reader: BufReader<File>,

    line_diffs: Vec<Vec<DiffSection>>,

    pub longest_line_length: usize,
    pub selected_line: usize,

    file1_spans: Vec<Spans<'a>>,
    file2_spans: Vec<Spans<'a>>,

    pub file1_list_lines: Vec<ListItem<'a>>,
    pub file2_list_lines: Vec<ListItem<'a>>,
}

pub struct DiffPosition {
    pub line_index: usize,
    pub file1_offset: usize,
    pub file2_offset: usize,
}

#[derive(Clone)]
pub enum DiffSection {
    Added(String),
    Modified { left: String, right: String },
    Same(String),
    Removed(String),
}

impl DiffSection {
    pub fn left_len(&self) -> usize {
        match self {
            DiffSection::Added(a) | DiffSection::Same(a) | DiffSection::Removed(a) => a.len(),
            DiffSection::Modified { left, right } => left.len(),
        }
    }
}

impl<'a> State<'a> {
    pub fn new(
        first_diff: Option<DiffPosition>,
        file1_line_positions: Vec<usize>,
        file2_line_positions: Vec<usize>,
        file1_reader: BufReader<File>,
        file2_reader: BufReader<File>,
    ) -> Self {
        State {
            first_diff,

            file1_line_positions,
            file2_line_positions,

            file1_reader,
            file2_reader,

            longest_line_length: 0,
            selected_line: 0,

            line_diffs: vec![],

            file1_spans: vec![],
            file2_spans: vec![],

            file1_list_lines: vec![],
            file2_list_lines: vec![],
        }
    }

    pub fn build_state(&mut self, lines_to_load: usize) {
        let (file1_raw_lines, file2_raw_lines) = if let Some(diff) = &self.first_diff {
            self.selected_line = diff.line_index;

            self.get_lines_around_line(diff.line_index, lines_to_load)
        } else {
            self.get_lines_around_line(0, lines_to_load)
        };

        self.longest_line_length = longest_line_length(&file1_raw_lines, &file2_raw_lines);

        self.line_diffs = self.calculate_diffs(&file1_raw_lines, &file2_raw_lines);

        let (file1_spans, file2_spans) = build_spans(&self.line_diffs);

        self.file1_spans = file1_spans;
        self.file2_spans = file2_spans;

        self.build_lines(0, 1);
    }

    pub fn build_lines(&mut self, horizontal_offset: usize, start_line_number: usize) {
        let (file1_list_lines, file2_list_lines) = build_lines(
            &self.file1_spans,
            &self.file2_spans,
            horizontal_offset,
            start_line_number,
        );

        self.file1_list_lines = file1_list_lines;
        self.file2_list_lines = file2_list_lines;
    }

    pub fn get_lines_around_line(
        &mut self,
        line_index: usize,
        line_count: usize,
    ) -> (Vec<String>, Vec<String>) {
        let bottom_line_index = (line_index as i32) - ((line_count / 2) as i32);

        let bottom_line_index = if bottom_line_index > 0 {
            bottom_line_index as usize
        } else {
            0
        };

        let top_line_index = bottom_line_index + line_count;

        let mut file1_lines = vec![];
        let mut file2_lines = vec![];

        for i in bottom_line_index..top_line_index {
            let line1_offset = self.file1_line_positions.get(i);
            if let Some(line1_offset) = line1_offset {
                let line1 = self
                    .read_line_at_offset(true, *line1_offset as u64)
                    .expect("Could not read line");
                file1_lines.push(line1);
            }

            let line2_offset = self.file2_line_positions.get(i);
            if let Some(line2_offset) = line2_offset {
                let line2 = self
                    .read_line_at_offset(false, *line2_offset as u64)
                    .expect("Could not read line");
                file2_lines.push(line2);
            }
        }

        (file1_lines, file2_lines)
    }

    pub fn calculate_diffs(
        &mut self,
        file1_lines: &Vec<String>,
        file2_lines: &Vec<String>,
    ) -> Vec<Vec<DiffSection>> {
        file1_lines
            .iter()
            .zip_longest(file2_lines)
            .map(|line| match line {
                EitherOrBoth::Both(line1, line2) => self.calculate_line_diffs(line1, line2),
                EitherOrBoth::Left(line1) => vec![DiffSection::Removed(line1.clone())],
                EitherOrBoth::Right(line2) => vec![DiffSection::Added(line2.clone())],
            })
            .collect()
    }

    fn calculate_line_diffs(&self, line1: &String, line2: &String) -> Vec<DiffSection> {
        let mut last_diff: Option<DiffSection> = None;

        let mut diffs: Vec<DiffSection> = vec![];

        let mut merge_diff = |last_diff: &mut Option<DiffSection>, new_diff: DiffSection| {
            if let Some(mut inner_last_diff) = last_diff.take() {
                match (&mut inner_last_diff, &new_diff) {
                    (DiffSection::Added(ref mut a), &DiffSection::Added(ref b))
                    | (DiffSection::Same(ref mut a), &DiffSection::Same(ref b))
                    | (DiffSection::Removed(ref mut a), &DiffSection::Removed(ref b)) => {
                        a.push_str(&b);
                        // We consumed the Option, so we have to re-place the value
                        *last_diff = Some(inner_last_diff);
                    }
                    (
                        DiffSection::Modified {
                            left: ref mut left_a,
                            right: ref mut right_a,
                        },
                        DiffSection::Modified {
                            left: ref left_b,
                            right: ref right_b,
                        },
                    ) => {
                        // Combine both sides
                        left_a.push_str(&left_b);
                        right_a.push_str(&right_b);
                        *last_diff = Some(inner_last_diff);
                    }
                    _ => {
                        // They don't match. Last diff is completed. Push new diff
                        diffs.push(inner_last_diff);
                        *last_diff = Some(new_diff);
                    }
                }
            } else {
                // Directly push new diff
                *last_diff = Some(new_diff);
            }
        };

        for combined_chars in line1.chars().zip_longest(line2.chars()) {
            match combined_chars {
                EitherOrBoth::Both(char1, char2) => {
                    if char1 == char2 {
                        merge_diff(&mut last_diff, DiffSection::Same(char1.to_string()));
                    } else {
                        merge_diff(
                            &mut last_diff,
                            DiffSection::Modified {
                                left: char1.to_string(),
                                right: char2.to_string(),
                            },
                        );
                    }
                }
                EitherOrBoth::Left(char) => {
                    merge_diff(&mut last_diff, DiffSection::Removed(char.to_string()))
                }
                EitherOrBoth::Right(char) => {
                    merge_diff(&mut last_diff, DiffSection::Added(char.to_string()))
                }
            }
        }

        if let Some(last_diff) = last_diff {
            diffs.push(last_diff);
        }

        diffs
    }

    fn read_line_at_offset(&mut self, file1: bool, offset: u64) -> io::Result<String> {
        let reader = if file1 {
            &mut self.file1_reader
        } else {
            &mut self.file2_reader
        };

        let mut buffer = String::new();

        reader.seek(SeekFrom::Start(offset))?;
        reader.read_line(&mut buffer)?;

        Ok(buffer)
    }

    pub fn find_next_diff(&self, match_line: usize, match_offset: usize) -> Option<(usize, usize)> {
        for (line_number, line_diffs) in self.line_diffs[match_line..].iter().enumerate() {
            // Make sure index is actually to the start of the lines
            let line_number = line_number + match_line;
            let mut line_offset = 0;

            for diff in line_diffs {
                match diff {
                    DiffSection::Added(_)
                    | DiffSection::Modified { left: _, right: _ }
                    | DiffSection::Removed(_) => {
                        if line_offset > match_offset || line_number > match_line {
                            // This is the next diff
                            return Some((line_number, line_offset));
                        }
                    }
                    _ => {}
                }

                line_offset += diff.left_len();
            }
        }

        None
    }

    pub fn find_prev_diff(&self, match_line: usize, match_offset: usize) -> Option<(usize, usize)> {
        for (line_number, line_diffs) in self.line_diffs[..match_line].iter().enumerate().rev() {
            let line_width = line_diffs
                .iter()
                .map(|diff| diff.left_len())
                .reduce(|acc, width| acc + width)
                .unwrap_or(0);

            let mut line_offset = line_width;

            for diff in line_diffs.iter().rev() {
                match diff {
                    DiffSection::Added(_)
                    | DiffSection::Modified { left: _, right: _ }
                    | DiffSection::Removed(_) => {
                        if line_offset < match_offset || line_number < match_line {
                            // This is the prev diff
                            return Some((line_number, line_offset));
                        }
                    }
                    _ => {}
                }

                line_offset -= diff.left_len();
            }
        }

        None
    }
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

fn build_spans<'a, 'b>(diffs: &'a Vec<Vec<DiffSection>>) -> (Vec<Spans<'b>>, Vec<Spans<'b>>) {
    diffs
        .iter()
        .map(|line_diffs| {
            let mut line1 = Spans::default();
            let mut line2 = Spans::default();

            for diff in line_diffs.iter() {
                match diff {
                    DiffSection::Added(string) => line2.0.push(Span::styled(
                        string.clone(),
                        Style::default().bg(Color::Rgb(0, 100, 0)),
                    )),
                    DiffSection::Modified { left, right } => {
                        line1.0.push(Span::styled(
                            left.clone(),
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .bg(Color::Blue),
                        ));

                        line2.0.push(Span::styled(
                            right.clone(),
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .bg(Color::Blue),
                        ));
                    }
                    DiffSection::Same(string) => {
                        let span = Span::raw(string.clone());

                        line1.0.push(span.clone());
                        line2.0.push(span);
                    }
                    DiffSection::Removed(string) => line1.0.push(Span::styled(
                        string.clone(),
                        Style::default().bg(Color::Red),
                    )),
                }
            }

            (line1, line2)
        })
        .unzip()
}

fn build_lines<'a>(
    file1_spans: &Vec<Spans<'a>>,
    file2_spans: &Vec<Spans<'a>>,
    horizontal_offset: usize,
    start_line_number: usize,
) -> (Vec<ListItem<'a>>, Vec<ListItem<'a>>) {
    let add_left_placeholder = |spans: Spans<'a>, original_length: usize| -> Spans<'a> {
        if original_length == 0 {
            // String was empty to begin with. EOF
            Spans::from(Span::styled(
                "EOF",
                Style::default().add_modifier(Modifier::DIM),
            ))
        } else if spans.width() == 0 {
            Spans::from(Span::styled(
                "<==",
                Style::default().add_modifier(Modifier::DIM),
            ))
        } else {
            spans
        }
    };

    let process_spans_into_lines = |spans: &Vec<Spans<'a>>| -> Vec<ListItem<'a>> {
        spans
            .iter()
            .enumerate()
            .map(|(index, spans)| {
                let original_length = spans.width();

                let mut spans = add_left_placeholder(
                    spans_substring(spans.clone(), horizontal_offset),
                    original_length,
                );

                let full_sized_number_string = format!("{} ", start_line_number + index);

                let number_string = if full_sized_number_string.len() <= 9 {
                    format!("{:8} ", start_line_number + index)
                } else {
                    full_sized_number_string
                };

                spans.0.insert(
                    0,
                    Span::styled(number_string, Style::default().add_modifier(Modifier::DIM)),
                );

                ListItem::new(spans)
            })
            .collect()
    };

    (
        process_spans_into_lines(file1_spans),
        process_spans_into_lines(file2_spans),
    )
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
