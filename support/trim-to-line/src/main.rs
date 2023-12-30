use std::{
    env,
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
};

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!(
            "trim-to-line expects three arguments, received {}:",
            args.len() - 1
        );
        println!("Usage: trim-to-line [input_file.log] [output_file.log] [line_number]");
        return Ok(());
    }

    let input_file_path = &args[1];
    let output_file_path = &args[2];
    let trim_line_number = &args[3]
        .parse::<usize>()
        .expect("Could not parse line number");

    let input_file = File::open(input_file_path)?;
    let mut input_reader = BufReader::new(input_file);

    let mut output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(output_file_path)
        .unwrap();

    let mut line = String::new();
    let mut line_number = 1;
    let mut start_saving = false;

    let mut input_file_result = input_reader.read_line(&mut line);

    while let Ok(length) = input_file_result.as_ref() {
        if *length == 0 {
            break;
        }

        if !start_saving && line_number == *trim_line_number {
            // This line onwards should be saved
            start_saving = true;
        }

        if start_saving {
            output_file.write(line.as_bytes())?;
        }

        line_number += 1;

        line.clear();
        input_file_result = input_reader.read_line(&mut line);
    }

    println!("Wrote {} lines", line_number - trim_line_number);

    Ok(())
}
