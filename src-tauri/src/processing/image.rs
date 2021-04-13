use std::path::Path;
use std::unimplemented;
use std::cmp::max;
use std::collections::HashMap;
use num::rational::Ratio;
use rawloader::{RawImage, RawImageData};
use rexif::{ExifTag, TagValue};
use crate::fileinfo::ExifContainer;


pub trait Mergable<Rhs=Self> {
    type Container;

    fn load_from_raw(path: &Path, intensity: f32) -> Result<Self::Container, &str>;
    fn merge(self, other: Rhs) -> Rhs;
}

pub struct Image {
    raw_image: RawImage,
    pub exif: ExifContainer
}

impl Mergable for Image {
    type Container = Image;

    fn load_from_raw(path: &Path, intensity: f32) -> Result<Self::Container, &str> {
        Ok( Image {
            raw_image: RawImage::load_from_raw(path, intensity)?,
            exif: ExifContainer::load_from_raw(path, intensity)?
        })
    }

    fn merge(self, other: Self) -> Self {
        Image {
            raw_image: self.raw_image.merge(other.raw_image),
            exif: self.exif.merge(other.exif)
        }
    }
}


impl Mergable for RawImage {
    type Container = RawImage;

    fn load_from_raw(path: &Path, intensity: f32) -> Result<Self::Container, &str> {
        let mut raw_image = rawloader::decode_file(path).unwrap();

        if (intensity - 1.0).abs() > 0.001 {
            raw_image.data = match raw_image.data {
                RawImageData::Integer(d) => RawImageData::Integer(d.iter().map(|x| (*x as f32 * intensity) as u16).collect()),
                RawImageData::Float(d) => RawImageData::Float(d.iter().map(|x| (*x as f32 * intensity)).collect())
            };
        }

        Ok( raw_image )
    }

    fn merge(self, other: RawImage) -> RawImage {
        if self.width != other.width || self.height != other.height || self.cpp != other.cpp {
            panic!("Images to merge have different dimensions");
        }

        let data_self = match self.data {
            RawImageData::Integer(d) => { d }
            RawImageData::Float(_) => unimplemented!("Floating point RAWs are not supported.")
        };

        let data_other = match other.data {
            RawImageData::Integer(d) => { d }
            RawImageData::Float(_) => unimplemented!("Floating point RAWs are not supported.")
        };


        let res = data_self.iter()
            .zip(data_other)
            .map(|(x, y)| max(*x, y))
            .collect();

        RawImage {
            make: self.make, model: self.model,
            clean_make: self.clean_make, clean_model: self.clean_model,
            width: self.width, height: self.height,
            cpp: self.cpp, wb_coeffs: self.wb_coeffs,
            whitelevels: self.whitelevels, blacklevels: self.blacklevels,
            xyz_to_cam: self.xyz_to_cam, cfa: self.cfa,
            crops: self.crops, blackareas: self.blackareas,
            orientation: self.orientation,
            data: RawImageData::Integer(res)
        }
    }
}


impl Mergable for ExifContainer {
    type Container = ExifContainer;

    fn load_from_raw(path: &Path, _intensity: f32) -> Result<Self::Container, &str> {
        Ok(ExifContainer::from_file(path).unwrap())
    }

    fn merge(self, other: Self) -> Self {
        let mut res = HashMap::new();
        for (key, self_entry) in self.mapped_entries.iter() {
            if !other.mapped_entries.contains_key(key) { continue; }
            let other_entry = other.mapped_entries.get(key).unwrap();

            let merged_value = match key {
                // Add exposure time of merged images
                ExifTag::ExposureTime => {
                    let mut new_entry = self_entry.clone();
                    new_entry.value = add_urationals(&self_entry.value, &other_entry.value);
                    Some(new_entry)
                },
                // Take earliest capture date
                ExifTag::DateTime | ExifTag::DateTimeDigitized | ExifTag::DateTimeOriginal => {
                    if self_entry.value.to_string() <= other_entry.value.to_string() {
                        Some(self_entry.clone())
                    } else {
                        Some(other_entry.clone())
                    }
                },
                // Copy all remaining entries which are identical in both images
                _ => {
                    if self_entry == other.mapped_entries.get(key).unwrap() { Some(self_entry.clone()) }
                    else { None }
                }
            };

            if let Some(v) = merged_value {
                res.insert(*key, v);
            }
        }

        ExifContainer {
            mapped_entries: res,
            all_entries: self.all_entries
        }
    }
}

fn add_urationals(op1: &rexif::TagValue, op2: &rexif::TagValue) -> rexif::TagValue {
    let v1 = match op1 {
        TagValue::URational(x) => { Ratio::new(x[0].numerator, x[0].denominator) }
        _ => { panic!("Exposure time has unexpected type."); }
    };

    let v2 = match op2 {
        TagValue::URational(x) => { Ratio::new(x[0].numerator, x[0].denominator) }
        _ => { panic!("Exposure time has unexpected type."); }
    };

    let res = (v1 + v2).reduced();
    rexif::TagValue::URational(vec![rexif::URational {
        numerator: *res.numer(),
        denominator: *res.denom()
    }])
}