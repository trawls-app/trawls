use crate::fileinfo::ExifContainer;
use arrayvec::ArrayVec;
use libdng::bindings::{Area, ImageInfoContainer};
use libdng::exif::ExifBox;
use libdng::image_info::RawSavableImage;
use num::rational::Ratio;
use rawloader::{RawImage, RawImageData};
use rexif::{ExifTag, TagValue};
use std::cmp::max;
use std::collections::HashMap;
use std::path::Path;
use std::unimplemented;

#[derive(Copy, Clone)]
pub enum MergeMode {
    Maximize,
    WeightedAverage,
}

pub trait Mergable<Rhs = Self> {
    type Container;

    fn load_from_raw(path: &Path, intensity: f32) -> Result<Self::Container, &str>;
    fn weighted_merge(
        self,
        other: Rhs,
        weight_self: f32,
        weight_other: f32,
        mode: MergeMode,
    ) -> Rhs;
    fn merge(self, other: Rhs, mode: MergeMode) -> Rhs;
}

pub struct Image {
    raw_image: RawImage,
    pub exif: ExifContainer,
    num_images: usize,
}

impl Image {
    pub fn apply_darkframe(self, darkframe: Image) -> Image {
        let light = self.get_image_data();
        let dark = darkframe.get_image_data();
        let avg_black =
            (dark.iter().fold(0u64, |acc, x| acc + *x as u64) / dark.len() as u64) as i16;

        let res = light
            .iter()
            .zip(dark)
            .map(|(x, y)| max(0, *x as i16 - (y as i16 - avg_black)) as u16)
            .collect();

        Image {
            raw_image: RawImage {
                make: self.raw_image.make,
                model: self.raw_image.model,
                clean_make: self.raw_image.clean_make,
                clean_model: self.raw_image.clean_model,
                width: self.raw_image.width,
                height: self.raw_image.height,
                cpp: self.raw_image.cpp,
                wb_coeffs: self.raw_image.wb_coeffs,
                whitelevels: self.raw_image.whitelevels,
                blacklevels: self.raw_image.blacklevels,
                xyz_to_cam: self.raw_image.xyz_to_cam,
                cfa: self.raw_image.cfa,
                crops: self.raw_image.crops,
                blackareas: self.raw_image.blackareas,
                orientation: self.raw_image.orientation,
                data: RawImageData::Integer(res),
            },
            exif: self.exif,
            num_images: self.num_images,
        }
    }
}

impl Mergable for Image {
    type Container = Image;

    fn load_from_raw(path: &Path, intensity: f32) -> Result<Self::Container, &str> {
        Ok(Image {
            raw_image: RawImage::load_from_raw(path, intensity)?,
            exif: ExifContainer::load_from_raw(path, intensity)?,
            num_images: 1,
        })
    }

    fn weighted_merge(
        self,
        other: Self,
        weight_self: f32,
        weight_other: f32,
        mode: MergeMode,
    ) -> Self {
        Image {
            raw_image: self.raw_image.weighted_merge(
                other.raw_image,
                weight_self,
                weight_other,
                mode,
            ),
            exif: self
                .exif
                .weighted_merge(other.exif, weight_self, weight_other, mode),
            num_images: self.num_images + other.num_images,
        }
    }

    fn merge(self, other: Self, mode: MergeMode) -> Self {
        let weight_self = self.num_images as f32;
        let weight_other = other.num_images as f32;
        match mode {
            MergeMode::Maximize => self.weighted_merge(other, 1.0, 1.0, mode),
            MergeMode::WeightedAverage => {
                self.weighted_merge(other, weight_self, weight_other, mode)
            }
        }
    }
}

impl RawSavableImage for Image {
    fn get_make_model(&self) -> (String, String) {
        (
            self.raw_image.clean_make.clone(),
            self.raw_image.clean_model.clone(),
        )
    }

    fn get_exif_box(&self) -> ExifBox {
        ExifBox {
            extractable: Box::new(self.exif.clone()),
        }
    }

    fn get_info_container(&self) -> ImageInfoContainer {
        let black_levels: ArrayVec<_, 4> = self
            .raw_image
            .blacklevels
            .iter()
            .map(|x| *x as f64)
            .collect();
        let white_levels: ArrayVec<_, 4> = self
            .raw_image
            .whitelevels
            .iter()
            .map(|x| *x as f64)
            .collect();
        let neutrals: ArrayVec<_, 3> = self.raw_image.wb_coeffs[0..3]
            .iter()
            .map(|x| 1.0 / (*x as f64))
            .collect();
        let colormatrix: ArrayVec<_, 3> = self.raw_image.xyz_to_cam[0..3].iter().copied().collect();

        ImageInfoContainer {
            width: self.raw_image.width as u16,
            height: self.raw_image.height as u16,
            black_levels: black_levels.into_inner().unwrap(),
            white_levels: white_levels.into_inner().unwrap(),
            camera_neutral: neutrals.into_inner().unwrap(),
            xyz_to_cam: colormatrix.into_inner().unwrap(),
            active_area: Area {
                top: self.raw_image.crops[0] as u16,
                left: self.raw_image.crops[3] as u16,
                bottom: (self.raw_image.height - self.raw_image.crops[2]) as u16,
                right: (self.raw_image.width - self.raw_image.crops[1]) as u16,
            },
        }
    }

    fn get_image_data(&self) -> Vec<u16> {
        if let rawloader::RawImageData::Integer(data) = &self.raw_image.data {
            data.clone()
        } else {
            unimplemented!("Can't parse RAWs with non-integer data, yet.");
        }
    }
}

impl Mergable for RawImage {
    type Container = RawImage;

    fn load_from_raw(path: &Path, intensity: f32) -> Result<Self::Container, &str> {
        let mut raw_image = rawloader::decode_file(path).unwrap();

        if (intensity - 1.0).abs() > 0.001 {
            raw_image.data = match raw_image.data {
                RawImageData::Integer(d) => RawImageData::Integer(
                    d.iter().map(|x| (*x as f32 * intensity) as u16).collect(),
                ),
                RawImageData::Float(d) => {
                    RawImageData::Float(d.iter().map(|x| (*x as f32 * intensity)).collect())
                }
            };
        }

        Ok(raw_image)
    }

    fn weighted_merge(
        self,
        other: Self,
        weight_self: f32,
        weight_other: f32,
        mode: MergeMode,
    ) -> Self {
        if self.width != other.width || self.height != other.height || self.cpp != other.cpp {
            panic!("Images to merge have different dimensions");
        }

        let data_self = match self.data {
            RawImageData::Integer(d) => d,
            RawImageData::Float(_) => unimplemented!("Floating point RAWs are not supported."),
        };

        let data_other = match other.data {
            RawImageData::Integer(d) => d,
            RawImageData::Float(_) => unimplemented!("Floating point RAWs are not supported."),
        };

        let res = data_self
            .iter()
            .zip(data_other)
            .map(|(x, y)| match mode {
                MergeMode::Maximize => max(*x, y),
                MergeMode::WeightedAverage => {
                    ((*x as f32 * weight_self + y as f32 * weight_other)
                        / (weight_self + weight_other)) as u16
                }
            })
            .collect();

        RawImage {
            make: self.make,
            model: self.model,
            clean_make: self.clean_make,
            clean_model: self.clean_model,
            width: self.width,
            height: self.height,
            cpp: self.cpp,
            wb_coeffs: self.wb_coeffs,
            whitelevels: self.whitelevels,
            blacklevels: self.blacklevels,
            xyz_to_cam: self.xyz_to_cam,
            cfa: self.cfa,
            crops: self.crops,
            blackareas: self.blackareas,
            orientation: self.orientation,
            data: RawImageData::Integer(res),
        }
    }

    fn merge(self, other: Self, mode: MergeMode) -> Self {
        self.weighted_merge(other, 1., 1., mode)
    }
}

impl Mergable for ExifContainer {
    type Container = ExifContainer;

    fn load_from_raw(path: &Path, _intensity: f32) -> Result<Self::Container, &str> {
        Ok(ExifContainer::from_file(path).unwrap())
    }

    fn weighted_merge(
        self,
        other: Self,
        _weight_self: f32,
        _weight_other: f32,
        _mode: MergeMode,
    ) -> Self {
        let mut res = HashMap::new();
        for (key, self_entry) in self.all_entries.iter() {
            if !other.all_entries.contains_key(key) {
                continue;
            }
            let other_entry = other.all_entries.get(key).unwrap();
            let mapped_key = self_entry.tag;

            let merged_value = match mapped_key {
                // Add exposure time of merged images
                ExifTag::ExposureTime => {
                    let mut new_entry = self_entry.clone();
                    new_entry.value = add_urationals(&self_entry.value, &other_entry.value);
                    Some(new_entry)
                }
                // Take earliest capture date
                ExifTag::DateTime | ExifTag::DateTimeDigitized | ExifTag::DateTimeOriginal => {
                    if self_entry.value.to_string() <= other_entry.value.to_string() {
                        Some(self_entry.clone())
                    } else {
                        Some(other_entry.clone())
                    }
                }
                // Copy all remaining entries which are identical in both images
                _ => {
                    if self_entry == other.all_entries.get(key).unwrap() {
                        Some(self_entry.clone())
                    } else {
                        None
                    }
                }
            };

            if let Some(v) = merged_value {
                res.insert(*key, v);
            }
        }

        ExifContainer {
            mapped_entries: ExifContainer::get_known_map(&res),
            all_entries: res,
        }
    }

    fn merge(self, other: Self, mode: MergeMode) -> Self {
        self.weighted_merge(other, 0., 0., mode)
    }
}

fn add_urationals(op1: &rexif::TagValue, op2: &rexif::TagValue) -> rexif::TagValue {
    let v1 = match op1 {
        TagValue::URational(x) => Ratio::new(x[0].numerator, x[0].denominator),
        _ => {
            panic!("Exposure time has unexpected type.");
        }
    };

    let v2 = match op2 {
        TagValue::URational(x) => Ratio::new(x[0].numerator, x[0].denominator),
        _ => {
            panic!("Exposure time has unexpected type.");
        }
    };

    let res = (v1 + v2).reduced();
    rexif::TagValue::URational(vec![rexif::URational {
        numerator: *res.numer(),
        denominator: *res.denom(),
    }])
}
