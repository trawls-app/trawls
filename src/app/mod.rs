use gpui::{AppContext, Context, Entity, FocusHandle, SharedString, Window};
use gpui_component::input::InputState;

use crate::model::{FrameInfo, MergeMode, PreviewData, ProcessingMock, SortKey};

mod logic;
mod views;

pub struct TrawlsDummy {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) active_step: usize,
    pub(crate) lightframes: Vec<FrameInfo>,
    pub(crate) darkframes: Vec<FrameInfo>,
    pub(crate) light_sort: SortKey,
    pub(crate) dark_sort: SortKey,
    pub(crate) output_path: Option<SharedString>,
    pub(crate) output_input: Entity<InputState>,
    pub(crate) merge_mode: MergeMode,
    pub(crate) processing: ProcessingMock,
    pub(crate) preview: Option<PreviewData>,
}

impl TrawlsDummy {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let output_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("No output path selected"));

        Self {
            focus_handle: cx.focus_handle(),
            active_step: 0,
            lightframes: Vec::new(),
            darkframes: Vec::new(),
            light_sort: SortKey::CreationTime,
            dark_sort: SortKey::CreationTime,
            output_path: None,
            output_input,
            merge_mode: MergeMode::Normal,
            processing: ProcessingMock::default(),
            preview: None,
        }
    }

    pub(crate) fn step_title(&self) -> &'static str {
        match self.active_step {
            0 => "1. Add Lightframes",
            1 => "2. Add Darkframes",
            2 => "3. Process images",
            _ => "4. Preview",
        }
    }

    pub(crate) fn step_description(&self) -> &'static str {
        match self.active_step {
            0 => "Select the lightframes in this step.",
            1 => "Optional: add darkframes with matching settings to reduce noise.",
            2 => "Select processing options and run the merge.",
            _ => "Inspect the generated preview after processing.",
        }
    }
}
