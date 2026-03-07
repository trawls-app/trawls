use crate::{
    processing::status::FrameMetadata,
    ui::model::{FrameInfo, MetadataStatus},
};

#[derive(Clone)]
pub struct MockFrameSeed {
    pub path: &'static str,
    pub filename: &'static str,
    pub metadata: Option<FrameMetadata>,
    pub error: Option<&'static str>,
}

impl MockFrameSeed {
    pub fn as_pending_row(&self) -> FrameInfo {
        FrameInfo {
            path: self.path.to_string(),
            filename: self.filename.to_string(),
            exposure_seconds: None,
            interval_seconds: None,
            aperture: None,
            iso: None,
            creation_time: None,
            metadata_status: MetadataStatus::Pending,
            error: None,
        }
    }
}

pub fn sample_lightframe_seeds() -> Vec<MockFrameSeed> {
    vec![
        MockFrameSeed {
            path: "/mock/lights/frame_0001.ARW",
            filename: "frame_0001.ARW",
            metadata: Some(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: None,
                aperture: 2.8,
                iso: 1600,
                creation_time: "2026-01-11T22:00:03".to_string(),
            }),
            error: None,
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0002.ARW",
            filename: "frame_0002.ARW",
            metadata: Some(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: Some(1.0),
                aperture: 2.8,
                iso: 1600,
                creation_time: "2026-01-11T22:00:24".to_string(),
            }),
            error: None,
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0003.ARW",
            filename: "frame_0003.ARW",
            metadata: Some(FrameMetadata {
                exposure_seconds: 25.0,
                interval_seconds: Some(9.5),
                aperture: 3.2,
                iso: 1600,
                creation_time: "2026-01-11T22:00:58".to_string(),
            }),
            error: None,
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0004.ARW",
            filename: "frame_0004.ARW",
            metadata: Some(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: Some(1.0),
                aperture: 2.8,
                iso: 2000,
                creation_time: "2026-01-11T22:01:19".to_string(),
            }),
            error: None,
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0005.ARW",
            filename: "frame_0005.ARW",
            metadata: None,
            error: Some("Could not parse EXIF metadata"),
        },
    ]
}

pub fn sample_darkframe_seeds() -> Vec<MockFrameSeed> {
    vec![
        MockFrameSeed {
            path: "/mock/darks/dark_001.ARW",
            filename: "dark_001.ARW",
            metadata: Some(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: None,
                aperture: 2.8,
                iso: 1600,
                creation_time: "2026-01-11T21:30:11".to_string(),
            }),
            error: None,
        },
        MockFrameSeed {
            path: "/mock/darks/dark_002.ARW",
            filename: "dark_002.ARW",
            metadata: Some(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: Some(1.0),
                aperture: 2.8,
                iso: 1600,
                creation_time: "2026-01-11T21:30:32".to_string(),
            }),
            error: None,
        },
    ]
}
