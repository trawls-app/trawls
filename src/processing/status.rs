use crate::processing::interface::FrameIdentifier;

#[derive(Clone)]
pub struct FrameMetadata {
    pub exposure_seconds: f32,
    pub interval_seconds: Option<f32>,
    pub aperture: f32,
    pub iso: u32,
    pub creation_time: String,
}

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
