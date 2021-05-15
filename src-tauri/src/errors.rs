use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;

#[derive(Debug)]
struct LoadingError {
    file: PathBuf
}

impl fmt::Display for LoadingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Could not load file '{}'", self.file.display())
    }
}

impl Error for LoadingError {}