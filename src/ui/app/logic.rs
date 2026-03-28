use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use gpui::{Context, PathPromptOptions, Timer, Window};
use gpui_component::WindowExt as _;

use crate::processing::{
    interface::FrameIdentifier,
    merge::{Comets, run_merge},
    metadata::{FrameMetadata, FrameMetadataLoader, RawlerMetadataLoader},
    status::{
        MetadataStatus as MetadataStatusSink, ProcessingCounters,
        ProcessingStatus as ProcessingStatusSink,
    },
};
use crate::ui::{
    model::{
        FrameInfo, LargestGap, LightframeStats, MetadataStatus, PreviewData, SortKey,
    },
};

use super::Trawls;

impl Trawls {
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

    pub(crate) fn choose_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let default_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let path = cx.prompt_for_new_path(&default_dir, Some("trawls_output.dng"));
        let view = cx.entity();
        cx.spawn_in(window, async move |_, window| {
            let path = path.await.ok()?.ok()??;
            window
                .update(|window, cx| {
                    _ = view.update(cx, |this, cx| {
                        let as_text = path.to_string_lossy().to_string();
                        this.output_path = Some(as_text.clone().into());
                        this.output_input.update(cx, |state, cx| {
                            state.set_value(as_text, window, cx);
                        });
                        cx.notify();
                    });
                })
                .ok();
            Some(())
        })
        .detach();
    }

    pub(crate) fn clear_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.output_path = None;
        self.output_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        cx.notify();
    }

    pub(crate) fn clear_frames(&mut self, is_light: bool, cx: &mut Context<Self>) {
        if is_light {
            self.lightframes.clear();
        } else {
            self.darkframes.clear();
        }
        self.selected_paths_mut(is_light).clear();
        cx.notify();
    }

    pub(crate) fn inspect_frame(&mut self, path: String, window: &Window, cx: &mut Context<Self>) {
        let filename = PathBuf::from(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Find the frame to build an EXIF summary
        let frame = self
            .lightframes
            .iter()
            .chain(self.darkframes.iter())
            .find(|f| f.path == path);
        let exif_summary = frame
            .map(|f| {
                let mut parts = Vec::new();
                if let Some(iso) = f.iso {
                    parts.push(format!("ISO {iso}"));
                }
                if let Some(aperture) = f.aperture {
                    parts.push(format!("f/{aperture:.1}"));
                }
                if let Some(exposure) = f.exposure_seconds {
                    parts.push(format!("{exposure:.1}s"));
                }
                if let Some(dt) = f.creation_time {
                    parts.push(dt.format("%Y-%m-%d %H:%M:%S").to_string());
                }
                parts.join(", ")
            })
            .unwrap_or_default();

        self.frame_preview = None;
        self.frame_preview_loading = true;
        self.frame_preview_viewer = Default::default();
        let viewport_size = window.viewport_size();
        cx.notify();

        let path_clone = path.clone();
        cx.spawn(async move |this, cx| {
            let result = thread::spawn(move || {
                crate::processing::thumbnail::extract_preview(Path::new(&path_clone))
            })
            .join()
            .map_err(|_| anyhow::anyhow!("thread panicked"))
            .and_then(|r| r);

            let _ = this.update(cx, |this, cx| {
                this.frame_preview_loading = false;
                if let Ok(preview) = result {
                    this.frame_preview_viewer.fit_to_viewport(
                        viewport_size,
                        preview.width_px,
                        preview.height_px,
                    );

                    this.frame_preview = Some(crate::ui::model::FramePreview {
                        filename,
                        image: preview.image,
                        width_px: preview.width_px,
                        height_px: preview.height_px,
                        exif_summary,
                    });
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn close_frame_preview(&mut self, cx: &mut Context<Self>) {
        self.frame_preview = None;
        self.frame_preview_loading = false;
        cx.notify();
    }

    pub(crate) fn remove_selected_frames(&mut self, is_light: bool, cx: &mut Context<Self>) {
        let selected = self.selected_paths(is_light).clone();
        let frames = if is_light {
            &mut self.lightframes
        } else {
            &mut self.darkframes
        };
        frames.retain(|f| !selected.contains(f.path.as_str()));
        recompute_intervals(frames);
        self.selected_paths_mut(is_light).clear();
        cx.notify();
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
                .update(|window, cx| {
                    _ = view.update(cx, |this, cx| {
                        if is_light && this.output_path.is_none() {
                            if let Some(dir) = files.first().and_then(|f| f.parent()) {
                                let default_output = dir.join("trawls_output.dng");
                                let as_text = default_output.to_string_lossy().to_string();
                                this.output_path = Some(as_text.clone().into());
                                this.output_input.update(cx, |state, cx| {
                                    state.set_value(as_text, window, cx);
                                });
                            }
                        }
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
        let same_list = if is_light { &self.lightframes } else { &self.darkframes };
        let other_list = if is_light { &self.darkframes } else { &self.lightframes };
        let existing_paths: std::collections::HashSet<String> =
            same_list.iter().map(|row| row.path.clone()).collect();
        let other_paths: std::collections::HashMap<String, &FrameInfo> = other_list
            .iter()
            .map(|row| (row.path.clone(), row))
            .collect();

        let mut new_files = Vec::new();
        let other_label = if is_light { "darkframe" } else { "lightframe" };
        let new_rows = files
            .into_iter()
            .filter(|path| !existing_paths.contains(path.to_string_lossy().as_ref()))
            .map(|path| {
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown");
                let path_string = path.to_string_lossy().to_string();

                // If the file is already loaded in the other list, copy its data
                if let Some(existing) = other_paths.get(&path_string) {
                    let mut row = (*existing).clone();
                    row.error = Some(format!("Also used as {other_label}"));
                    return row;
                }

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
                    thumbnail: None,
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

        // Metadata loading — parallel, fast (EXIF only)
        let (meta_tx, meta_rx) = mpsc::channel::<MetadataLoadResult>();
        let meta_files = new_files.clone();
        thread::spawn(move || {
            use rayon::prelude::*;
            meta_files.into_par_iter().for_each_with(meta_tx, |tx, path| {
                let frame_id = FrameIdentifier::new(path);
                let metadata = RawlerMetadataLoader.load_metadata(&frame_id);
                let _ = tx.send(MetadataLoadResult { frame_id, metadata });
            });
        });
        cx.spawn(async move |this, cx| {
            poll_channel(meta_rx, |this, cx, result| {
                match result.metadata {
                    Ok(metadata) => this.metadata_loaded(&result.frame_id, metadata),
                    Err(error) => this.metadata_failed(&result.frame_id, error.to_string()),
                }
                cx.notify();
            }, this, cx).await;
        })
        .detach();

        // Thumbnail loading — parallel, slower (JPEG extraction + resize)
        let (thumb_tx, thumb_rx) = mpsc::channel::<ThumbnailLoadResult>();
        let thumb_files = new_files;
        thread::spawn(move || {
            use rayon::prelude::*;
            thumb_files.into_par_iter().for_each_with(thumb_tx, |tx, path| {
                let frame_id = FrameIdentifier::new(path);
                let thumbnail = crate::processing::thumbnail::extract_thumbnail(frame_id.as_path())
                    .map_err(|e| log::warn!("Thumbnail extraction failed for {:?}: {e}", frame_id.as_path()))
                    .ok();
                if let Some(thumb) = thumbnail {
                    let _ = tx.send(ThumbnailLoadResult { frame_id, thumbnail: thumb });
                }
            });
        });
        cx.spawn(async move |this, cx| {
            poll_channel(thumb_rx, |this, _, result| {
                if let Some(row) = find_frame_mut(this, &result.frame_id) {
                    row.thumbnail = Some(result.thumbnail);
                }
            }, this, cx).await;
        })
        .detach();
    }

    pub(crate) fn start_processing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.processing_running {
            return;
        }

        let output_path = match self.output_path.as_ref().map(|s| resolve_path(s.as_ref())) {
            Some(path) => path,
            None => {
                window.push_notification("Select an output path first.", cx);
                return;
            }
        };

        if self
            .lightframes
            .iter()
            .chain(self.darkframes.iter())
            .any(|row| row.metadata_status == MetadataStatus::Pending)
        {
            window.push_notification("Metadata is still loading.", cx);
            return;
        }
        if self
            .lightframes
            .iter()
            .chain(self.darkframes.iter())
            .any(|row| row.error.is_some())
        {
            window.push_notification("Some selected files have metadata errors.", cx);
            return;
        }
        if self.lightframes.len() < 2 {
            window.push_notification("Select at least two lightframes.", cx);
            return;
        }

        let light_paths = self
            .lightframes
            .iter()
            .map(|f| PathBuf::from(&f.path))
            .collect::<Vec<_>>();
        let dark_paths = self
            .darkframes
            .iter()
            .map(|f| PathBuf::from(&f.path))
            .collect::<Vec<_>>();
        let frame_count = self.lightframes.len();
        let total_exposure: f32 = self
            .lightframes
            .iter()
            .filter_map(|f| f.exposure_seconds)
            .sum();
        let ui_merge_mode = self.merge_mode;
        let mode = match self.merge_mode {
            crate::ui::model::MergeMode::Normal => Comets::Normal,
            crate::ui::model::MergeMode::Falling => Comets::Falling,
            crate::ui::model::MergeMode::Raising => Comets::Raising,
        };

        self.processing = ProcessingCounters {
            count_lights: self.lightframes.len(),
            count_darks: self.darkframes.len(),
            ..Default::default()
        };
        self.processing_running = true;
        self.preview = None;
        let viewport_size = window.viewport_size();
        let view = cx.entity();
        let (progress_tx, progress_rx) = mpsc::channel::<ProcessingCounters>();
        let (done_tx, done_rx) = mpsc::channel::<anyhow::Result<PreviewData>>();

        thread::spawn(move || {
            let status = Arc::new(Mutex::new(ChannelProcessingStatus { tx: progress_tx }));
            let result = run_merge(light_paths, dark_paths, mode, status).and_then(|image| {
                let writer = image.get_image_writer()?;
                writer.write_dng(output_path.clone())?;
                let output_file_size = std::fs::metadata(&output_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                let preview_path = output_path.with_extension("preview.jpg");
                writer.write_preview_jpg(preview_path.clone())?;
                let (width_px, height_px) = writer.preview_dimensions();
                let exif = writer.preview_exif_summary();
                Ok(PreviewData {
                    image_path: preview_path,
                    exif,
                    width_px,
                    height_px,
                    frame_count,
                    total_exposure_seconds: total_exposure,
                    merge_mode: ui_merge_mode,
                    output_file_size,
                })
            });
            let _ = done_tx.send(result);
        });

        cx.spawn_in(window, async move |_, window| {
            loop {
                let mut latest = None;
                while let Ok(counters) = progress_rx.try_recv() {
                    latest = Some(counters);
                }
                if let Some(counters) = latest {
                    _ = view.update_in(window, |this, _, cx| {
                        this.processing_updated(counters);
                        cx.notify();
                    });
                }

                match done_rx.try_recv() {
                    Ok(result) => {
                        _ = view.update_in(window, |this, window, cx| {
                            this.processing_running = false;
                            match result {
                                Ok(preview) => {
                                    this.preview_viewer.fit_to_viewport(
                                        viewport_size,
                                        preview.width_px,
                                        preview.height_px,
                                    );
                                    this.preview = Some(preview);
                                    this.active_step = 3;
                                    window.push_notification("Processing finished.", cx);
                                }
                                Err(err) => {
                                    this.preview = None;
                                    window
                                        .push_notification(format!("Processing failed: {err}"), cx);
                                }
                            }
                            cx.notify();
                        });
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => break,
                }
                Timer::after(Duration::from_millis(50)).await;
            }
        })
        .detach();

        window.push_notification("Processing started.", cx);
        cx.notify();
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

impl MetadataStatusSink for Trawls {
    fn metadata_queued(&mut self, frame_id: &FrameIdentifier) {
        if let Some(row) = find_frame_mut(self, frame_id) {
            row.metadata_status = MetadataStatus::Pending;
            row.error = None;
        }
    }

    fn metadata_loaded(&mut self, frame_id: &FrameIdentifier, metadata: FrameMetadata) {
        let target = frame_id.as_path().to_string_lossy();
        let is_light = self.lightframes.iter().any(|row| row.path == target);

        let Some(row) = find_frame_mut(self, frame_id) else {
            return;
        };
        row.exposure_seconds = Some(metadata.exposure_seconds);
        row.interval_seconds = metadata.interval_seconds;
        row.aperture = Some(metadata.aperture);
        row.iso = Some(metadata.iso);
        row.creation_time = metadata.creation_time;
        row.metadata_status = MetadataStatus::Loaded;
        row.error = None;

        recompute_intervals(if is_light {
            &mut self.lightframes
        } else {
            &mut self.darkframes
        });
    }

    fn metadata_failed(&mut self, frame_id: &FrameIdentifier, error: String) {
        if let Some(row) = find_frame_mut(self, frame_id) {
            row.metadata_status = MetadataStatus::Failed;
            row.error = Some(error);
        }
    }
}

impl ProcessingStatusSink for Trawls {
    fn processing_updated(&mut self, counters: ProcessingCounters) {
        self.processing = counters;
    }
}

struct ChannelProcessingStatus {
    tx: mpsc::Sender<ProcessingCounters>,
}

impl ProcessingStatusSink for ChannelProcessingStatus {
    fn processing_updated(&mut self, counters: ProcessingCounters) {
        let _ = self.tx.send(counters);
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
    app: &'a mut Trawls,
    frame_id: &FrameIdentifier,
) -> Option<&'a mut FrameInfo> {
    let target = frame_id.as_path().to_string_lossy();
    app.lightframes
        .iter_mut()
        .find(|row| row.path == target)
        .or_else(|| app.darkframes.iter_mut().find(|row| row.path == target))
}

fn resolve_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home).join(stripped);
        }
    }
    PathBuf::from(path)
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

struct MetadataLoadResult {
    frame_id: FrameIdentifier,
    metadata: anyhow::Result<FrameMetadata>,
}

struct ThumbnailLoadResult {
    frame_id: FrameIdentifier,
    thumbnail: Arc<gpui::Image>,
}

async fn poll_channel<T: 'static>(
    rx: mpsc::Receiver<T>,
    mut apply: impl FnMut(&mut Trawls, &mut Context<Trawls>, T),
    this: gpui::WeakEntity<Trawls>,
    cx: &mut gpui::AsyncApp,
) {
    loop {
        let mut got_any = false;
        while let Ok(item) = rx.try_recv() {
            got_any = true;
            let _ = this.update(cx, |this, cx| {
                apply(this, cx, item);
                cx.notify();
            });
        }
        if got_any {
            continue;
        }
        if rx.try_recv().is_err_and(|e| e == mpsc::TryRecvError::Disconnected) {
            break;
        }
        Timer::after(Duration::from_millis(30)).await;
    }
}
