use chrono::NaiveDateTime;
use gpui::{Context, ParentElement as _, Styled as _, Window, div};
use gpui_component::{
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};

use crate::model::{FrameInfo, LargestGap, LightframeStats, PreviewData, ProcessingMock, SortKey};

use super::TrawlsDummy;

impl TrawlsDummy {
    pub(crate) fn sorted_frames(frames: &[FrameInfo], sort: SortKey) -> Vec<FrameInfo> {
        let mut rows: Vec<FrameInfo> = frames.to_vec();
        rows.sort_by(|a, b| {
            if a.error.is_some() && b.error.is_none() {
                return std::cmp::Ordering::Less;
            }
            if a.error.is_none() && b.error.is_some() {
                return std::cmp::Ordering::Greater;
            }

            match sort {
                SortKey::CreationTime => a.creation_time.cmp(b.creation_time),
                SortKey::Filename => a.filename.cmp(b.filename),
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

        for frame in frames.iter().filter(|f| f.error.is_none()) {
            apertures.insert(format!("{:.1}", frame.aperture));
            exposures.insert(format!("{:.1}", frame.exposure_seconds));
            isos.insert(frame.iso);
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

    pub(crate) fn run_dummy_processing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let issues = self.collect_readiness_issues();

        if !issues.is_empty() {
            window.open_dialog(cx, move |dialog, _, _| {
                dialog
                    .title("Can't start processing")
                    .child(
                        v_flex().gap_2().children(
                            issues
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

        self.update_processing_state();
        self.preview = Some(PreviewData {
            image_url: "https://images.unsplash.com/photo-1462331940025-496dfbfc7564?w=1600",
            exif: "ISO 1600, f/2.8, 20s",
        });
        self.active_step = 3;

        window.push_notification("Dummy processing finished and preview generated.", cx);
        cx.notify();
    }

    fn collect_readiness_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if Self::error_warning(&self.lightframes) || Self::error_warning(&self.darkframes) {
            issues.push("The metadata of some files could not be loaded.".to_string());
        }
        if self.lightframes.len() < 2 {
            issues.push("Less than two lightframes are selected.".to_string());
        }
        if self.output_path.is_none() {
            issues.push("No output path is specified.".to_string());
        }

        issues
    }

    fn update_processing_state(&mut self) {
        let count_lights = self.lightframes.len();
        let count_darks = self.darkframes.len();
        let merge_total = count_lights.saturating_sub(1) + count_darks.saturating_sub(1);

        self.processing = ProcessingMock {
            count_lights,
            count_darks,
            count_loaded: count_lights + count_darks,
            count_loading: 0,
            count_merged: merge_total.saturating_sub(1),
            count_merging: usize::from(merge_total > 0),
        };
    }

    pub(crate) fn advance_dummy_progress(&mut self, cx: &mut Context<Self>) {
        let loading_total = self.processing.count_lights + self.processing.count_darks;
        if self.processing.count_loaded < loading_total {
            self.processing.count_loading = 1;
            self.processing.count_loaded = (self.processing.count_loaded + 1).min(loading_total);
            if self.processing.count_loaded == loading_total {
                self.processing.count_loading = 0;
            }
        }

        let merge_total = self.processing.count_lights.saturating_sub(1)
            + self.processing.count_darks.saturating_sub(1);
        if self.processing.count_merged < merge_total {
            self.processing.count_merging = 1;
            self.processing.count_merged = (self.processing.count_merged + 1).min(merge_total);
            if self.processing.count_merged == merge_total {
                self.processing.count_merging = 0;
            }
        }

        cx.notify();
    }

    pub(crate) fn loading_percent(&self) -> f32 {
        let total = (self.processing.count_lights + self.processing.count_darks).max(1) as f32;
        let current =
            self.processing.count_loaded as f32 + (self.processing.count_loading as f32 * 0.5);
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
        let valid_rows: Vec<&FrameInfo> = rows.iter().filter(|r| r.error.is_none()).collect();
        if valid_rows.is_empty() {
            return None;
        }
        let valid_count = valid_rows.len();

        let total_exposure_seconds: f32 = valid_rows.iter().map(|r| r.exposure_seconds).sum();
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
            .filter_map(|r| {
                NaiveDateTime::parse_from_str(r.creation_time, "%Y-%m-%dT%H:%M:%S").ok()
            })
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
            .filter_map(|r| {
                NaiveDateTime::parse_from_str(r.creation_time, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .map(|dt| (*r, dt))
            })
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
            let gap = (dt - prev_row.exposure_seconds).max(0.0);

            match &best {
                Some(cur) if cur.seconds >= gap => {}
                _ => {
                    best = Some(LargestGap {
                        from: prev_row.filename.to_string(),
                        to: next_row.filename.to_string(),
                        seconds: gap,
                    });
                }
            }
        }

        best
    }
}
