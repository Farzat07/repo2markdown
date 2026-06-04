use std::io::{Read, Write};

use repo2markdown::renderer::render;

fn main() {
    println!("Hello, world!");
}

pub fn run<R: Read, W: Write>(
    mut input: R,
    mut output: W,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;

    let mut owned = Vec::new();

    for segment in buf.split(|b| *b == 0) {
        if segment.is_empty() {
            continue;
        }

        let path = std::str::from_utf8(segment)?;
        let bytes = std::fs::read(path)?;

        owned.push((path.to_string(), bytes));
    }

    // convert to expected renderer input
    let refs: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_slice()))
        .collect();

    let rendered = render("Project name", &refs)?;
    output.write_all(rendered.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::env::temp_dir;
    use std::fs;
    use std::io::Cursor;

    #[test]
    fn cli_with_empty_input_produces_empty_project() {
        let input = Cursor::new(b"");
        let mut output = Vec::new();

        run(input, &mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "# Project name\n");
    }

    #[test]
    fn cli_reads_single_file_from_stdin() {
        // create a temporary file
        let path = "test_main.rs";
        fs::write(path, "fn main() {}").unwrap();

        // null-delimited input
        let input = Cursor::new(format!("{}\0", path).into_bytes());
        let mut output = Vec::new();

        run(input, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("### test_main.rs"));
        assert!(output_str.contains("fn main() {}"));

        // cleanup
        fs::remove_file(path).unwrap();
    }
}
