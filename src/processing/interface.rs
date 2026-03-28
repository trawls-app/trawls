use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FrameIdentifier(PathBuf);

impl FrameIdentifier {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &std::path::Path {
        &self.0
    }
}

impl From<&str> for FrameIdentifier {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for FrameIdentifier {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
