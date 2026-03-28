use crate::processing::interface::FrameIdentifier;
use crate::processing::metadata::FrameMetadata;

#[derive(Clone, Copy, Default, Debug)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processing_counters_default() {
        let c = ProcessingCounters::default();
        assert_eq!(c.count_lights, 0);
        assert_eq!(c.count_darks, 0);
        assert_eq!(c.count_loaded, 0);
        assert_eq!(c.count_loading, 0);
        assert_eq!(c.count_merged, 0);
        assert_eq!(c.count_merging, 0);
    }

    #[test]
    fn processing_counters_copy() {
        let a = ProcessingCounters {
            count_lights: 5,
            count_darks: 2,
            ..Default::default()
        };
        let b = a;
        assert_eq!(b.count_lights, 5);
        assert_eq!(b.count_darks, 2);
    }
}
