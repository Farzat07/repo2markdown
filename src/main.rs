use std::{
    ffi::OsStr,
    io::{Read, Write},
    os::unix::ffi::OsStrExt,
    path::Path,
};

use repo2markdown::{normalizer::Normalizer, renderer::render};

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
    let normalizer = Normalizer::new(Path::new("."), Path::new("."))?;

    for segment in buf.split(|b| *b == 0) {
        if segment.is_empty() {
            continue;
        }

        let path = Path::new(OsStr::from_bytes(segment));
        let path = normalizer.normalize(path)?;
        let bytes = std::fs::read(&path)?;

        owned.push((path, bytes));
    }

    // convert to expected renderer input
    let refs: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(p, b)| (p.to_str().unwrap(), b.as_slice()))
        .collect();

    let rendered = render("Project name", &refs)?;
    output.write_all(rendered.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
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

    #[test]
    fn cli_reads_multiple_files_in_order() {
        fs::write("a.rs", "A").unwrap();
        fs::write("b.rs", "B").unwrap();

        let input = Cursor::new(b"a.rs\0b.rs\0");
        let mut output = Vec::new();

        run(input, &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();

        let a_pos = output.find("a.rs").unwrap();
        let b_pos = output.find("b.rs").unwrap();

        assert!(a_pos < b_pos);

        fs::remove_file("a.rs").unwrap();
        fs::remove_file("b.rs").unwrap();
    }

    #[test]
    fn cli_normalizes_paths_before_rendering() {
        fs::create_dir_all("test").unwrap();
        fs::write("test/main.rs", "fn main() {}").unwrap();

        let input = Cursor::new(b"test/./main.rs\0");
        let mut output = Vec::new();

        run(input, &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("### test/main.rs"));

        fs::remove_file("test/main.rs").unwrap();
        fs::remove_dir("test").unwrap();
    }
}
