use std::path::PathBuf;

use crate::processing::status::ProcessingCounters;
use crate::ui::model::{FrameInfo, PreviewData};

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

pub struct ProcessingInput<'a> {
    pub lightframes: &'a [FrameInfo],
    pub darkframes: &'a [FrameInfo],
    pub output_path_set: bool,
    pub has_metadata_errors: bool,
    pub has_pending_metadata: bool,
}

pub struct ProcessingStartResult {
    pub issues: Vec<String>,
    pub state: ProcessingCounters,
    pub preview: Option<PreviewData>,
}

pub trait Processor {
    fn start(&self, input: ProcessingInput<'_>) -> ProcessingStartResult;
}
