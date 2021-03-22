#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    source: Option<Box<dyn std::error::Error>>,
}

impl Error {
    pub fn new(error_type: ErrorKind, source: Option<Box<dyn std::error::Error>>) -> Error {
        Error {
            kind: error_type,
            source,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(underlying_error) = self.source.as_ref() {
            f.write_fmt(format_args!(
                "Error in TOML Parser, error was {}, underlying error was {}",
                self.kind, underlying_error
            ))?;
        } else {
            f.write_fmt(format_args!(
                "Error in TOML Parser, error was {}",
                self.kind
            ))?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
#[derive(Debug)]
pub enum ErrorKind {
    /// There are only two valid line endings \n and \r\n
    InvalidEndOfLine(usize),
    /// No Value was found on the given line - ie 'fred =' or 'fred'
    MissingValue(usize),
    /// Unrecognizable value type ie 'fred = ..! ' this is not a known value type
    UnknownValueType(usize),
    /// Invalid Value ie including letters in a number, not ending a string with a "
    InvalidValue(usize),
    /// Name contains invalid characters, ie fred\n = 4 or fred \n = 4
    InvalidName(usize),
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::InvalidEndOfLine(line_number) => f.write_fmt(format_args!("Line {} ended with an unsupported line ending, only \\n or \\r\\n are supported in TOML files", line_number)),
            ErrorKind::MissingValue(line_number) => f.write_fmt(format_args!("Line {} is missing a value", line_number)),
            ErrorKind::UnknownValueType(line_number) => f.write_fmt(format_args!("The value on line {} is of an unknown type", line_number)),
            ErrorKind::InvalidValue(line_number) => f.write_fmt(format_args!("The value on line {} is invalid", line_number)),
            ErrorKind::InvalidName(line_number) => f.write_fmt(format_args!("The name on line {} is invalid", line_number)),
        }
    }
}
