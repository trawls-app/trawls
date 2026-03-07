use crate::processing::interface::FrameIdentifier;
use crate::processing::metadata::FrameMetadata;

#[derive(Clone, Copy, Default)]
pub struct ProcessingCounters {
    pub count_lights: usize,
    pub count_darks: usize,
    pub count_loaded: usize,
    pub count_loading: usize,
    pub count_merged: usize,
    pub count_merging: usize,
}

pub trait MetadataStatus {
    fn metadata_queued(&mut self, frame_id: &FrameIdentifier);
    fn metadata_loaded(&mut self, frame_id: &FrameIdentifier, metadata: FrameMetadata);
    fn metadata_failed(&mut self, frame_id: &FrameIdentifier, error: String);
}

pub trait ProcessingStatus {
    fn processing_updated(&mut self, counters: ProcessingCounters);
}
