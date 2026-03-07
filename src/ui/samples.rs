use crate::ui::model::{FrameInfo, MetadataStatus};

#[derive(Clone)]
pub struct MockFrameSeed {
    pub path: &'static str,
    pub filename: &'static str,
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
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0002.ARW",
            filename: "frame_0002.ARW",
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0003.ARW",
            filename: "frame_0003.ARW",
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0004.ARW",
            filename: "frame_0004.ARW",
        },
        MockFrameSeed {
            path: "/mock/lights/frame_0005.ARW",
            filename: "frame_0005.ARW",
        },
    ]
}

pub fn sample_darkframe_seeds() -> Vec<MockFrameSeed> {
    vec![
        MockFrameSeed {
            path: "/mock/darks/dark_001.ARW",
            filename: "dark_001.ARW",
        },
        MockFrameSeed {
            path: "/mock/darks/dark_002.ARW",
            filename: "dark_002.ARW",
        },
    ]
}
