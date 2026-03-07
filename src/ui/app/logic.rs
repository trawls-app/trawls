use std::{path::PathBuf, time::Duration};

use gpui::{
    Context, ParentElement as _, PathPromptOptions, Pixels, Point, Size, Styled as _, Timer,
    Window, div, point, px,
};
use gpui_component::{
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};

use crate::processing::{
    dummy::{DUMMY_METADATA_PROCESSOR, DUMMY_PROCESSOR},
    interface::{FrameIdentifier, ProcessingInput, Processor},
    metadata::{FrameMetadata, FrameMetadataLoader, RawlerMetadataLoader},
    status::{
        MetadataStatus as MetadataStatusSink, ProcessingCounters,
        ProcessingStatus as ProcessingStatusSink,
    },
};
use crate::ui::{
    model::{FrameInfo, LargestGap, LightframeStats, MetadataStatus, SortKey},
    samples::{MockFrameSeed, sample_darkframe_seeds, sample_lightframe_seeds},
};

use super::TrawlsDummy;

const MIN_PREVIEW_ZOOM: f32 = 0.02;
const MAX_PREVIEW_ZOOM: f32 = 8.0;

impl TrawlsDummy {
    pub(crate) fn sorted_frames(frames: &[FrameInfo], sort: SortKey) -> Vec<FrameInfo> {
        let mut rows: Vec<FrameInfo> = frames.to_vec();
        rows.sort_by(|a, b| {
            let a_rank = frame_rank(a);
            let b_rank = frame_rank(b);
            if a_rank != b_rank {
                return a_rank.cmp(&b_rank);
            }

            match sort {
                SortKey::CreationTime => a.creation_time.cmp(&b.creation_time),
                SortKey::Filename => a.filename.cmp(&b.filename),
            }
        });
        rows
    }

    pub(crate) fn error_warning(frames: &[FrameInfo]) -> bool {
        frames.iter().any(|f| f.error.is_some())
    }

    pub(crate) fn camera_setting_warning(frames: &[FrameInfo]) -> bool {
        let mut apertures = std::collections::BTreeSet::new();
        let mut exposures = std::collections::BTreeSet::new();
        let mut isos = std::collections::BTreeSet::new();

        for frame in frames
            .iter()
            .filter(|f| f.metadata_status == MetadataStatus::Loaded && f.error.is_none())
        {
            if let Some(aperture) = frame.aperture {
                apertures.insert(format!("{aperture:.1}"));
            }
            if let Some(exposure) = frame.exposure_seconds {
                exposures.insert(format!("{exposure:.1}"));
            }
            if let Some(iso) = frame.iso {
                isos.insert(iso);
            }
        }

        apertures.len() > 1 || exposures.len() > 1 || isos.len() > 1
    }

    pub(crate) fn interval_warning_threshold(frames: &[FrameInfo]) -> Option<f32> {
        let mut sum = 0.0;
        let mut count = 0;
        for frame in frames {
            if let Some(dt) = frame.interval_seconds {
                sum += dt.abs();
                count += 1;
            }
        }

        if count > 0 {
            Some((sum / count as f32) + 1.0)
        } else {
            None
        }
    }

    pub(crate) fn interval_warning(frames: &[FrameInfo]) -> bool {
        if let Some(threshold) = Self::interval_warning_threshold(frames) {
            return frames
                .iter()
                .any(|f| f.interval_seconds.is_some_and(|v| v.abs() > threshold));
        }

        false
    }

    pub(crate) fn choose_dummy_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let next = if self.output_path.is_some() {
            "~/Pictures/startrail_result.dng"
        } else {
            "~/Pictures/trawls_output.dng"
        };

        self.output_path = Some(next.into());
        self.output_input.update(cx, |state, cx| {
            state.set_value(next, window, cx);
        });
        cx.notify();
    }

    pub(crate) fn clear_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.output_path = None;
        self.output_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        cx.notify();
    }

    pub(crate) fn queue_mock_metadata_loading(&mut self, is_light: bool, cx: &mut Context<Self>) {
        let seeds = if is_light {
            sample_lightframe_seeds()
        } else {
            sample_darkframe_seeds()
        };

        let rows = seeds
            .iter()
            .map(MockFrameSeed::as_pending_row)
            .collect::<Vec<_>>();
        if is_light {
            self.lightframes = rows;
            self.selected_light_paths.clear();
        } else {
            self.darkframes = rows;
            self.selected_dark_paths.clear();
        }
        self.reset_processing_state_for_selection();
        for seed in &seeds {
            self.metadata_queued(&FrameIdentifier::from(seed.path));
        }
        cx.notify();

        for (ix, seed) in seeds.into_iter().enumerate() {
            let delay_ms = mock_metadata_delay_ms(seed.path, ix);
            let frame_id = FrameIdentifier::from(seed.path);
            cx.spawn(async move |this, cx| {
                Timer::after(Duration::from_millis(delay_ms)).await;
                let load_result = DUMMY_METADATA_PROCESSOR.load_metadata(&frame_id);
                let _ = this.update(cx, |this, cx| {
                    match load_result {
                        Ok(metadata) => this.metadata_loaded(&frame_id, metadata),
                        Err(error) => this.metadata_failed(&frame_id, error.to_string()),
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }

    pub(crate) fn pick_image_files(
        &mut self,
        is_light: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected_paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Select image files".into()),
        });
        let view = cx.entity();

        cx.spawn_in(window, async move |_, window| {
            let selected_paths = selected_paths.await.ok()?.ok()??;
            let files: Vec<PathBuf> = selected_paths.into_iter().collect();
            if files.is_empty() {
                return Some(());
            }

            window
                .update(|_, cx| {
                    _ = view.update(cx, |this, cx| {
                        this.queue_selected_metadata_loading(files, is_light, cx);
                    });
                })
                .ok();

            Some(())
        })
        .detach();
    }

    pub(crate) fn queue_selected_metadata_loading(
        &mut self,
        files: Vec<PathBuf>,
        is_light: bool,
        cx: &mut Context<Self>,
    ) {
        let existing_paths = if is_light {
            self.lightframes
                .iter()
                .map(|row| row.path.clone())
                .collect::<std::collections::HashSet<_>>()
        } else {
            self.darkframes
                .iter()
                .map(|row| row.path.clone())
                .collect::<std::collections::HashSet<_>>()
        };

        let mut new_files = Vec::new();
        let new_rows = files
            .into_iter()
            .filter(|path| !existing_paths.contains(path.to_string_lossy().as_ref()))
            .map(|path| {
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown");
                let path_string = path.to_string_lossy().to_string();
                new_files.push(PathBuf::from(&path_string));
                FrameInfo {
                    path: path_string,
                    filename: filename.to_string(),
                    exposure_seconds: None,
                    interval_seconds: None,
                    aperture: None,
                    iso: None,
                    creation_time: None,
                    metadata_status: MetadataStatus::Pending,
                    error: None,
                }
            })
            .collect::<Vec<_>>();

        if is_light {
            self.lightframes.extend(new_rows);
        } else {
            self.darkframes.extend(new_rows);
        }
        self.reset_processing_state_for_selection();
        for path in &new_files {
            self.metadata_queued(&FrameIdentifier::new(path.clone()));
        }
        cx.notify();

        for path in new_files {
            let frame_id = FrameIdentifier::new(path);
            cx.spawn(async move |this, cx| {
                let load_result = RawlerMetadataLoader.load_metadata(&frame_id);
                let _ = this.update(cx, |this, cx| {
                    match load_result {
                        Ok(metadata) => this.metadata_loaded(&frame_id, metadata),
                        Err(error) => this.metadata_failed(&frame_id, error.to_string()),
                    }
                    cx.notify();
                });
            })
            .detach();
        }
    }

    pub(crate) fn run_dummy_processing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.processing_running {
            return;
        }

        let result = DUMMY_PROCESSOR.start(ProcessingInput {
            lightframes: &self.lightframes,
            darkframes: &self.darkframes,
            output_path_set: self.output_path.is_some(),
            has_metadata_errors: Self::error_warning(&self.lightframes)
                || Self::error_warning(&self.darkframes),
            has_pending_metadata: self
                .lightframes
                .iter()
                .chain(self.darkframes.iter())
                .any(|row| row.metadata_status == MetadataStatus::Pending),
        });

        if !result.issues.is_empty() {
            window.open_dialog(cx, move |dialog, _, _| {
                dialog
                    .title("Can't start processing")
                    .child(
                        v_flex().gap_2().children(
                            result
                                .issues
                                .iter()
                                .map(|item| div().text_sm().child(format!("- {item}"))),
                        ),
                    )
                    .footer(|_, _, _, _| {
                        vec![
                            Button::new("ready-ok")
                                .primary()
                                .label("OK")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        ]
                    })
            });
            return;
        }

        self.processing = result.state;
        self.preview = None;
        self.processing_running = true;
        let preview = result.preview;
        let should_go_to_preview = self.active_step != 3;
        let viewport_size = window.viewport_size();

        cx.spawn(async move |this, cx| {
            loop {
                let phase = match this.update(cx, |this, _| {
                    DUMMY_PROCESSOR.current_phase(&this.processing)
                }) {
                    Ok(phase) => phase,
                    Err(_) => return,
                };

                let Some(delay_ms) = DUMMY_PROCESSOR.phase_delay_ms(phase) else {
                    break;
                };
                Timer::after(Duration::from_millis(delay_ms)).await;

                let _ = this.update(cx, |this, cx| {
                    let mut state = std::mem::take(&mut this.processing);
                    DUMMY_PROCESSOR.emulate_step(&mut state, this);
                    this.processing = state;
                    cx.notify();
                });
            }

            let _ = this.update(cx, |this, cx| {
                this.processing_running = false;
                this.preview = preview.clone();
                if this.preview.is_some() {
                    this.preview_zoom = this.fit_preview_zoom_for_viewport(viewport_size);
                }
                if should_go_to_preview {
                    this.active_step = 3;
                }
                cx.notify();
            });
        })
        .detach();

        window.push_notification("Dummy processing started.", cx);
        cx.notify();
    }

    pub(crate) fn set_preview_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        self.preview_zoom = zoom.clamp(MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM);
        cx.notify();
    }

    pub(crate) fn zoom_preview_in(&mut self, cx: &mut Context<Self>) {
        self.preview_zoom = self
            .next_preview_zoom(true)
            .clamp(MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM);
        cx.notify();
    }

    pub(crate) fn zoom_preview_out(&mut self, cx: &mut Context<Self>) {
        self.preview_zoom = self
            .next_preview_zoom(false)
            .clamp(MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM);
        cx.notify();
    }

    pub(crate) fn zoom_preview_at_cursor(
        &mut self,
        cursor: Point<Pixels>,
        zoom_in: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let old_zoom = self.preview_zoom;
        let new_zoom = self.next_preview_zoom(zoom_in);
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        let viewport_bounds = self.preview_scroll_handle.bounds();
        let old_offset = self.preview_scroll_handle.offset();

        let local_x = (f32::from(cursor.x) - f32::from(viewport_bounds.origin.x))
            .clamp(0.0, f32::from(viewport_bounds.size.width));
        let local_y = (f32::from(cursor.y) - f32::from(viewport_bounds.origin.y))
            .clamp(0.0, f32::from(viewport_bounds.size.height));

        let offset_x = f32::from(old_offset.x);
        let offset_y = f32::from(old_offset.y);
        let scale = new_zoom / old_zoom.max(0.0001);

        let target_offset = point(
            px(offset_x * scale + local_x * (1.0 - scale)),
            px(offset_y * scale + local_y * (1.0 - scale)),
        );

        self.preview_zoom = new_zoom;
        self.preview_scroll_handle.set_offset(target_offset);
        window.defer(cx, {
            let handle = self.preview_scroll_handle.clone();
            move |_, _| {
                handle.set_offset(target_offset);
            }
        });
        cx.notify();
    }

    pub(crate) fn fit_preview_to_window(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.preview_zoom = self.fit_preview_zoom_for_viewport(window.viewport_size());
        cx.notify();
    }

    fn fit_preview_zoom_for_viewport(&self, viewport: Size<Pixels>) -> f32 {
        let Some(preview) = &self.preview else {
            return 1.0;
        };

        let available_w = (f32::from(viewport.width) - 360.0).max(220.0);
        let available_h = (f32::from(viewport.height) - 250.0).max(180.0);

        let zoom_w = available_w / preview.width_px as f32;
        let zoom_h = available_h / preview.height_px as f32;
        zoom_w.min(zoom_h).clamp(MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM)
    }

    fn next_preview_zoom(&self, zoom_in: bool) -> f32 {
        if zoom_in {
            self.preview_zoom * 1.25
        } else {
            self.preview_zoom / 1.25
        }
        .clamp(MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM)
    }

    pub(crate) fn loading_percent(&self) -> f32 {
        let total = (self.processing.count_lights + self.processing.count_darks).max(1) as f32;
        let current = self.processing.count_loaded.min(total as usize) as f32;
        (current / total * 100.0).clamp(0.0, 100.0)
    }

    pub(crate) fn merge_percent(&self) -> f32 {
        let total = (self.processing.count_lights.saturating_sub(1)
            + self.processing.count_darks.saturating_sub(1))
        .max(1) as f32;
        let current =
            self.processing.count_merged as f32 + (self.processing.count_merging as f32 * 0.5);
        (current / total * 100.0).clamp(0.0, 100.0)
    }

    pub(crate) fn lightframe_stats(rows: &[FrameInfo]) -> Option<LightframeStats> {
        let total_count = rows.len();
        let error_count = rows.iter().filter(|r| r.error.is_some()).count();
        let valid_rows: Vec<&FrameInfo> = rows
            .iter()
            .filter(|r| {
                r.metadata_status == MetadataStatus::Loaded
                    && r.error.is_none()
                    && r.exposure_seconds.is_some()
            })
            .collect();
        if valid_rows.is_empty() {
            return None;
        }
        let valid_count = valid_rows.len();

        let total_exposure_seconds: f32 =
            valid_rows.iter().filter_map(|r| r.exposure_seconds).sum();
        let non_exposing_seconds: f32 = valid_rows
            .iter()
            .filter_map(|r| r.interval_seconds)
            .map(|v| v.max(0.0))
            .sum();
        let total_time_seconds = Self::capture_span_seconds(&valid_rows)
            .unwrap_or(total_exposure_seconds + non_exposing_seconds);
        let duty_cycle_percent = if total_time_seconds > 0.0 {
            (total_exposure_seconds / total_time_seconds * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let largest_gap = Self::largest_gap(&valid_rows);

        Some(LightframeStats {
            total_count,
            valid_count,
            error_count,
            total_exposure_seconds,
            total_time_seconds,
            non_exposing_seconds,
            duty_cycle_percent,
            largest_gap,
        })
    }

    fn capture_span_seconds(rows: &[&FrameInfo]) -> Option<f32> {
        let mut times = rows
            .iter()
            .filter_map(|r| r.creation_time)
            .collect::<Vec<_>>();

        if times.len() < 2 {
            return None;
        }

        times.sort();
        let first = times.first()?;
        let last = times.last()?;
        Some((last.signed_duration_since(*first).num_seconds() as f32).max(0.0))
    }

    fn largest_gap(rows: &[&FrameInfo]) -> Option<LargestGap> {
        let mut with_time = rows
            .iter()
            .filter_map(|r| r.creation_time.map(|dt| (*r, dt)))
            .collect::<Vec<_>>();

        if with_time.len() < 2 {
            return None;
        }
        with_time.sort_by_key(|(_, dt)| *dt);

        let mut best: Option<LargestGap> = None;
        for window in with_time.windows(2) {
            let (prev_row, prev_dt) = window[0];
            let (next_row, next_dt) = window[1];
            let dt = next_dt.signed_duration_since(prev_dt).num_seconds() as f32;
            let gap = (dt - prev_row.exposure_seconds.unwrap_or(0.0)).max(0.0);

            match &best {
                Some(cur) if cur.seconds >= gap => {}
                _ => {
                    best = Some(LargestGap {
                        from: prev_row.filename.clone(),
                        to: next_row.filename.clone(),
                        seconds: gap,
                    });
                }
            }
        }

        best
    }

    fn reset_processing_state_for_selection(&mut self) {
        self.processing_updated(ProcessingCounters {
            count_lights: self.lightframes.len(),
            count_darks: self.darkframes.len(),
            count_loaded: 0,
            count_loading: 0,
            count_merged: 0,
            count_merging: 0,
        });
        self.processing_running = false;
        self.preview = None;
    }
}

impl MetadataStatusSink for TrawlsDummy {
    fn metadata_queued(&mut self, frame_id: &FrameIdentifier) {
        if let Some(row) = find_frame_mut(self, frame_id) {
            row.metadata_status = MetadataStatus::Pending;
            row.error = None;
        }
    }

    fn metadata_loaded(&mut self, frame_id: &FrameIdentifier, metadata: FrameMetadata) {
        if let Some(row) = self
            .lightframes
            .iter_mut()
            .find(|row| row.path == frame_id.as_path().to_string_lossy())
        {
            row.exposure_seconds = Some(metadata.exposure_seconds);
            row.interval_seconds = metadata.interval_seconds;
            row.aperture = Some(metadata.aperture);
            row.iso = Some(metadata.iso);
            row.creation_time = metadata.creation_time;
            row.metadata_status = MetadataStatus::Loaded;
            row.error = None;
            recompute_intervals(&mut self.lightframes);
            return;
        }

        if let Some(row) = self
            .darkframes
            .iter_mut()
            .find(|row| row.path == frame_id.as_path().to_string_lossy())
        {
            row.exposure_seconds = Some(metadata.exposure_seconds);
            row.interval_seconds = metadata.interval_seconds;
            row.aperture = Some(metadata.aperture);
            row.iso = Some(metadata.iso);
            row.creation_time = metadata.creation_time;
            row.metadata_status = MetadataStatus::Loaded;
            row.error = None;
            recompute_intervals(&mut self.darkframes);
        }
    }

    fn metadata_failed(&mut self, frame_id: &FrameIdentifier, error: String) {
        if let Some(row) = find_frame_mut(self, frame_id) {
            row.metadata_status = MetadataStatus::Failed;
            row.error = Some(error);
        }
    }
}

impl ProcessingStatusSink for TrawlsDummy {
    fn processing_updated(&mut self, counters: ProcessingCounters) {
        self.processing.count_lights = counters.count_lights;
        self.processing.count_darks = counters.count_darks;
        self.processing.count_loaded = counters.count_loaded;
        self.processing.count_loading = counters.count_loading;
        self.processing.count_merged = counters.count_merged;
        self.processing.count_merging = counters.count_merging;
    }
}

fn frame_rank(frame: &FrameInfo) -> u8 {
    match frame.metadata_status {
        MetadataStatus::Failed => 0,
        MetadataStatus::Pending => 1,
        MetadataStatus::Loaded => 2,
    }
}

fn find_frame_mut<'a>(
    app: &'a mut TrawlsDummy,
    frame_id: &FrameIdentifier,
) -> Option<&'a mut FrameInfo> {
    let target = frame_id.as_path().to_string_lossy();
    app.lightframes
        .iter_mut()
        .find(|row| row.path == target)
        .or_else(|| app.darkframes.iter_mut().find(|row| row.path == target))
}

fn mock_metadata_delay_ms(path: &str, index: usize) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    150 + ((hash.wrapping_add((index as u64) * 97)) % 850)
}

fn recompute_intervals(frames: &mut [FrameInfo]) {
    for row in frames.iter_mut() {
        row.interval_seconds = None;
    }

    let mut ordered = frames
        .iter()
        .enumerate()
        .filter_map(|(ix, row)| {
            Some((
                ix,
                row.creation_time?,
                row.exposure_seconds?,
                row.metadata_status == MetadataStatus::Loaded && row.error.is_none(),
            ))
        })
        .filter(|(_, _, _, ok)| *ok)
        .map(|(ix, time, exposure, _)| (ix, time, exposure))
        .collect::<Vec<_>>();

    if ordered.is_empty() {
        return;
    }

    ordered.sort_by_key(|(_, time, _)| *time);
    for (position, (ix, time, exposure)) in ordered.iter().enumerate() {
        if position == 0 {
            frames[*ix].interval_seconds = None;
            continue;
        }

        let (_, prev_time, _) = ordered[position - 1];
        let dt = time.signed_duration_since(prev_time).num_seconds() as f32;
        let interval = (dt - *exposure).max(0.0);
        frames[*ix].interval_seconds = Some(interval);
    }
}
