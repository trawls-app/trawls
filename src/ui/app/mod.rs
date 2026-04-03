use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use gpui::{AppContext, Context, Entity, FocusHandle, SharedString, Window};
use gpui_component::{VirtualListScrollHandle, input::InputState};

use crate::processing::status::ProcessingCounters;
use crate::ui::model::{FrameInfo, FramePreview, MergeMode, PreviewData, SortKey};

mod logic;
mod views;

use views::image_viewer::ImageViewerState;

pub struct Trawls {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) active_step: usize,
    pub(crate) lightframes: Vec<FrameInfo>,
    pub(crate) darkframes: Vec<FrameInfo>,
    pub(crate) selected_light_paths: HashSet<String>,
    pub(crate) selected_dark_paths: HashSet<String>,
    pub(crate) light_sort: SortKey,
    pub(crate) dark_sort: SortKey,
    pub(crate) last_clicked_light: Option<usize>,
    pub(crate) last_clicked_dark: Option<usize>,
    pub(crate) output_path: Option<SharedString>,
    pub(crate) output_input: Entity<InputState>,
    pub(crate) merge_mode: MergeMode,
    pub(crate) processing: ProcessingCounters,
    pub(crate) processing_running: bool,
    pub(crate) processing_cancel: Arc<AtomicBool>,
    pub(crate) processing_started_at: Option<Instant>,
    pub(crate) processing_elapsed: Option<f32>,
    pub(crate) preview: Option<PreviewData>,
    pub(crate) preview_viewer: ImageViewerState,
    pub(crate) table_scroll_handle: VirtualListScrollHandle,
    pub(crate) table_scroll_state: gpui_component::scroll::ScrollbarState,
    pub(crate) frame_preview: Option<FramePreview>,
    pub(crate) frame_preview_viewer: ImageViewerState,
    pub(crate) frame_preview_loading: bool,
}

impl Trawls {
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
            last_clicked_light: None,
            last_clicked_dark: None,
            output_path: None,
            output_input,
            merge_mode: MergeMode::Normal,
            processing: ProcessingCounters::default(),
            processing_running: false,
            processing_cancel: Arc::new(AtomicBool::new(false)),
            processing_started_at: None,
            processing_elapsed: None,
            preview: None,
            preview_viewer: ImageViewerState::default(),
            table_scroll_handle: VirtualListScrollHandle::new(),
            table_scroll_state: gpui_component::scroll::ScrollbarState::default(),
            frame_preview: None,
            frame_preview_viewer: ImageViewerState::default(),
            frame_preview_loading: false,
        }
    }

    pub(crate) fn selected_paths_mut(&mut self, is_light: bool) -> &mut HashSet<String> {
        if is_light {
            &mut self.selected_light_paths
        } else {
            &mut self.selected_dark_paths
        }
    }

    pub(crate) fn selected_paths(&self, is_light: bool) -> &HashSet<String> {
        if is_light {
            &self.selected_light_paths
        } else {
            &self.selected_dark_paths
        }
    }

    pub(crate) fn toggle_selection(&mut self, path: &str, is_light: bool) {
        let set = self.selected_paths_mut(is_light);
        if !set.remove(path) {
            set.insert(path.to_string());
        }
    }

    pub(crate) fn set_selection(&mut self, path: &str, is_light: bool, selected: bool) {
        let set = self.selected_paths_mut(is_light);
        if selected {
            set.insert(path.to_string());
        } else {
            set.remove(path);
        }
    }

    pub(crate) fn last_clicked_mut(&mut self, is_light: bool) -> &mut Option<usize> {
        if is_light {
            &mut self.last_clicked_light
        } else {
            &mut self.last_clicked_dark
        }
    }

    pub(crate) fn row_clicked(
        &mut self,
        ix: usize,
        path: &str,
        is_light: bool,
        shift: bool,
        sorted_paths: &[String],
    ) {
        let last = self.last_clicked_mut(is_light);
        if shift {
            if let Some(anchor) = *last {
                // Match the selection state of the anchor row
                let anchor_selected = sorted_paths
                    .get(anchor)
                    .is_some_and(|p| self.selected_paths(is_light).contains(p.as_str()));
                let from = anchor.min(ix);
                let to = anchor.max(ix);
                for i in from..=to {
                    if let Some(p) = sorted_paths.get(i) {
                        self.set_selection(p, is_light, anchor_selected);
                    }
                }
                return;
            }
        }
        self.toggle_selection(path, is_light);
        *self.last_clicked_mut(is_light) = Some(ix);
    }

    pub(crate) fn set_active_step(&mut self, step: usize, cx: &mut Context<Self>) {
        self.active_step = step;
        self.frame_preview = None;
        self.frame_preview_loading = false;
        cx.notify();
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
