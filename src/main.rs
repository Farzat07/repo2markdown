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
    root: &Path,
    origin_base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;

    let mut owned = Vec::new();
    let normalizer = Normalizer::new(root, origin_base)?;

    for segment in buf.split(|b| *b == 0) {
        if segment.is_empty() {
            continue;
        }

        let path = Path::new(OsStr::from_bytes(segment));
        let normalized_path = normalizer.normalize(path)?;
        let bytes = std::fs::read(normalized_path.absolute)?;

        owned.push((normalized_path.relative, bytes));
    }

    // convert to expected renderer input
    let refs: Vec<(&Path, &[u8])> = owned
        .iter()
        .map(|(p, b)| (p.as_path(), b.as_slice()))
        .collect();

    let rendered = render("Project name", &refs)?;
    output.write_all(rendered.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;
    use std::{env, fs};

    use super::run;

    fn paths_to_null_sep_bytes(file_paths: &[&Path]) -> Vec<u8> {
        let mut output = Vec::new();
        for path in file_paths {
            output.extend(path.as_os_str().as_encoded_bytes());
            output.push(0);
        }
        output
    }

    #[test]
    fn cli_with_empty_input_produces_empty_project() {
        let input = Cursor::new(b"");
        let mut output = Vec::new();
        let root = Path::new(".");
        let origin_base = Path::new(".");

        run(input, &mut output, root, origin_base).unwrap();

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
        let root = Path::new(".");
        let origin_base = Path::new(".");

        run(input, &mut output, root, origin_base).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("### \"test_main.rs\""));
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
        let root = Path::new(".");
        let origin_base = Path::new(".");

        run(input, &mut output, root, origin_base).unwrap();

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
        let root = Path::new(".");
        let origin_base = Path::new(".");

        run(input, &mut output, root, origin_base).unwrap();

        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("### \"test/main.rs\""));

        fs::remove_file("test/main.rs").unwrap();
        fs::remove_dir("test").unwrap();
    }

    #[test]
    fn cli_reads_from_origin_but_outputs_relative_to_root() {
        fs::create_dir_all("sandbox/src").unwrap();
        fs::write("sandbox/src/main.rs", "fn main() {}").unwrap();

        // stdin provides path relative to origin_base
        let input = Cursor::new(b"main.rs\0");
        let mut output = Vec::new();
        let root = Path::new("project");
        let origin_base = Path::new("sandbox/src");

        run(input, &mut output, root, origin_base).unwrap();

        let output = String::from_utf8(output).unwrap();

        // Must contain file content → proves correct reading
        assert!(output.contains("fn main() {}"));

        // Must contain normalized path → proves normalization applied
        assert!(output.contains("sandbox/src/main.rs"));

        fs::remove_file("sandbox/src/main.rs").unwrap();
        fs::remove_dir("sandbox/src").unwrap();
        fs::remove_dir("sandbox").unwrap();
    }

    #[test]
    fn cli_ignores_origin_when_input_path_is_absolute() {
        let temp_dir = env::temp_dir();
        let filepath = temp_dir.join("test_main.rs");
        fs::create_dir_all(temp_dir).unwrap();
        fs::write(&filepath, "fn main() {}").unwrap();

        // stdin provides path relative to origin_base
        let input = Cursor::new(paths_to_null_sep_bytes(&[&filepath]));
        let mut output = Vec::new();
        let root = Path::new("project");
        let origin_base = Path::new("sandbox/src");

        run(input, &mut output, root, origin_base).unwrap();

        let output = String::from_utf8(output).unwrap();

        // Must contain file content → proves correct reading
        assert!(output.contains("fn main() {}"));

        fs::remove_file(filepath).unwrap();
    }
}
