use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde_json::json;
use libdng::exif::ExifExtractable;
use libdng::bindings::*;
use num::rational::Ratio;
use rexif::TagValue;
use num::ToPrimitive;
use anyhow;


pub struct ImageCandidate {
    pub path: PathBuf,
    exif: Arc<Mutex<ExifContainer>>,
}

impl ImageCandidate {
    pub fn load(path: &Path) -> anyhow::Result<ImageCandidate> {
        Ok(ImageCandidate {
            path: path.to_path_buf(),
            exif: Arc::new(Mutex::new(ExifContainer::from_file(path)?))
        })
    }

    pub fn json(self) -> serde_json::Value {
        let exif = self.exif.lock().unwrap();
        let exposure = match exif.get_urational(ExifTag_Photo_ExposureTime, 0) {
            None => None,
            Some(x) => x.to_f64()
        };
        let aperture = match exif.get_urational(ExifTag_Photo_FNumber, 0) {
            None => None,
            Some(x) => x.to_f64()
        };

        json!({
            "path": self.path.to_str(),
            "filename": self.path.file_name().unwrap().to_str().unwrap(),
            "creation_time": exif.get_string(ExifTag_Photo_DateTimeOriginal),
            "exposure_seconds": exposure,
            "aperture": aperture,
            "iso": exif.get_uint(ExifTag_Photo_ISOSpeedRatings, 0),
        })
    }
}

#[derive(Clone)]
pub struct ExifContainer {
    pub mapped_entries: HashMap<rexif::ExifTag, rexif::ExifEntry>,
    pub all_entries: HashMap<u16, rexif::ExifEntry>
}

impl ExifContainer {
    pub fn from_file(path: &Path) -> anyhow::Result<ExifContainer> {
        let data = rexif::parse_file(path)?;
        Ok(ExifContainer::from_rexif_data(data))
    }

    pub fn from_rexif_data(data: rexif::ExifData) -> ExifContainer {
        let all = data.entries.iter()
            .map(|x| (x.ifd.tag, x.clone())).collect();


        ExifContainer {
            mapped_entries: ExifContainer::get_known_map(&all),
            all_entries: all
        }
    }

    pub fn get_known_map(map: &HashMap<u16, rexif::ExifEntry>) -> HashMap<rexif::ExifTag, rexif::ExifEntry> {
        map .values().into_iter()
            .filter(|x| x.tag != rexif::ExifTag::UnknownToMe)
            .map(|x| (x.tag, x.clone())).collect()
    }

    #[allow(dead_code)]
    pub fn print_all(&self) {
        println!("\n\nMAPPED EXIF ENTRIES");
        self.print_mapped();

        println!("\n\nALL EXIF ENTRIES");
        self.print_unknown();
    }

    pub fn print_mapped(&self) {
        for entry in self.mapped_entries.values() {
            if entry.tag == rexif::ExifTag::MakerNote { continue; }
            println!("\t{}\t{}: {} ({})", entry.ifd.tag, entry.tag, entry.value, entry.value_more_readable);
        }
    }

    pub fn print_unknown(&self) {
        for (ifd_tag, entry) in &self.all_entries {
            println!("\t{}\t{}: {}", ifd_tag, entry.tag, entry.value);
        }
    }
}

impl ExifExtractable for ExifContainer {
    fn get_uint(&self, tag: u32, index: u16) -> Option<u32> {
        let r_tag = map_external_tag_to_rexif(tag)?;
        let entry = self.mapped_entries.get(&r_tag)?;

        match &entry.value {
            TagValue::U8(x) => Some(*x.get(index as usize)? as u32),
            TagValue::U16(x) => Some(*x.get(index as usize)? as u32),
            TagValue::U32(x)  => Some(*x.get(index as usize)?),
            _ => None
        }
    }

    fn get_urational(&self, tag: u32, index: u16) -> Option<Ratio<u32>> {
        let r_tag = map_external_tag_to_rexif(tag)?;
        let entry = self.mapped_entries.get(&r_tag)?;

        match &entry.value {
            TagValue::URational(x) => {
                let r = x.get(index as usize)?;
                Some(Ratio::new(r.numerator, r.denominator))
            },
            _ => None
        }
    }

    fn get_srational(&self, tag: u32, index: u16) -> Option<Ratio<i32>> {
        let r_tag = map_external_tag_to_rexif(tag)?;
        let entry = self.mapped_entries.get(&r_tag)?;

        match &entry.value {
            TagValue::IRational(x) => {
                let r = x.get(index as usize)?;
                Some(Ratio::new(r.numerator, r.denominator))
            },
            TagValue::URational(x) => {
                let r = x.get(index as usize)?;
                Some(Ratio::new(r.numerator as i32, r.denominator as i32))
            }
            _ => None
        }
    }

    fn get_string(&self, tag: u32) -> Option<String> {
        let r_tag = map_external_tag_to_rexif(tag)?;
        match self.mapped_entries.get(&r_tag) {
            Some(x) => Some(x.value.to_string()),
            None => None
        }
    }
}

fn map_external_tag_to_rexif(tag: ExifTag) -> Option<rexif::ExifTag> {
    match tag {
        ExifTag_Image_ImageDescription => Some(rexif::ExifTag::ImageDescription),
        ExifTag_Image_Make => Some(rexif::ExifTag::Make),
        ExifTag_Image_Model => Some(rexif::ExifTag::Model),
        ExifTag_Image_Software => Some(rexif::ExifTag::Software),
        ExifTag_Image_DateTime => Some(rexif::ExifTag::DateTime),
        ExifTag_Image_Copyright => Some(rexif::ExifTag::Copyright),
        ExifTag_Photo_ExposureTime => Some(rexif::ExifTag::ExposureTime),
        ExifTag_Photo_FNumber => Some(rexif::ExifTag::FNumber),
        ExifTag_Photo_ExposureProgram => Some(rexif::ExifTag::ExposureProgram),
        ExifTag_Photo_ISOSpeedRatings => Some(rexif::ExifTag::ISOSpeedRatings),
        ExifTag_Photo_SensitivityType => Some(rexif::ExifTag::SensitivityType),
        ExifTag_Photo_DateTimeOriginal => Some(rexif::ExifTag::DateTimeOriginal),
        ExifTag_Photo_DateTimeDigitized => Some(rexif::ExifTag::DateTimeDigitized),
        ExifTag_Photo_ShutterSpeedValue => Some(rexif::ExifTag::ShutterSpeedValue),
        ExifTag_Photo_ApertureValue => Some(rexif::ExifTag::ApertureValue),
        ExifTag_Photo_BrightnessValue => Some(rexif::ExifTag::BrightnessValue),
        ExifTag_Photo_ExposureBiasValue => Some(rexif::ExifTag::ExposureBiasValue),
        ExifTag_Photo_MaxApertureValue => Some(rexif::ExifTag::MaxApertureValue),
        ExifTag_Photo_SubjectDistance => Some(rexif::ExifTag::SubjectDistance),
        ExifTag_Photo_MeteringMode => Some(rexif::ExifTag::MeteringMode),
        ExifTag_Photo_LightSource => Some(rexif::ExifTag::LightSource),
        ExifTag_Photo_Flash => Some(rexif::ExifTag::Flash),
        ExifTag_Photo_FocalLength => Some(rexif::ExifTag::FocalLength),
        ExifTag_Photo_SubjectArea => Some(rexif::ExifTag::SubjectArea),
        ExifTag_Photo_MakerNote => Some(rexif::ExifTag::MakerNote),
        ExifTag_Photo_UserComment => Some(rexif::ExifTag::UserComment),
        ExifTag_Photo_ColorSpace => Some(rexif::ExifTag::ColorSpace),
        ExifTag_Photo_FocalPlaneXResolution => Some(rexif::ExifTag::FocalPlaneXResolution),
        ExifTag_Photo_FocalPlaneYResolution => Some(rexif::ExifTag::FocalPlaneYResolution),
        ExifTag_Photo_FocalPlaneResolutionUnit => Some(rexif::ExifTag::FocalPlaneResolutionUnit),
        ExifTag_Photo_ExposureIndex => Some(rexif::ExifTag::ExposureIndex),
        ExifTag_Photo_SensingMethod => Some(rexif::ExifTag::SensingMethod),
        ExifTag_Photo_FileSource => Some(rexif::ExifTag::FileSource),
        ExifTag_Photo_SceneType => Some(rexif::ExifTag::SceneType),
        ExifTag_Photo_CustomRendered => Some(rexif::ExifTag::CustomRendered),
        ExifTag_Photo_ExposureMode => Some(rexif::ExifTag::ExposureMode),
        ExifTag_Photo_DigitalZoomRatio => Some(rexif::ExifTag::DigitalZoomRatio),
        ExifTag_Photo_FocalLengthIn35mmFilm => Some(rexif::ExifTag::FocalLengthIn35mmFilm),
        ExifTag_Photo_SceneCaptureType => Some(rexif::ExifTag::SceneCaptureType),
        ExifTag_Photo_GainControl => Some(rexif::ExifTag::GainControl),
        ExifTag_Photo_Contrast => Some(rexif::ExifTag::Contrast),
        ExifTag_Photo_Saturation => Some(rexif::ExifTag::Saturation),
        ExifTag_Photo_Sharpness => Some(rexif::ExifTag::Sharpness),
        ExifTag_Photo_SubjectDistanceRange => Some(rexif::ExifTag::SubjectDistanceRange),
        ExifTag_Photo_LensSpecification => Some(rexif::ExifTag::LensSpecification),
        ExifTag_Photo_LensMake => Some(rexif::ExifTag::LensMake),
        ExifTag_Photo_LensModel => Some(rexif::ExifTag::LensModel),
        ExifTag_Photo_Gamma => Some(rexif::ExifTag::Gamma),
        ExifTag_GPSInfo_GPSVersionID => Some(rexif::ExifTag::GPSVersionID),
        ExifTag_GPSInfo_GPSLatitudeRef => Some(rexif::ExifTag::GPSLatitudeRef),
        ExifTag_GPSInfo_GPSLatitude => Some(rexif::ExifTag::GPSLatitude),
        ExifTag_GPSInfo_GPSLongitudeRef => Some(rexif::ExifTag::GPSLongitudeRef),
        ExifTag_GPSInfo_GPSLongitude => Some(rexif::ExifTag::GPSLongitude),
        ExifTag_GPSInfo_GPSAltitudeRef => Some(rexif::ExifTag::GPSAltitudeRef),
        ExifTag_GPSInfo_GPSAltitude => Some(rexif::ExifTag::GPSAltitude),
        ExifTag_GPSInfo_GPSTimeStamp => Some(rexif::ExifTag::GPSTimeStamp),
        ExifTag_GPSInfo_GPSSatellites => Some(rexif::ExifTag::GPSSatellites),
        ExifTag_GPSInfo_GPSStatus => Some(rexif::ExifTag::GPSStatus),
        ExifTag_GPSInfo_GPSMeasureMode => Some(rexif::ExifTag::GPSMeasureMode),
        ExifTag_GPSInfo_GPSDOP => Some(rexif::ExifTag::GPSDOP),
        ExifTag_GPSInfo_GPSSpeedRef => Some(rexif::ExifTag::GPSSpeedRef),
        ExifTag_GPSInfo_GPSSpeed => Some(rexif::ExifTag::GPSSpeed),
        ExifTag_GPSInfo_GPSTrackRef => Some(rexif::ExifTag::GPSTrackRef),
        ExifTag_GPSInfo_GPSTrack => Some(rexif::ExifTag::GPSTrack),
        ExifTag_GPSInfo_GPSImgDirectionRef => Some(rexif::ExifTag::GPSImgDirectionRef),
        ExifTag_GPSInfo_GPSImgDirection => Some(rexif::ExifTag::GPSImgDirection),
        ExifTag_GPSInfo_GPSMapDatum => Some(rexif::ExifTag::GPSMapDatum),
        ExifTag_GPSInfo_GPSDestLatitudeRef => Some(rexif::ExifTag::GPSDestLatitudeRef),
        ExifTag_GPSInfo_GPSDestLatitude => Some(rexif::ExifTag::GPSDestLatitude),
        ExifTag_GPSInfo_GPSDestLongitudeRef => Some(rexif::ExifTag::GPSDestLongitudeRef),
        ExifTag_GPSInfo_GPSDestLongitude => Some(rexif::ExifTag::GPSDestLongitude),
        ExifTag_GPSInfo_GPSDestBearingRef => Some(rexif::ExifTag::GPSDestBearingRef),
        ExifTag_GPSInfo_GPSDestBearing => Some(rexif::ExifTag::GPSDestBearing),
        ExifTag_GPSInfo_GPSDestDistanceRef => Some(rexif::ExifTag::GPSDestDistanceRef),
        ExifTag_GPSInfo_GPSDestDistance => Some(rexif::ExifTag::GPSDestDistance),
        ExifTag_GPSInfo_GPSProcessingMethod => Some(rexif::ExifTag::GPSProcessingMethod),
        ExifTag_GPSInfo_GPSAreaInformation => Some(rexif::ExifTag::GPSAreaInformation),
        ExifTag_GPSInfo_GPSDateStamp => Some(rexif::ExifTag::GPSDateStamp),
        ExifTag_GPSInfo_GPSDifferential => Some(rexif::ExifTag::GPSDifferential),
        _ => None
    }
}