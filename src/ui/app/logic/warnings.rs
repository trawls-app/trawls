use crate::ui::model::{FrameInfo, MetadataStatus};

use super::super::Trawls;

impl Trawls {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::logic::statistics::tests::{make_frame, make_loaded_frame};

    #[test]
    fn error_warning_no_errors() {
        assert!(!Trawls::error_warning(&[make_frame(
            "a.arw",
            MetadataStatus::Loaded
        )]));
    }

    #[test]
    fn error_warning_with_error() {
        let mut frame = make_frame("a.arw", MetadataStatus::Failed);
        frame.error = Some("bad".to_string());
        assert!(Trawls::error_warning(&[frame]));
    }

    #[test]
    fn camera_setting_warning_consistent() {
        let frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 02:00:00"),
        ];
        assert!(!Trawls::camera_setting_warning(&frames));
    }

    #[test]
    fn camera_setting_warning_different_iso() {
        let frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 3200, "2024-01-01 02:00:00"),
        ];
        assert!(Trawls::camera_setting_warning(&frames));
    }

    #[test]
    fn camera_setting_warning_different_exposure() {
        let frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 30.0, 2.8, 1600, "2024-01-01 02:00:00"),
        ];
        assert!(Trawls::camera_setting_warning(&frames));
    }

    #[test]
    fn camera_setting_warning_ignores_pending() {
        let frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_frame("b.arw", MetadataStatus::Pending),
        ];
        assert!(!Trawls::camera_setting_warning(&frames));
    }

    #[test]
    fn interval_warning_threshold_no_intervals() {
        assert!(
            Trawls::interval_warning_threshold(&[make_frame("a.arw", MetadataStatus::Loaded)])
                .is_none()
        );
    }

    #[test]
    fn interval_warning_threshold_with_intervals() {
        let mut frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
        ];
        frames[0].interval_seconds = Some(2.0);
        frames[1].interval_seconds = Some(4.0);
        let threshold = Trawls::interval_warning_threshold(&frames).unwrap();
        assert!((threshold - 4.0).abs() < 0.01);
    }

    #[test]
    fn interval_warning_none_above_threshold() {
        let mut frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
        ];
        frames[0].interval_seconds = Some(2.0);
        frames[1].interval_seconds = Some(2.0);
        assert!(!Trawls::interval_warning(&frames));
    }

    #[test]
    fn interval_warning_one_above_threshold() {
        let mut frames = vec![
            make_loaded_frame("a.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:00"),
            make_loaded_frame("b.arw", 20.0, 2.8, 1600, "2024-01-01 01:00:25"),
            make_loaded_frame("c.arw", 20.0, 2.8, 1600, "2024-01-01 01:01:00"),
        ];
        frames[0].interval_seconds = Some(2.0);
        frames[1].interval_seconds = Some(2.0);
        frames[2].interval_seconds = Some(20.0);
        assert!(Trawls::interval_warning(&frames));
    }
}
