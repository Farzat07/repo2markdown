use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    io::{self, Read, Write},
    os::unix::ffi::OsStrExt,
    path::Path,
};

use repo2markdown::{
    logger::{Logger, Verbosity},
    normalizer::Normalizer,
    renderer::Renderer,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);

    let mut root = None;
    let mut origin = None;
    let mut name = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next(),
            "--origin" => origin = args.next(),
            "--name" => name = args.next(),
            _ => {
                eprintln!("Unknown argument: {}", arg);
                std::process::exit(1);
            }
        }
    }

    let root = root
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new("."));

    let origin = origin
        .as_deref()
        .map(Path::new)
        .unwrap_or_else(|| Path::new("."));

    let stdin = io::stdin();
    let stdout = io::stdout();

    let logger = Logger::new(Verbosity::Normal);
    run(
        stdin.lock(),
        stdout.lock(),
        root,
        origin,
        name.as_deref(),
        logger,
    )
}

const DEFAULT_PROJECT_NAME: &str = "Project Outline";

pub fn run<R: Read, W: Write>(
    mut input: R,
    output: W,
    root: &Path,
    origin_base: &Path,
    project_name: Option<&str>,
    logger: Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;

    let normalizer = Normalizer::new(root, origin_base)?;

    let mut renderer = Renderer::new(output).with_logger(logger);
    let project_name = project_name.unwrap_or_else(|| derive_project_name(root));
    renderer.render_header(project_name)?;

    let mut seen_paths = HashSet::new();
    for segment in buf.split(|b| *b == 0) {
        if segment.is_empty() {
            continue;
        }

        let path = Path::new(OsStr::from_bytes(segment));
        let normalized_path = normalizer.normalize(path)?;
        if !seen_paths.insert(normalized_path.relative.clone()) {
            logger.warn(format!(
                "Duplicate file detected: {:?}",
                normalized_path.relative
            ));
            continue;
        }
        renderer.render_path(&normalized_path)?;
    }
    Ok(())
}

fn derive_project_name(root: &Path) -> &str {
    if let Some(os_str_name) = root.file_name()
        && let Some(name) = os_str_name.to_str()
    {
        name
    } else {
        DEFAULT_PROJECT_NAME
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::io::{Cursor, Read, Write};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    use repo2markdown::logger::Logger;
    use tempfile::tempdir;

    use super::{DEFAULT_PROJECT_NAME, derive_project_name, run};

    fn paths_to_null_sep_bytes(file_paths: &[&Path]) -> Vec<u8> {
        let mut output = Vec::new();
        for path in file_paths {
            output.extend(path.as_os_str().as_encoded_bytes());
            output.push(0);
        }
        output
    }

    fn run_with_default_logger<R: Read, W: Write>(
        input: R,
        output: W,
        root: &Path,
        origin_base: &Path,
        project_name: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let logger = Logger::default();
        run(input, output, root, origin_base, project_name, logger)
    }

    #[test]
    fn cli_with_empty_input_produces_empty_project_with_specified_project_name() {
        let temp_dir = tempdir().unwrap();
        let input = Cursor::new(b"");
        let mut output = Vec::new();
        let root = temp_dir.path();
        let origin_base = temp_dir.path();

        run_with_default_logger(input, &mut output, root, origin_base, Some("Project name"))
            .unwrap();

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

        run_with_default_logger(input, &mut output, root, origin_base, None).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("## File: test_main.rs"));
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

        run_with_default_logger(input, &mut output, root, origin_base, None).unwrap();

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

        run_with_default_logger(input, &mut output, root, origin_base, None).unwrap();

        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("## File: test/main.rs"));
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

        run_with_default_logger(input, &mut output, &root, &origin_base, None).unwrap();

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

        run_with_default_logger(input, &mut output, root, origin_base, None).unwrap();

        let output = String::from_utf8(output).unwrap();

        // Must contain file content → proves correct reading
        assert!(output.contains("fn main() {}"));
    }

    #[test]
    fn duplicate_files_in_sequence_are_skipped() {
        let temp_dir = tempdir().unwrap();
        let origin = temp_dir.path();
        let root = temp_dir.path();

        fs::write(origin.join("a.rs"), "A").unwrap();

        let input = Cursor::new(b"a.rs\0a.rs\0");
        let mut output = Vec::new();

        run_with_default_logger(input, &mut output, root, origin, None).unwrap();

        let output = String::from_utf8(output).unwrap();

        assert_eq!(output.matches("## File: a.rs").count(), 1);
    }

    #[test]
    fn duplicate_files_are_skipped_with_preserved_display_order_even_if_not_adjacent() {
        let temp_dir = tempdir().unwrap();
        let origin = temp_dir.path();
        let root = temp_dir.path();

        fs::write(origin.join("a.rs"), "A").unwrap();
        fs::write(origin.join("b.rs"), "B").unwrap();

        let input = Cursor::new(b"a.rs\0b.rs\0a.rs\0");
        let mut output = Vec::new();

        run_with_default_logger(input, &mut output, root, origin, None).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches("## File: a.rs").count(), 1);
        assert_eq!(output.matches("## File: b.rs").count(), 1);
        let a_pos = output.find("a.rs").unwrap();
        let b_pos = output.find("b.rs").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn project_name_is_derived_from_root_by_default_even_if_directory_does_not_exist() {
        let temp_dir = tempdir().unwrap();
        let origin_base = temp_dir.path();
        let input = Cursor::new(b"");
        let mut output = Vec::new();
        let root = temp_dir.path().join("repo2markdown");

        run_with_default_logger(input, &mut output, &root, origin_base, None).unwrap();

        let output_str = String::from_utf8(output).unwrap();

        assert_eq!(output_str, "# repo2markdown\n");
    }

    #[test]
    fn project_name_fallsback_to_default_if_root_is_filesystem_root() {
        assert_eq!(derive_project_name(Path::new("/")), DEFAULT_PROJECT_NAME);
    }

    #[test]
    fn project_name_fallsback_if_root_ending_is_not_utf8() {
        let root = Path::new(OsStr::from_bytes(b"/root/fd\xC3"));
        assert_eq!(derive_project_name(root), DEFAULT_PROJECT_NAME);
    }

    #[test]
    fn deriving_project_name_from_root_ignores_trailing_slash() {
        let root = Path::new("/root/repo2markdown/");
        assert_eq!(derive_project_name(root), "repo2markdown");
    }
}
