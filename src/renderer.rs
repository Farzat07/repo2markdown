pub fn render(project_name: &str, files: &[(&str, &str)]) -> String {
    let mut output = format!("# {}\n", project_name);
    if !files.is_empty() {
        output.push_str("\n## Files\n");
    }
    for (filename, content) in files {
        let outer_backticks = outer_backticks(content);
        output.push_str(&format!(
            "\n### {}\n{}\n{}\n{}\n",
            filename, outer_backticks, content, outer_backticks
        ));
    }
    output
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

    #[test]
    fn multiple_files_are_rendered_in_order() {
        let files = vec![("main.rs", "fn main() {}"), ("lib.rs", "pub fn hello() {}")];

        let output = render("Project name", &files);

        assert_eq!(
            output,
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
        let files = vec![("example.rs", "fn main() { println!(\"``` inside\"); }")];

        let output = render("Project name", &files);

        assert_eq!(
            output,
            "# Project name\n\n\
            ## Files\n\n\
            ### example.rs\n\
            ````\n\
            fn main() { println!(\"``` inside\"); }\n\
            ````\n"
        );
    }
}
