use crate::ui::model::{FrameInfo, LargestGap, LightframeStats, MetadataStatus, SortKey};

use super::super::Trawls;

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
        let total_time_seconds = capture_span_seconds(&valid_rows)
            .unwrap_or(total_exposure_seconds + non_exposing_seconds);
        let duty_cycle_percent = if total_time_seconds > 0.0 {
            (total_exposure_seconds / total_time_seconds * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let largest_gap = largest_gap(&valid_rows);

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
}

fn frame_rank(frame: &FrameInfo) -> u8 {
    match frame.metadata_status {
        MetadataStatus::Failed => 0,
        MetadataStatus::Pending => 1,
        MetadataStatus::Loaded => 2,
    }
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

pub(crate) fn recompute_intervals(frames: &mut [FrameInfo]) {
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    pub(crate) fn make_frame(filename: &str, status: MetadataStatus) -> FrameInfo {
        FrameInfo {
            path: format!("/test/{filename}"),
            filename: filename.to_string(),
            exposure_seconds: None,
            interval_seconds: None,
            aperture: None,
            iso: None,
            creation_time: None,
            metadata_status: status,
            error: None,
            thumbnail: None,
        }
    }

    pub(crate) fn make_loaded_frame(
        filename: &str,
        exposure: f32,
        aperture: f32,
        iso: u32,
        time_str: &str,
    ) -> FrameInfo {
        FrameInfo {
            path: format!("/test/{filename}"),
            filename: filename.to_string(),
            exposure_seconds: Some(exposure),
            interval_seconds: None,
            aperture: Some(aperture),
            iso: Some(iso),
            creation_time: Some(
                NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S").unwrap(),
            ),
            metadata_status: MetadataStatus::Loaded,
            error: None,
            thumbnail: None,
        }
    }

    #[test]
    fn frame_rank_ordering() {
        let failed = make_frame("a.arw", MetadataStatus::Failed);
        let pending = make_frame("b.arw", MetadataStatus::Pending);
        let loaded = make_frame("c.arw", MetadataStatus::Loaded);
        assert_eq!(frame_rank(&failed), 0);
        assert_eq!(frame_rank(&pending), 1);
        assert_eq!(frame_rank(&loaded), 2);
    }

    #[test]
    fn sorted_frames_by_filename() {
        let frames = vec![
            make_frame("c.arw", MetadataStatus::Loaded),
            make_frame("a.arw", MetadataStatus::Loaded),
            make_frame("b.arw", MetadataStatus::Loaded),
        ];
        let sorted = Trawls::sorted_frames(&frames, SortKey::Filename);
        assert_eq!(sorted[0].filename, "a.arw");
        assert_eq!(sorted[1].filename, "b.arw");
        assert_eq!(sorted[2].filename, "c.arw");
    }

    #[test]
    fn sorted_frames_errors_first() {
        let mut error_frame = make_frame("z.arw", MetadataStatus::Failed);
        error_frame.error = Some("bad".to_string());
        let loaded = make_frame("a.arw", MetadataStatus::Loaded);
        let pending = make_frame("b.arw", MetadataStatus::Pending);
        let sorted = Trawls::sorted_frames(&[error_frame, loaded, pending], SortKey::Filename);
        assert_eq!(sorted[0].filename, "z.arw");
        assert_eq!(sorted[1].filename, "b.arw");
        assert_eq!(sorted[2].filename, "a.arw");
    }

    #[test]
    fn sorted_frames_by_creation_time() {
        let frames = vec![
            make_loaded_frame("c.arw", 20.0, 2.8, 1600, "2024-01-01 03:00:00"),
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 02:00:00"),
        ];
        let sorted = Trawls::sorted_frames(&frames, SortKey::CreationTime);
        assert_eq!(sorted[0].filename, "a.arw");
        assert_eq!(sorted[1].filename, "b.arw");
        assert_eq!(sorted[2].filename, "c.arw");
    }

    #[test]
    fn recompute_intervals_empty() {
        let mut frames: Vec<FrameInfo> = vec![];
        recompute_intervals(&mut frames);
    }

    #[test]
    fn recompute_intervals_single_frame() {
        let mut frames = vec![make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00")];
        recompute_intervals(&mut frames);
        assert!(frames[0].interval_seconds.is_none());
    }

    #[test]
    fn recompute_intervals_two_frames() {
        let mut frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
        ];
        recompute_intervals(&mut frames);
        assert!(frames[0].interval_seconds.is_none());
        assert!((frames[1].interval_seconds.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn recompute_intervals_out_of_order() {
        let mut frames = vec![
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
        ];
        recompute_intervals(&mut frames);
        assert!(frames[1].interval_seconds.is_none());
        assert!((frames[0].interval_seconds.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn recompute_intervals_ignores_pending() {
        let mut frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_frame("b.arw", MetadataStatus::Pending),
            make_loaded_frame("c.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
        ];
        recompute_intervals(&mut frames);
        assert!(frames[0].interval_seconds.is_none());
        assert!(frames[1].interval_seconds.is_none());
        assert!((frames[2].interval_seconds.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn lightframe_stats_empty() {
        assert!(Trawls::lightframe_stats(&[]).is_none());
    }

    #[test]
    fn lightframe_stats_only_pending() {
        assert!(Trawls::lightframe_stats(&[make_frame("a.arw", MetadataStatus::Pending)]).is_none());
    }

    #[test]
    fn lightframe_stats_basic() {
        let frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
            make_loaded_frame("c.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:50"),
        ];
        let stats = Trawls::lightframe_stats(&frames).unwrap();
        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.valid_count, 3);
        assert_eq!(stats.error_count, 0);
        assert!((stats.total_exposure_seconds - 60.0).abs() < 0.01);
        assert!((stats.total_time_seconds - 50.0).abs() < 0.01);
    }

    #[test]
    fn lightframe_stats_with_errors() {
        let mut error_frame = make_frame("bad.arw", MetadataStatus::Failed);
        error_frame.error = Some("corrupt".to_string());
        let frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            error_frame,
            make_loaded_frame("c.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
        ];
        let stats = Trawls::lightframe_stats(&frames).unwrap();
        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.valid_count, 2);
        assert_eq!(stats.error_count, 1);
    }
}
