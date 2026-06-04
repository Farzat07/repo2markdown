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
    use std::fs;
    use std::io::Cursor;
    use std::path::Path;

    use tempfile::tempdir;

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
        let temp_dir = tempdir().unwrap();
        let input = Cursor::new(b"");
        let mut output = Vec::new();
        let root = temp_dir.path();
        let origin_base = temp_dir.path();

        run(input, &mut output, &root, &origin_base).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "# Project name\n");
    }

    #[test]
    fn cli_reads_single_file_from_stdin() {
        let temp_dir = tempdir().unwrap();
        let origin_base = temp_dir.path();
        let input = Cursor::new(b"test_main.rs\0");
        let mut output = Vec::new();
        let root = temp_dir.path();

        fs::write(origin_base.join("test_main.rs"), "fn main() {}").unwrap();

        run(input, &mut output, root, origin_base).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("### \"test_main.rs\""));
        assert!(output_str.contains("fn main() {}"));
    }

    #[test]
    fn cli_reads_multiple_files_in_order() {
        let temp_dir = tempdir().unwrap();
        let origin_base = temp_dir.path();
        let input = Cursor::new(b"a.rs\0b.rs\0");
        let mut output = Vec::new();
        let root = temp_dir.path();

        fs::write(origin_base.join("a.rs"), "A").unwrap();
        fs::write(origin_base.join("b.rs"), "B").unwrap();

        run(input, &mut output, root, origin_base).unwrap();

        let output = String::from_utf8(output).unwrap();

        let a_pos = output.find("a.rs").unwrap();
        let b_pos = output.find("b.rs").unwrap();

        assert!(a_pos < b_pos);
    }

    #[test]
    fn cli_normalizes_paths_before_rendering() {
        let temp_dir = tempdir().unwrap();
        let origin_base = temp_dir.path();
        let input = Cursor::new(b"test/./main.rs\0");
        let mut output = Vec::new();
        let root = temp_dir.path();

        let write_dir = temp_dir.path().join("test");
        fs::create_dir_all(&write_dir).unwrap();
        fs::write(write_dir.join("main.rs"), "fn main() {}").unwrap();

        run(input, &mut output, root, origin_base).unwrap();

        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("### \"test/main.rs\""));
    }

    #[test]
    fn cli_reads_from_origin_but_outputs_relative_to_root() {
        let temp_dir = tempdir().unwrap();
        let origin_base = temp_dir.path().join("sandbox/src");
        let input = Cursor::new(b"main.rs\0");
        let mut output = Vec::new();
        let root = temp_dir.path().join("project");

        fs::create_dir_all(&origin_base).unwrap();
        fs::write(origin_base.join("main.rs"), "fn main() {}").unwrap();

        run(input, &mut output, &root, &origin_base).unwrap();

        let output = String::from_utf8(output).unwrap();

        // Must contain file content → proves correct reading
        assert!(output.contains("fn main() {}"));

        // Must contain normalized path → proves normalization applied
        assert!(output.contains("sandbox/src/main.rs"));
    }

    #[test]
    fn cli_ignores_origin_when_input_path_is_absolute() {
        let temp_dir1 = tempdir().unwrap();
        let temp_dir2 = tempdir().unwrap();
        let origin_base = temp_dir2.path();
        let filepath = temp_dir1.path().join("test_main.rs");
        let input = Cursor::new(paths_to_null_sep_bytes(&[&filepath]));
        let mut output = Vec::new();
        let root = temp_dir2.path();
        fs::write(&filepath, "fn main() {}").unwrap();

        run(input, &mut output, &root, &origin_base).unwrap();

        let output = String::from_utf8(output).unwrap();

        // Must contain file content → proves correct reading
        assert!(output.contains("fn main() {}"));
    }
}
