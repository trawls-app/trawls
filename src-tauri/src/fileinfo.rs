use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::BufReader;
use serde_json::json;
use exif::Exif;


pub struct ImageCandidate {
    pub path: PathBuf,
    exif: Arc<Mutex<exif::Exif>>,
}

impl ImageCandidate {
    pub fn load(path: &Path) -> Result<ImageCandidate, &'static str> {
        let exif = load_exif(&path).unwrap();
        
        Ok(ImageCandidate {
            path: path.to_path_buf(),
            exif: Arc::new(Mutex::new(exif))
        })
    }

    pub fn json(self) -> serde_json::Value {
        json!({
            "path": self.path.to_str(),
            "filename": self.path.file_name().unwrap().to_str().unwrap(),
            "creation_time": get_exif_string(self.exif.clone(), exif::Tag::DateTime, true),
            "height": get_exif_int(self.exif.clone(), exif::Tag::PixelYDimension),
            "width": get_exif_int(self.exif.clone(), exif::Tag::PixelXDimension),
            "iso": get_exif_int(self.exif.clone(), exif::Tag::ISOSpeedLatitudeyyy),
        })
    }
}

fn load_exif(path: &Path) -> Result<exif::Exif, exif::Error> {
    let file = File::open(path)?;
    let exif = exif::Reader::new().read_from_container(
        &mut BufReader::new(&file))?;

    Ok(exif)
}

fn get_exif_string(exif: Arc<Mutex<Exif>>, tag: exif::Tag, return_empty: bool) -> Option<String> {
    if let Some(field) = exif.lock().unwrap().get_field(tag, exif::In::PRIMARY) {
        return Some(field.display_value().to_string())
    }

    if return_empty {
        return Some("".to_string())
    }

    None
}

fn get_exif_int(exif: Arc<Mutex<Exif>>, tag: exif::Tag) -> Option<u32> {
    if let Some(field) = exif.lock().unwrap().get_field(tag, exif::In::PRIMARY) {
        return field.value.get_uint(0)
    }

    None
}