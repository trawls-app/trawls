use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use gpui::{Context, Timer, Window};
use gpui_component::WindowExt as _;

use crate::processing::{
    merge::{Comets, run_merge},
    status::{ProcessingCounters, ProcessingStatus as ProcessingStatusSink},
};
use crate::ui::model::{MetadataStatus, PreviewData};

use super::super::Trawls;

impl Trawls {
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
                let (image, width_px, height_px) = writer.preview_as_gpui_image()?;
                let exif = writer.preview_exif_summary();
                Ok(PreviewData {
                    image,
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

    pub(super) fn reset_processing_state_for_selection(&mut self) {
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

fn resolve_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home).join(stripped);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_absolute() {
        assert_eq!(resolve_path("/some/path").to_str().unwrap(), "/some/path");
    }

    #[test]
    fn resolve_path_tilde() {
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(
                resolve_path("~/test/file.dng").to_str().unwrap(),
                format!("{home}/test/file.dng")
            );
        }
    }

    #[test]
    fn resolve_path_relative() {
        assert_eq!(
            resolve_path("relative/path").to_str().unwrap(),
            "relative/path"
        );
    }
}
