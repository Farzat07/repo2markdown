use std::{
    fmt,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum RenderError {
    BinaryFile(PathBuf),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BinaryFile(filename) => {
                write!(f, "Binary file encountered: {:?}", filename)
            }
        }
    }
}

impl std::error::Error for RenderError {}

pub struct Renderer<W: Write> {
    output: W,
}

impl<W: Write> Renderer<W> {
    pub fn new(output: W) -> Self {
        Self { output }
    }

    pub fn render_header(&mut self, project_name: &str) -> std::io::Result<()> {
        writeln!(self.output, "# {}", project_name)
    }
}

pub fn render(project_name: &str, files: &[(&Path, &[u8])]) -> Result<String, RenderError> {
    let mut output = format!("# {}\n", project_name);
    for (filename, bytes) in files {
        let printable_filename = render_filename(filename);
        let content = std::str::from_utf8(bytes)
            .map_err(|_| RenderError::BinaryFile(filename.to_path_buf()))?;
        let outer_backticks = outer_backticks(content);
        output.push_str(&format!(
            "\n## File: {}\n{}\n{}\n{}\n",
            printable_filename, outer_backticks, content, outer_backticks
        ));
    }
    Ok(output)
}

fn outer_backticks(contents: &str) -> String {
    let mut max_ticks = 0;
    let mut current_count = 0;
    for char in contents.chars() {
        if char == '`' {
            current_count += 1;
            if current_count > max_ticks {
                max_ticks = current_count;
            }
        } else {
            current_count = 0;
        }
    }
    let fence_len = std::cmp::max(3, max_ticks + 1);
    "`".repeat(fence_len)
}

fn render_filename(path: &Path) -> String {
    let s = format!("{:?}", path);
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(&s)
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

    use super::{RenderError, Renderer, render};

    #[test]
    fn empty_project_renders_only_title() {
        let output = render("Project name", &[]);
        assert_eq!(output.unwrap(), "# Project name\n");
    }

    #[test]
    fn single_file_is_rendered() {
        let files: Vec<(&Path, &[u8])> = vec![(Path::new("main.rs"), b"fn main() {}")];

        let output = render("Project name", &files);

        assert_eq!(
            output.unwrap(),
            "# Project name\n\n\
            ## File: main.rs\n\
            ```\n\
            fn main() {}\n\
            ```\n"
        );
    }

    #[test]
    fn multiple_files_are_rendered_in_order() {
        let files: Vec<(&Path, &[u8])> = vec![
            (Path::new("main.rs"), b"fn main() {}"),
            (Path::new("lib.rs"), b"pub fn hello() {}"),
        ];

        let output = render("Project name", &files);

        assert_eq!(
            output.unwrap(),
            "# Project name\n\n\
            ## File: main.rs\n\
            ```\n\
            fn main() {}\n\
            ```\n\n\
            ## File: lib.rs\n\
            ```\n\
            pub fn hello() {}\n\
            ```\n"
        );
    }

    #[test]
    fn file_with_backticks_is_handled_safely() {
        let files: Vec<(&Path, &[u8])> = vec![(
            Path::new("example.rs"),
            b"fn main() { println!(\"``` inside\"); }",
        )];

        let output = render("Project name", &files);

        assert_eq!(
            output.unwrap(),
            "# Project name\n\n\
            ## File: example.rs\n\
            ````\n\
            fn main() { println!(\"``` inside\"); }\n\
            ````\n"
        );
    }

    #[test]
    fn binary_file_is_rejected() {
        let files: Vec<(&Path, &[u8])> = vec![(Path::new("image.png"), &[0x00, 0x01, 0x02, 0xc3])];

        let result = render("Project name", &files);

        assert!(
            matches!(result, Err(RenderError::BinaryFile(name)) if name == Path::new("image.png"))
        );
    }

    #[test]
    fn filename_with_linebreaks_and_invalid_chars_handled_properly() {
        let files: Vec<(&Path, &[u8])> = vec![(
            Path::new(OsStr::from_bytes(b"jap\xE3\x81\x82dir/some\nma\xc3in.rs")),
            b"fn main() {}",
        )];

        let output = render("Project name", &files);

        assert_eq!(
            output.unwrap(),
            "# Project name\n\n\
            ## File: japあdir/some\\nma\\xC3in.rs\n\
            ```\n\
            fn main() {}\n\
            ```\n"
        );
    }

    #[test]
    fn renderer_writes_header() {
        let mut output = Vec::new();
        let mut renderer = Renderer::new(&mut output);

        renderer.render_header("Project name").unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "# Project name\n");
    }
}
