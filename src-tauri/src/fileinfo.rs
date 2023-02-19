use chrono::NaiveDateTime;
use rawler::decoders::RawDecodeParams;
use rawler::exif::Exif;
use rawler::RawFile;
use serde_json::json;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct ImageCandidate {
    pub path: PathBuf,
    exif: Arc<Mutex<Exif>>,
}

impl ImageCandidate {
    pub fn load(path: &Path) -> anyhow::Result<ImageCandidate> {
        // Get a decoder
        let file_buffer = BufReader::new(File::open(path)?);
        let mut rawfile = RawFile::new(path, file_buffer);
        let decoder = rawler::get_decoder(&mut rawfile)?;

        // Decode metadata
        let raw_params = RawDecodeParams { image_index: 0 };
        let metadata = decoder.raw_metadata(&mut rawfile, raw_params)?;

        Ok(ImageCandidate {
            path: path.to_path_buf(),
            exif: Arc::new(Mutex::new(metadata.exif)),
        })
    }

    pub fn json(self) -> serde_json::Value {
        let exif = self.exif.lock().unwrap();

        let aperture = exif.fnumber.unwrap_or_default().as_f32();
        let exposure_time = exif.exposure_time.unwrap_or_default().as_f32();
        let iso_speed = exif.iso_speed_ratings.unwrap_or_default();

        // Bring the date into something more ISO conform
        let creation_time = NaiveDateTime::parse_from_str(
            exif.date_time_original.clone().unwrap_or_default().as_str(),
            "%Y:%m:%d %H:%M:%S",
        )
        .ok()
        .unwrap_or_default();

        json!({
            "path": self.path.to_str(),
            "filename": self.path.file_name().unwrap().to_str().unwrap(),
            "creation_time": creation_time,
            "exposure_seconds": exposure_time,
            "aperture": format!("{:.1}", aperture),
            "iso": iso_speed,
        })
    }
}
