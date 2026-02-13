use crate::ui::model::{FrameInfo, PreviewData, ProcessingMock};

pub struct ProcessingInput<'a> {
    pub lightframes: &'a [FrameInfo],
    pub darkframes: &'a [FrameInfo],
    pub output_path_set: bool,
    pub has_metadata_errors: bool,
}

pub struct ProcessingStartResult {
    pub issues: Vec<String>,
    pub state: ProcessingMock,
    pub preview: Option<PreviewData>,
}

pub trait Processor {
    fn start(&self, input: ProcessingInput<'_>) -> ProcessingStartResult;
    fn advance(&self, state: &mut ProcessingMock);
}
