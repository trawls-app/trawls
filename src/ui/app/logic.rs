use chrono::NaiveDateTime;
use gpui::{Context, ParentElement as _, Pixels, Point, Styled as _, Window, div, point, px};
use gpui_component::{
    WindowExt as _,
    button::{Button, ButtonVariants as _},
    v_flex,
};

use crate::processing::{
    dummy::DUMMY_PROCESSOR,
    interface::{ProcessingInput, Processor},
};
use crate::ui::model::{FrameInfo, LargestGap, LightframeStats, SortKey};

use super::TrawlsDummy;

const MIN_PREVIEW_ZOOM: f32 = 0.02;
const MAX_PREVIEW_ZOOM: f32 = 8.0;

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
        let result = DUMMY_PROCESSOR.start(ProcessingInput {
            lightframes: &self.lightframes,
            darkframes: &self.darkframes,
            output_path_set: self.output_path.is_some(),
            has_metadata_errors: Self::error_warning(&self.lightframes)
                || Self::error_warning(&self.darkframes),
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
        self.preview = result.preview;
        self.preview_zoom = self.fit_preview_zoom_for_window(window);
        self.active_step = 3;

        window.push_notification("Dummy processing finished and preview generated.", cx);
        cx.notify();
    }

    pub(crate) fn advance_dummy_progress(&mut self, cx: &mut Context<Self>) {
        DUMMY_PROCESSOR.advance(&mut self.processing);
        cx.notify();
    }

    pub(crate) fn set_preview_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        self.preview_zoom = zoom.clamp(MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM);
        cx.notify();
    }

    pub(crate) fn zoom_preview_in(&mut self, cx: &mut Context<Self>) {
        // Multiplicative steps keep zoom useful across tiny and huge scales.
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

        // Keep the image pixel under the cursor fixed while changing zoom.
        // In GPUI, scroll offsets are typically negative as you scroll down/right.
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
        self.preview_zoom = self.fit_preview_zoom_for_window(window);
        cx.notify();
    }

    fn fit_preview_zoom_for_window(&self, window: &Window) -> f32 {
        let Some(preview) = &self.preview else {
            return 1.0;
        };

        let viewport = window.viewport_size();
        // Reserve space for sidebar, title/header, paddings and preview controls.
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
