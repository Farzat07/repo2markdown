use std::io::{Read, Write};

use repo2markdown::renderer::render;

fn main() {
    println!("Hello, world!");
}

pub fn run<R: Read, W: Write>(mut input: R, mut output: W) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    input.read_to_end(&mut buf)?;
    let rendered = render("Project name", &[])?;
    output.write_all(rendered.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn cli_with_empty_input_produces_empty_project() {
        use std::io::Cursor;

        let input = Cursor::new(b"");
        let mut output = Vec::new();

        run(input, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "# Project name\n"
        );
    }
}
