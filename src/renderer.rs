use std::{
    io::{Read, Write},
    path::Path,
};

#[derive(Debug)]
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

    pub fn render_file<R: Read>(&mut self, filename: &Path, mut reader: R) -> std::io::Result<()> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let contents = if let Ok(utf8string) = std::str::from_utf8(&bytes) {
            utf8string
        } else {
            return self.render_binary_file(filename);
        };
        let name = render_filename(filename);
        let fence = outer_backticks(contents);
        writeln!(self.output)?;
        writeln!(self.output, "## File: {}", name)?;
        writeln!(self.output, "{}", fence)?;
        writeln!(self.output, "{}", contents)?;
        writeln!(self.output, "{}", fence)
    }

    fn render_binary_file(&mut self, filename: &Path) -> std::io::Result<()> {
        let name = render_filename(filename);
        writeln!(self.output)?;
        writeln!(self.output, "## File: {}", name)?;
        writeln!(self.output, "[BINARY FILE]")
    }
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
    use std::{ffi::OsStr, io::Cursor, os::unix::ffi::OsStrExt, path::Path};

    use super::Renderer;

    #[test]
    fn renderer_writes_header() {
        let mut output = Vec::new();
        let mut renderer = Renderer::new(&mut output);

        renderer.render_header("Project name").unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "# Project name\n");
    }

    #[test]
    fn renderer_renders_single_file() {
        let mut output = Vec::new();
        let mut renderer = Renderer::new(&mut output);

        let input = Cursor::new("fn main() {}");
        renderer.render_file(Path::new("main.rs"), input).unwrap();
        let expected = "\n## File: main.rs\n```\nfn main() {}\n```\n";

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn renderer_places_a_placeholder_for_binary_files_by_default() {
        let mut output = Vec::new();
        let mut renderer = Renderer::new(&mut output);

        let input = Cursor::new(&[0x00, 0x01, 0x02, 0xc3]);
        renderer.render_file(Path::new("image.png"), input).unwrap();
        let expected = "\n## File: image.png\n[BINARY FILE]\n";

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn filename_with_linebreaks_and_invalid_chars_handled_properly() {
        let mut output = Vec::new();
        let mut renderer = Renderer::new(&mut output);

        let input = Cursor::new("fn main() {}");
        let filename = Path::new(OsStr::from_bytes(b"jap\xE3\x81\x82dir/some\nma\xc3in.rs"));
        renderer.render_file(filename, input).unwrap();
        let expected = "\n## File: japあdir/some\\nma\\xC3in.rs\n```\nfn main() {}\n```\n";

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn file_with_backticks_is_handled_safely() {
        let mut output = Vec::new();
        let mut renderer = Renderer::new(&mut output);

        let input = Cursor::new("fn main() { println!(\"``` inside\"); }");
        renderer
            .render_file(Path::new("example.rs"), input)
            .unwrap();
        let expected = "\n## File: example.rs\n````\n\
            fn main() { println!(\"``` inside\"); }\n````\n";

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }
}
