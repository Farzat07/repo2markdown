use std::fmt;

#[derive(Debug)]
pub enum RenderError {
    BinaryFile(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::BinaryFile(filename) => {
                write!(f, "Binary file encountered: {}", filename)
            }
        }
    }
}

impl std::error::Error for RenderError {}

pub fn render(project_name: &str, files: &[(&str, &[u8])]) -> Result<String, RenderError> {
    let mut output = format!("# {}\n", project_name);
    if !files.is_empty() {
        output.push_str("\n## Files\n");
    }
    for (filename, bytes) in files {
        let content = std::str::from_utf8(bytes)
            .map_err(|_| RenderError::BinaryFile(filename.to_string()))?;
        let outer_backticks = outer_backticks(content);
        output.push_str(&format!(
            "\n### {}\n{}\n{}\n{}\n",
            filename, outer_backticks, content, outer_backticks
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

#[cfg(test)]
mod tests {
    use super::{RenderError, render};

    #[test]
    fn empty_project_renders_only_title() {
        let output = render("Project name", &[]);
        assert_eq!(output.unwrap(), "# Project name\n");
    }

    #[test]
    fn single_file_is_rendered() {
        let files: Vec<(&str, &[u8])> = vec![("main.rs", b"fn main() {}")];

        let output = render("Project name", &files);

        assert_eq!(
            output.unwrap(),
            "# Project name\n\n\
            ## Files\n\n\
            ### main.rs\n\
            ```\n\
            fn main() {}\n\
            ```\n"
        );
    }

    #[test]
    fn multiple_files_are_rendered_in_order() {
        let files: Vec<(&str, &[u8])> = vec![
            ("main.rs", b"fn main() {}"),
            ("lib.rs", b"pub fn hello() {}"),
        ];

        let output = render("Project name", &files);

        assert_eq!(
            output.unwrap(),
            "# Project name\n\n\
            ## Files\n\n\
            ### main.rs\n\
            ```\n\
            fn main() {}\n\
            ```\n\n\
            ### lib.rs\n\
            ```\n\
            pub fn hello() {}\n\
            ```\n"
        );
    }

    #[test]
    fn file_with_backticks_is_handled_safely() {
        let files: Vec<(&str, &[u8])> =
            vec![("example.rs", b"fn main() { println!(\"``` inside\"); }")];

        let output = render("Project name", &files);

        assert_eq!(
            output.unwrap(),
            "# Project name\n\n\
            ## Files\n\n\
            ### example.rs\n\
            ````\n\
            fn main() { println!(\"``` inside\"); }\n\
            ````\n"
        );
    }

    #[test]
    fn binary_file_is_rejected() {
        let files: Vec<(&str, &[u8])> = vec![("image.png", &[0x00, 0x01, 0x02, 0xc3])];

        let result = render("Project name", &files);

        assert!(matches!(
                result,
                Err(RenderError::BinaryFile(name)) if name == "image.png"
        ));
    }
}
