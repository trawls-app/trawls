use crate::model::FrameInfo;

pub fn sample_lightframes() -> Vec<FrameInfo> {
    vec![
        FrameInfo {
            path: "/mock/lights/frame_0001.ARW",
            filename: "frame_0001.ARW",
            exposure_seconds: 20.0,
            interval_seconds: None,
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T22:00:03",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0002.ARW",
            filename: "frame_0002.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T22:00:24",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0003.ARW",
            filename: "frame_0003.ARW",
            exposure_seconds: 25.0,
            interval_seconds: Some(9.5),
            aperture: 3.2,
            iso: 1600,
            creation_time: "2026-01-11T22:00:58",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0004.ARW",
            filename: "frame_0004.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 2000,
            creation_time: "2026-01-11T22:01:19",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0005.ARW",
            filename: "frame_0005.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T22:01:40",
            error: Some("Could not parse EXIF metadata"),
        },
    ]
}

pub fn sample_darkframes() -> Vec<FrameInfo> {
    vec![
        FrameInfo {
            path: "/mock/darks/dark_001.ARW",
            filename: "dark_001.ARW",
            exposure_seconds: 20.0,
            interval_seconds: None,
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T21:30:11",
            error: None,
        },
        FrameInfo {
            path: "/mock/darks/dark_002.ARW",
            filename: "dark_002.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T21:30:32",
            error: None,
        },
    ]
}
