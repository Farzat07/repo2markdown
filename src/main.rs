use std::{
    env,
    io::{self},
    path::Path,
};

use repo2markdown::{
    logger::{Logger, Verbosity},
    run::run,
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
