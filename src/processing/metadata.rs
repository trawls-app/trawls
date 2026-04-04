use std::{fs::File, io::BufReader, path::Path};

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use rawler::{RawFile, decoders::RawDecodeParams};

use crate::processing::interface::FrameIdentifier;

#[derive(Clone)]
pub struct FrameMetadata {
    pub exposure_seconds: f32,
    pub interval_seconds: Option<f32>,
    pub aperture: f32,
    pub iso: u32,
    pub creation_time: Option<NaiveDateTime>,
}

pub trait FrameMetadataLoader {
    fn load_metadata(&self, frame_id: &FrameIdentifier) -> Result<FrameMetadata>;
}

pub struct RawlerMetadataLoader;

impl FrameMetadataLoader for RawlerMetadataLoader {
    fn load_metadata(&self, frame_id: &FrameIdentifier) -> Result<FrameMetadata> {
        let path: &Path = frame_id.as_path();
        let file =
            File::open(path).with_context(|| format!("opening raw file {}", path.display()))?;
        let file_buffer = BufReader::new(file);
        let mut rawfile = RawFile::new(path, file_buffer);
        let decoder = rawler::get_decoder(&mut rawfile)
            .with_context(|| format!("creating decoder for {}", path.display()))?;

        let raw_params = RawDecodeParams { image_index: 0 };
        let metadata = decoder
            .raw_metadata(&mut rawfile, raw_params)
            .with_context(|| format!("decoding metadata for {}", path.display()))?;
        let exif = metadata.exif;

        let aperture = exif.fnumber.unwrap_or_default().as_f32();
        let exposure_time = exif.exposure_time.unwrap_or_default().as_f32();
        let iso_speed = exif.iso_speed_ratings.unwrap_or_default() as u32;

        let creation_time = NaiveDateTime::parse_from_str(
            exif.date_time_original.clone().unwrap_or_default().as_str(),
            "%Y:%m:%d %H:%M:%S",
        )
        .ok();

        Ok(FrameMetadata {
            exposure_seconds: exposure_time,
            interval_seconds: None,
            aperture,
            iso: iso_speed,
            creation_time,
        })
    }
}
