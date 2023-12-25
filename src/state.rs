use std::{
    fs::File,
    io::{self, BufRead, BufReader, Seek, SeekFrom},
};

use itertools::{EitherOrBoth, Itertools};

pub struct State {
    pub first_diff: Option<DiffPosition>,

    pub file1_line_positions: Vec<usize>,
    pub file2_line_positions: Vec<usize>,

    pub file1_reader: BufReader<File>,
    pub file2_reader: BufReader<File>,
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

impl State {
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
            .zip(file2_lines)
            .map(|(line1, line2)| self.calculate_line_diffs(line1, line2))
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
}
