use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
};

use itertools::{EitherOrBoth, Itertools};
use state::{DiffPosition, State};
use ui::build_app;

mod state;
mod string;
mod ui;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("trace-log-comparer expects two arguments, one for each file.");
        println!("Received {} arguments.", args.len());
        return Ok(());
    }

    let file1_path = &args[1];
    let file2_path = &args[2];

    let mut file1_reader = buf_reader(file1_path).expect("Could not open file 1");
    let mut file2_reader = buf_reader(file2_path).expect("Could not open file 2");

    let mut line_index = 0;

    let mut file1_line_positions = Vec::new();
    let mut file2_line_positions = Vec::new();

    let mut first_diff_positions = None;

    let mut line1 = String::new();
    let mut line2 = String::new();

    let mut file1_offset = 0;
    let mut file2_offset = 0;

    let mut extra_line_count = 20;

    let mut file1_result = file1_reader.read_line(&mut line1);
    let mut file2_result = file2_reader.read_line(&mut line2);

    while let (Ok(line1_length), Ok(line2_length)) = (file1_result.as_ref(), file2_result.as_ref())
    {
        if *line1_length == 0 || *line2_length == 0 {
            if extra_line_count > 0 {
                // Load extra lines after the end of the shorter file
                extra_line_count -= 1;
            } else {
                break;
            }
        }

        if line1 != line2 {
            if first_diff_positions.is_none() {
                let find_offset = || -> usize {
                    for (offset, combined_chars) in
                        line1.chars().zip_longest(line2.chars()).enumerate()
                    {
                        match combined_chars {
                            EitherOrBoth::Both(char1, char2) => {
                                if char1 != char2 {
                                    return offset;
                                }
                            }
                            EitherOrBoth::Left(_) | EitherOrBoth::Right(_) => {
                                return offset;
                            }
                        }
                    }

                    return 0;
                };

                first_diff_positions = Some(DiffPosition {
                    line_index,
                    line_offset: find_offset(),
                    file1_offset,
                    file2_offset,
                });
            }
        }

        if *line1_length > 0 {
            file1_line_positions.push(file1_offset);
        }

        if *line2_length > 0 {
            file2_line_positions.push(file2_offset);
        }

        file1_offset += line1_length;
        file2_offset += line2_length;

        line_index += 1;

        line1.clear();
        line2.clear();

        file1_result = file1_reader.read_line(&mut line1);
        file2_result = file2_reader.read_line(&mut line2);
    }

    let line1_length = if let Ok(length) = file1_result {
        length
    } else {
        0
    };

    let line2_length = if let Ok(length) = file2_result {
        length
    } else {
        0
    };

    if line1_length == 0 && line2_length == 0 {
        println!("Both files are the same length");
    } else if line1_length == 0 {
        println!("File 2 is longer");
    } else {
        println!("File 1 is longer");
    }

    build_app(State::new(
        first_diff_positions,
        file1_line_positions,
        file2_line_positions,
        file1_reader,
        file2_reader,
    ))?;

    Ok(())
}

fn buf_reader<'a, P>(filename: P) -> io::Result<BufReader<File>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file))
}
