use log::trace;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct ImageCandidate {
    pub path: PathBuf,
    exif: Arc<Mutex<ExifContainer>>,
}

impl ImageCandidate {
    pub fn load(path: &Path) -> anyhow::Result<ImageCandidate> {
        Ok(ImageCandidate {
            path: path.to_path_buf(),
            exif: Arc::new(Mutex::new(ExifContainer::from_file(path)?)),
        })
    }

    pub fn json(self) -> serde_json::Value {
        let exif = self.exif.lock().unwrap();

        json!({
            "path": self.path.to_str(),
            "filename": self.path.file_name().unwrap().to_str().unwrap(),
            "creation_time": "unknown",
            "exposure_seconds": 0,
            "aperture": 0,
            "iso": 0,
        })
    }
}

#[derive(Clone)]
pub struct ExifContainer {
    pub mapped_entries: HashMap<rexif::ExifTag, rexif::ExifEntry>,
    pub all_entries: HashMap<u16, rexif::ExifEntry>,
}

impl ExifContainer {
    pub fn from_file(path: &Path) -> anyhow::Result<ExifContainer> {
        let data = rexif::parse_file(path)?;
        Ok(ExifContainer::from_rexif_data(data))
    }

    pub fn from_rexif_data(data: rexif::ExifData) -> ExifContainer {
        let all = data.entries.iter().map(|x| (x.ifd.tag, x.clone())).collect();

        ExifContainer {
            mapped_entries: ExifContainer::get_known_map(&all),
            all_entries: all,
        }
    }

    pub fn get_known_map(map: &HashMap<u16, rexif::ExifEntry>) -> HashMap<rexif::ExifTag, rexif::ExifEntry> {
        map.values()
            .into_iter()
            .filter(|x| x.tag != rexif::ExifTag::UnknownToMe)
            .map(|x| (x.tag, x.clone()))
            .collect()
    }

    #[allow(dead_code)]
    pub fn print_all(&self) {
        trace!("\n\nMAPPED EXIF ENTRIES");
        self.print_mapped();

        trace!("\n\nALL EXIF ENTRIES");
        self.print_unknown();
    }

    pub fn print_mapped(&self) {
        for entry in self.mapped_entries.values() {
            if entry.tag == rexif::ExifTag::MakerNote {
                continue;
            }
            trace!("\t{}\t{}: {} ({})", entry.ifd.tag, entry.tag, entry.value, entry.value_more_readable);
        }
    }

    pub fn print_unknown(&self) {
        for (ifd_tag, entry) in &self.all_entries {
            trace!("\t{}\t{}: {}", ifd_tag, entry.tag, entry.value);
        }
    }
}
