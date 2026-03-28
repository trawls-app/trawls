use chrono::NaiveDateTime;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct FrameInfo {
    pub path: String,
    pub filename: String,
    pub exposure_seconds: Option<f32>,
    pub interval_seconds: Option<f32>,
    pub aperture: Option<f32>,
    pub iso: Option<u32>,
    pub creation_time: Option<NaiveDateTime>,
    pub metadata_status: MetadataStatus,
    pub error: Option<String>,
    pub thumbnail: Option<Arc<gpui::Image>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MetadataStatus {
    Pending,
    Loaded,
    Failed,
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

impl MergeMode {
    pub fn label(self) -> &'static str {
        match self {
            MergeMode::Normal => "Normal",
            MergeMode::Falling => "Falling Comets",
            MergeMode::Raising => "Raising Comets",
        }
    }
}

#[derive(Clone)]
pub struct PreviewData {
    pub image_path: PathBuf,
    pub exif: String,
    pub width_px: u32,
    pub height_px: u32,
    pub frame_count: usize,
    pub total_exposure_seconds: f32,
    pub merge_mode: MergeMode,
    pub output_file_size: u64,
}


#[derive(Clone)]
pub struct FramePreview {
    pub filename: String,
    pub image: Arc<gpui::Image>,
    pub width_px: u32,
    pub height_px: u32,
    pub exif_summary: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_mode_labels() {
        assert_eq!(MergeMode::Normal.label(), "Normal");
        assert_eq!(MergeMode::Falling.label(), "Falling Comets");
        assert_eq!(MergeMode::Raising.label(), "Raising Comets");
    }
}
