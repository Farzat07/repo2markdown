pub fn render(project_name: &str, files: &[(&str, &str)]) -> String {
    let mut output = format!("# {}\n", project_name);
    if !files.is_empty() {
        output.push_str("\n## Files\n");
    }
    for (filename, content) in files {
        output.push_str(&format!("\n### {}\n```\n{}\n```\n", filename, content));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn empty_project_renders_only_title() {
        let output = render("Project name", &[]);
        assert_eq!(output, "# Project name\n");
    }

    #[test]
    fn single_file_is_rendered() {
        let files = vec![("main.rs", "fn main() {}")];

        let output = render("Project name", &files);

        assert_eq!(
            output,
            "# Project name\n\n\
            ## Files\n\n\
            ### main.rs\n\
            ```\n\
            fn main() {}\n\
            ```\n"
        );
    }
}
