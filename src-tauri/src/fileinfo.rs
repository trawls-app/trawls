use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde_json::json;
use rexif;


pub struct ImageCandidate {
    pub path: PathBuf,
    exif: Arc<Mutex<ExifContainer>>,
}

impl ImageCandidate {
    pub fn load(path: &Path) -> Result<ImageCandidate, ()> {
        Ok(ImageCandidate {
            path: path.to_path_buf(),
            exif: Arc::new(Mutex::new(ExifContainer::from_file(path)?))
        })
    }

    pub fn json(self) -> serde_json::Value {
        let exif = self.exif.lock().unwrap();

        json!({
            "path": self.path.to_str(),
            "filename": self.path.file_name().unwrap().to_str().unwrap(),
            "creation_time": exif.get_string(rexif::ExifTag::DateTimeOriginal),
            "height": exif.get_int(rexif::ExifTag::XResolution),
            "width": exif.get_int(rexif::ExifTag::YResolution),
            "iso": exif.get_int(rexif::ExifTag::ISOSpeedRatings),
        })
    }
}


pub struct ExifContainer {
    entries: HashMap<rexif::ExifTag, rexif::ExifEntry>
}

impl ExifContainer {
    pub fn from_file(path: &Path) -> Result<ExifContainer, ()> {
        let data = rexif::parse_file(path).unwrap();
        Ok(ExifContainer::from_rexif_data(data))
    }

    pub fn from_rexif_data(data: rexif::ExifData) -> ExifContainer {
        let exif_map = data.entries.iter().map(|x| (x.tag, x.clone())).collect();

        ExifContainer { entries: exif_map }
    }

    #[allow(dead_code)]
    pub fn print_all(&self) {
        for entry in self.entries.values() {
            println!("\t{}: {}", entry.tag, entry.value);
        }
    }

    pub fn get_string(&self, tag: rexif::ExifTag) -> Option<String> {
        match self.entries.get(&tag) {
            Some(x) => Some(x.value.to_string()),
            None => None
        }
    }

    pub fn get_int(&self, tag: rexif::ExifTag) -> Option<i64> {
        match self.entries.get(&tag) {
            Some(x) => x.value.to_i64(0),
            None => None
        }
    }

    #[allow(dead_code)]
    pub fn get_float(&self, tag: rexif::ExifTag) -> Option<f64> {
        match self.entries.get(&tag) {
            Some(x) => x.value.to_f64(0),
            None => None
        }
    }
}
