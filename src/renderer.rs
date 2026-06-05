use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::normalizer::NormalizedPath;

const DEFAULT_MAX_FILE_SIZE: u64 = 1_000_000;

#[derive(Debug)]
pub struct Renderer<W: Write> {
    output: W,
    max_file_size: u64,
}

impl<W: Write> Renderer<W> {
    pub fn new(output: W) -> Self {
        Self {
            output,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        }
    }

    pub fn with_max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = max_file_size;
        self
    }

    pub fn render_header(&mut self, project_name: &str) -> std::io::Result<()> {
        writeln!(self.output, "# {}", project_name)
    }

    pub fn render_path(&mut self, normalized_path: &NormalizedPath) -> std::io::Result<()> {
        let metadata = std::fs::metadata(&normalized_path.absolute)?;
        if metadata.len() > self.max_file_size {
            self.warn_about_filesize(&normalized_path.relative, metadata.len());
            self.render_large_file(&normalized_path.relative)
        } else {
            let file = File::open(&normalized_path.absolute)?;
            self.render_file(&normalized_path.relative, file)
        }
    }

    fn render_file<R: Read>(&mut self, filename: &Path, mut reader: R) -> std::io::Result<()> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let contents = if let Ok(utf8string) = std::str::from_utf8(&bytes) {
            utf8string
        } else {
            self.warn_about_binary_file(filename);
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

    fn render_large_file(&mut self, filename: &Path) -> std::io::Result<()> {
        let name = render_filename(filename);
        writeln!(self.output)?;
        writeln!(self.output, "## File: {}", name)?;
        writeln!(self.output, "[FILE TOO LARGE]")
    }

    fn render_binary_file(&mut self, filename: &Path) -> std::io::Result<()> {
        let name = render_filename(filename);
        writeln!(self.output)?;
        writeln!(self.output, "## File: {}", name)?;
        writeln!(self.output, "[BINARY FILE]")
    }

    fn warn_about_filesize(&self, filename: &Path, filesize: u64) {
        eprintln!(
            "Warning: skipping large file: {} ({} > limit {})",
            render_filename(filename),
            human_readable_size(filesize),
            human_readable_size(self.max_file_size),
        )
    }

    fn warn_about_binary_file(&self, filename: &Path) {
        eprintln!(
            "Warning: skipping binary file: {}",
            render_filename(filename),
        )
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

fn human_readable_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsStr,
        io::Cursor,
        os::unix::ffi::OsStrExt,
        path::{Path, PathBuf},
    };

    use tempfile::tempdir;

    use crate::normalizer::NormalizedPath;

    use super::{Renderer, human_readable_size};

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

    #[test]
    fn renderer_places_placeholder_for_large_files_by_default() {
        let mut output = Vec::new();
        let mut renderer = Renderer::new(&mut output).with_max_file_size(5); // smaller than file

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("big.txt");

        let content = "A".repeat(10); // 10 bytes -> bigger than the limit
        std::fs::write(&file_path, &content).unwrap();
        let normalized_path = NormalizedPath {
            relative: PathBuf::from("big.txt"),
            absolute: file_path,
        };

        renderer.render_path(&normalized_path).unwrap();
        let expected = "\n## File: big.txt\n[FILE TOO LARGE]\n";

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn format_readable_filesizes() {
        assert_eq!(human_readable_size(10), "10 B");
        assert_eq!(human_readable_size(1500), "1.5 KiB");
        assert_eq!(human_readable_size(1_048_576), "1.0 MiB");
        assert_eq!(human_readable_size(5_242_880), "5.0 MiB");
    }
}
