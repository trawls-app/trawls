use std::collections::HashSet;

use gpui::{AppContext, Context, Entity, FocusHandle, ScrollHandle, SharedString, Window};
use gpui_component::{VirtualListScrollHandle, input::InputState, scroll::ScrollbarState};

use crate::processing::status::ProcessingCounters;
use crate::ui::model::{FrameInfo, MergeMode, PreviewData, SortKey};

mod logic;
mod views;

pub struct TrawlsDummy {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) active_step: usize,
    pub(crate) lightframes: Vec<FrameInfo>,
    pub(crate) darkframes: Vec<FrameInfo>,
    pub(crate) selected_light_paths: HashSet<String>,
    pub(crate) selected_dark_paths: HashSet<String>,
    pub(crate) light_sort: SortKey,
    pub(crate) dark_sort: SortKey,
    pub(crate) output_path: Option<SharedString>,
    pub(crate) output_input: Entity<InputState>,
    pub(crate) merge_mode: MergeMode,
    pub(crate) processing: ProcessingCounters,
    pub(crate) processing_running: bool,
    pub(crate) preview: Option<PreviewData>,
    pub(crate) preview_zoom: f32,
    pub(crate) preview_scroll_handle: ScrollHandle,
    pub(crate) preview_scroll_state: ScrollbarState,
    pub(crate) table_scroll_handle: VirtualListScrollHandle,
    pub(crate) table_scroll_state: ScrollbarState,
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
            selected_light_paths: HashSet::new(),
            selected_dark_paths: HashSet::new(),
            light_sort: SortKey::CreationTime,
            dark_sort: SortKey::CreationTime,
            output_path: None,
            output_input,
            merge_mode: MergeMode::Normal,
            processing: ProcessingCounters::default(),
            processing_running: false,
            preview: None,
            preview_zoom: 1.0,
            preview_scroll_handle: ScrollHandle::new(),
            preview_scroll_state: ScrollbarState::default(),
            table_scroll_handle: VirtualListScrollHandle::new(),
            table_scroll_state: ScrollbarState::default(),
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
