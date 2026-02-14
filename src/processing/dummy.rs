use crate::processing::interface::{ProcessingInput, ProcessingStartResult, Processor};
use crate::ui::model::{PreviewData, ProcessingMock};

pub static DUMMY_PROCESSOR: DummyProcessor = DummyProcessor;

pub struct DummyProcessor;

impl Processor for DummyProcessor {
    fn start(&self, input: ProcessingInput<'_>) -> ProcessingStartResult {
        let issues = readiness_issues(&input);
        if !issues.is_empty() {
            return ProcessingStartResult {
                issues,
                state: ProcessingMock::default(),
                preview: None,
            };
        }

        let state = initial_state(input.lightframes.len(), input.darkframes.len());
        let preview = Some(PreviewData {
            image_url: "examples/dummy_preview.jpg",
            exif: "ISO 1600, f/2.8, 20s",
            width_px: 6000,
            height_px: 4000,
        });

        ProcessingStartResult {
            issues,
            state,
            preview,
        }
    }

    fn advance(&self, state: &mut ProcessingMock) {
        let loading_total = state.count_lights + state.count_darks;
        if state.count_loaded < loading_total {
            state.count_loading = 1;
            state.count_loaded = (state.count_loaded + 1).min(loading_total);
            if state.count_loaded == loading_total {
                state.count_loading = 0;
            }
        }

        let merge_total =
            state.count_lights.saturating_sub(1) + state.count_darks.saturating_sub(1);
        if state.count_merged < merge_total {
            state.count_merging = 1;
            state.count_merged = (state.count_merged + 1).min(merge_total);
            if state.count_merged == merge_total {
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
    if input.lightframes.len() < 2 {
        issues.push("Less than two lightframes are selected.".to_string());
    }
    if !input.output_path_set {
        issues.push("No output path is specified.".to_string());
    }

    issues
}

fn initial_state(count_lights: usize, count_darks: usize) -> ProcessingMock {
    let merge_total = count_lights.saturating_sub(1) + count_darks.saturating_sub(1);

    ProcessingMock {
        count_lights,
        count_darks,
        count_loaded: count_lights + count_darks,
        count_loading: 0,
        count_merged: merge_total.saturating_sub(1),
        count_merging: usize::from(merge_total > 0),
    }
}
