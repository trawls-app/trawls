use anyhow::{Result, anyhow};
use chrono::NaiveDateTime;

use crate::processing::{
    interface::FrameIdentifier,
    metadata::{FrameMetadata, FrameMetadataLoader},
};

pub static DUMMY_METADATA_PROCESSOR: DummyMetadataProcessor = DummyMetadataProcessor;

pub struct DummyMetadataProcessor;

impl FrameMetadataLoader for DummyMetadataProcessor {
    fn load_metadata(&self, frame_id: &FrameIdentifier) -> Result<FrameMetadata> {
        match frame_id.as_path().to_string_lossy().as_ref() {
            "/mock/lights/frame_0001.ARW" => Ok(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: None,
                aperture: 2.8,
                iso: 1600,
                creation_time: Some(parse_datetime("2026-01-11T22:00:03")),
            }),
            "/mock/lights/frame_0002.ARW" => Ok(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: Some(1.0),
                aperture: 2.8,
                iso: 1600,
                creation_time: Some(parse_datetime("2026-01-11T22:00:24")),
            }),
            "/mock/lights/frame_0003.ARW" => Ok(FrameMetadata {
                exposure_seconds: 25.0,
                interval_seconds: Some(9.5),
                aperture: 3.2,
                iso: 1600,
                creation_time: Some(parse_datetime("2026-01-11T22:00:58")),
            }),
            "/mock/lights/frame_0004.ARW" => Ok(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: Some(1.0),
                aperture: 2.8,
                iso: 2000,
                creation_time: Some(parse_datetime("2026-01-11T22:01:19")),
            }),
            "/mock/lights/frame_0005.ARW" => Err(anyhow!("Could not parse EXIF metadata")),
            "/mock/darks/dark_001.ARW" => Ok(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: None,
                aperture: 2.8,
                iso: 1600,
                creation_time: Some(parse_datetime("2026-01-11T21:30:11")),
            }),
            "/mock/darks/dark_002.ARW" => Ok(FrameMetadata {
                exposure_seconds: 20.0,
                interval_seconds: Some(1.0),
                aperture: 2.8,
                iso: 1600,
                creation_time: Some(parse_datetime("2026-01-11T21:30:32")),
            }),
            _ => Err(anyhow!("Unknown mock frame path")),
        }
    }
}

fn parse_datetime(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").expect("valid sample datetime")
}
