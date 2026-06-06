#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Logger {
    verbosity: Verbosity,
}

impl Logger {
    pub fn new(verbosity: Verbosity) -> Self {
        Self { verbosity }
    }

    pub fn warn(&self, message: impl AsRef<str>) {
        if self.verbosity != Verbosity::Quiet {
            eprintln!("{}", message.as_ref());
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            verbosity: Verbosity::Quiet,
        }
    }
}
