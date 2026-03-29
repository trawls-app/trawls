use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use gpui::{Context, PathPromptOptions, Timer, Window};

use crate::processing::{
    interface::FrameIdentifier,
    metadata::{FrameMetadata, FrameMetadataLoader, RawlerMetadataLoader},
    status::MetadataStatus as MetadataStatusSink,
};
use crate::ui::model::{FrameInfo, MetadataStatus};

use super::super::Trawls;
use super::statistics::recompute_intervals;

impl Trawls {
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
            meta_files
                .into_par_iter()
                .for_each_with(meta_tx, |tx, path| {
                    let frame_id = FrameIdentifier::new(path);
                    let metadata = RawlerMetadataLoader.load_metadata(&frame_id);
                    let _ = tx.send(MetadataLoadResult { frame_id, metadata });
                });
        });
        cx.spawn(async move |this, cx| {
            poll_channel(
                meta_rx,
                |this, cx, result| {
                    match result.metadata {
                        Ok(metadata) => this.metadata_loaded(&result.frame_id, metadata),
                        Err(error) => this.metadata_failed(&result.frame_id, error.to_string()),
                    }
                    cx.notify();
                },
                this,
                cx,
            )
            .await;
        })
        .detach();

        // Thumbnail loading — parallel, slower (JPEG extraction + resize)
        let (thumb_tx, thumb_rx) = mpsc::channel::<ThumbnailLoadResult>();
        let thumb_files = new_files;
        thread::spawn(move || {
            use rayon::prelude::*;
            thumb_files
                .into_par_iter()
                .for_each_with(thumb_tx, |tx, path| {
                    let frame_id = FrameIdentifier::new(path);
                    let thumbnail =
                        crate::processing::thumbnail::extract_thumbnail(frame_id.as_path())
                            .map_err(|e| {
                                log::warn!(
                                    "Thumbnail extraction failed for {:?}: {e}",
                                    frame_id.as_path()
                                )
                            })
                            .ok();
                    if let Some(thumb) = thumbnail {
                        let _ = tx.send(ThumbnailLoadResult {
                            frame_id,
                            thumbnail: thumb,
                        });
                    }
                });
        });
        cx.spawn(async move |this, cx| {
            poll_channel(
                thumb_rx,
                |this, _, result| {
                    if let Some(row) = find_frame_mut(this, &result.frame_id) {
                        row.thumbnail = Some(result.thumbnail);
                    }
                },
                this,
                cx,
            )
            .await;
        })
        .detach();
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
        if rx
            .try_recv()
            .is_err_and(|e| e == mpsc::TryRecvError::Disconnected)
        {
            break;
        }
        Timer::after(Duration::from_millis(30)).await;
    }
}
