#[derive(Clone)]
pub struct FrameInfo {
    pub path: &'static str,
    pub filename: &'static str,
    pub exposure_seconds: f32,
    pub interval_seconds: Option<f32>,
    pub aperture: f32,
    pub iso: u32,
    pub creation_time: &'static str,
    pub error: Option<&'static str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    CreationTime,
    Filename,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MergeMode {
    Normal,
    Falling,
    Raising,
}

#[derive(Clone)]
pub struct PreviewData {
    pub image_url: &'static str,
    pub exif: &'static str,
}

#[derive(Clone)]
pub struct ProcessingMock {
    pub count_lights: usize,
    pub count_darks: usize,
    pub count_loaded: usize,
    pub count_loading: usize,
    pub count_merged: usize,
    pub count_merging: usize,
}

impl Default for ProcessingMock {
    fn default() -> Self {
        Self {
            count_lights: 0,
            count_darks: 0,
            count_loaded: 0,
            count_loading: 0,
            count_merged: 0,
            count_merging: 0,
        }
    }
}

#[derive(Clone)]
pub struct LightframeStats {
    pub total_count: usize,
    pub valid_count: usize,
    pub error_count: usize,
    pub total_exposure_seconds: f32,
    pub total_time_seconds: f32,
    pub non_exposing_seconds: f32,
    pub duty_cycle_percent: f32,
    pub largest_gap: Option<LargestGap>,
}

#[derive(Clone)]
pub struct LargestGap {
    pub from: String,
    pub to: String,
    pub seconds: f32,
}
