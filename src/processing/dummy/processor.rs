use std::path::PathBuf;

use crate::processing::{
    interface::{ProcessingInput, ProcessingStartResult, Processor},
    status::{ProcessingCounters, ProcessingStatus},
};

use crate::ui::model::PreviewData;

pub static DUMMY_PROCESSOR: DummyProcessor = DummyProcessor;

pub struct DummyProcessor;

impl Processor for DummyProcessor {
    fn start(&self, input: ProcessingInput<'_>) -> ProcessingStartResult {
        let issues = readiness_issues(&input);
        if !issues.is_empty() {
            return ProcessingStartResult {
                issues,
                state: ProcessingCounters::default(),
                preview: None,
            };
        }

        let state = initial_state(input.lightframes.len(), input.darkframes.len());
        let frame_count = input.lightframes.len();
        let total_exposure: f32 = input
            .lightframes
            .iter()
            .filter_map(|f| f.exposure_seconds)
            .sum();
        let preview = Some(PreviewData {
            image_path: PathBuf::from("examples/dummy_preview.jpg"),
            exif: "ISO 1600, f/2.8, 20s".to_string(),
            width_px: 6000,
            height_px: 4000,
            frame_count,
            total_exposure_seconds: total_exposure,
            merge_mode: crate::ui::model::MergeMode::Normal,
            output_file_size: 0,
        });

        ProcessingStartResult {
            issues,
            state,
            preview,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessingPhase {
    Loading,
    Merging,
    Done,
}

impl DummyProcessor {
    pub fn current_phase(&self, state: &ProcessingCounters) -> ProcessingPhase {
        let loading_total = state.count_lights + state.count_darks;
        let merge_total =
            state.count_lights.saturating_sub(1) + state.count_darks.saturating_sub(1);
        if state.count_loaded < loading_total {
            ProcessingPhase::Loading
        } else if state.count_merged < merge_total {
            ProcessingPhase::Merging
        } else {
            ProcessingPhase::Done
        }
    }

    pub fn phase_delay_ms(&self, phase: ProcessingPhase) -> Option<u64> {
        match phase {
            ProcessingPhase::Loading => Some(140),
            ProcessingPhase::Merging => Some(320),
            ProcessingPhase::Done => None,
        }
    }

    pub fn emulate_step<S: ProcessingStatus>(&self, state: &mut ProcessingCounters, status: &mut S) {
        let phase = self.current_phase(state);
        self.advance_phase(state, phase);
        status.processing_updated(*state);
    }

    fn advance_phase(&self, state: &mut ProcessingCounters, phase: ProcessingPhase) {
        let loading_total = state.count_lights + state.count_darks;
        let merge_total =
            state.count_lights.saturating_sub(1) + state.count_darks.saturating_sub(1);
        match phase {
            ProcessingPhase::Loading => {
                state.count_loaded = (state.count_loaded + 1).min(loading_total);
                state.count_loading = usize::from(state.count_loaded < loading_total);
                if state.count_loaded == loading_total {
                    state.count_merging = usize::from(merge_total > 0);
                }
            }
            ProcessingPhase::Merging => {
                state.count_merged = (state.count_merged + 1).min(merge_total);
                state.count_merging = usize::from(state.count_merged < merge_total);
                if state.count_merged == merge_total {
                    state.count_loading = 0;
                }
            }
            ProcessingPhase::Done => {
                state.count_loading = 0;
                state.count_merging = 0;
            }
        }
    }
}

fn readiness_issues(input: &ProcessingInput<'_>) -> Vec<String> {
    let mut issues = Vec::new();

    if input.has_metadata_errors {
        issues.push("The metadata of some files could not be loaded.".to_string());
    }
    if input.has_pending_metadata {
        issues.push("Metadata is still loading. Wait for EXIF parsing to complete.".to_string());
    }
    if input.lightframes.len() < 2 {
        issues.push("Less than two lightframes are selected.".to_string());
    }
    if !input.output_path_set {
        issues.push("No output path is specified.".to_string());
    }

    issues
}

fn initial_state(count_lights: usize, count_darks: usize) -> ProcessingCounters {
    let loading_total = count_lights + count_darks;

    ProcessingCounters {
        count_lights,
        count_darks,
        count_loaded: 0,
        count_loading: usize::from(loading_total > 0),
        count_merged: 0,
        count_merging: 0,
    }
}
