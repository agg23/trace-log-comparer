use std::{
    fs::File,
    io::{self, BufRead, BufReader, Seek, SeekFrom},
};

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
