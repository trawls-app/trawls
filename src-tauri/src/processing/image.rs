use anyhow;
use num::rational::Ratio;
use num::ToPrimitive;
use rawler::decoders::RawDecodeParams;
use rawler::formats::tiff;
use rawler::{exif::Exif, RawFile, RawImage, RawImageData};
use std::cmp::max;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Copy, Clone)]
pub enum MergeMode {
    Maximize,
    WeightedAverage,
}

pub trait Mergable<Rhs = Self> {
    fn weighted_merge(self, other: Rhs, weight_self: f32, weight_other: f32, mode: MergeMode) -> anyhow::Result<Rhs>;
    fn merge(self, other: Rhs, mode: MergeMode) -> anyhow::Result<Rhs>;
}

pub struct Image {
    raw_image: RawImage,
    pub exif: Exif,
    num_images: usize,
}

impl Image {
    pub fn apply_darkframe(self, darkframe: Image) -> anyhow::Result<Image> {
        let light = self.get_image_data()?;
        let dark = darkframe.get_image_data()?;
        let avg_black = (dark.iter().fold(0u64, |acc, x| acc + *x as u64) / dark.len() as u64) as i16;

        let res = light
            .iter()
            .zip(dark)
            .map(|(x, y)| max(0, *x as i16 - (y as i16 - avg_black)) as u16)
            .collect();

        Ok(Image {
            raw_image: RawImage {
                camera: self.raw_image.camera,
                make: self.raw_image.make,
                model: self.raw_image.model,
                clean_make: self.raw_image.clean_make,
                clean_model: self.raw_image.clean_model,
                width: self.raw_image.width,
                height: self.raw_image.height,
                cpp: self.raw_image.cpp,
                bps: self.raw_image.bps,
                wb_coeffs: self.raw_image.wb_coeffs,
                whitelevel: self.raw_image.whitelevel,
                blacklevel: self.raw_image.blacklevel,
                xyz_to_cam: self.raw_image.xyz_to_cam,
                cfa: self.raw_image.cfa,
                active_area: self.raw_image.active_area,
                crop_area: self.raw_image.crop_area,
                blackareas: self.raw_image.blackareas,
                orientation: self.raw_image.orientation,
                data: RawImageData::Integer(res),
                color_matrix: self.raw_image.color_matrix,
                dng_tags: self.raw_image.dng_tags,
            },
            exif: self.exif,
            num_images: self.num_images,
        })
    }

    fn get_image_data(&self) -> anyhow::Result<Vec<u16>> {
        if let rawler::RawImageData::Integer(data) = &self.raw_image.data {
            Ok(data.clone())
        } else {
            anyhow::bail!("Can't parse RAWs with non-integer data, yet.");
        }
    }

    pub fn from_raw_file(path: &Path, intensity: f32) -> anyhow::Result<Image> {
        // Get a decoder
        let file_buffer = BufReader::new(File::open(path)?);
        let mut rawfile = RawFile::new(path, file_buffer);
        let decoder = rawler::get_decoder(&mut rawfile)?;

        // Decode the file
        let raw_params = RawDecodeParams { image_index: 0 };
        let metadata = decoder.raw_metadata(&mut rawfile, raw_params.clone())?;
        let mut raw_image = decoder.raw_image(&mut rawfile, raw_params.clone(), false)?;

        // Apply intensity if applicable
        if (intensity - 1.0).abs() > 0.001 {
            raw_image.data = match raw_image.data {
                RawImageData::Integer(d) => {
                    RawImageData::Integer(d.iter().map(|x| (*x as f32 * intensity) as u16).collect())
                }
                RawImageData::Float(d) => RawImageData::Float(d.iter().map(|x| (*x as f32 * intensity)).collect()),
            };
        }

        Ok(Image {
            raw_image,
            exif: metadata.exif,
            num_images: 1,
        })
    }

    pub fn write_dng(self, path: PathBuf) -> anyhow::Result<()> {
        println!("Exif: {:#?}", self.exif);
        todo!();
    }
}

impl Mergable for Image {
    fn weighted_merge(self, other: Self, weight_self: f32, weight_other: f32, mode: MergeMode) -> anyhow::Result<Self> {
        let img = Image {
            raw_image: self
                .raw_image
                .weighted_merge(other.raw_image, weight_self, weight_other, mode)?,
            exif: self.exif.weighted_merge(other.exif, weight_self, weight_other, mode)?,
            num_images: self.num_images + other.num_images,
        };

        Ok(img)
    }

    fn merge(self, other: Self, mode: MergeMode) -> anyhow::Result<Self> {
        let weight_self = self.num_images as f32;
        let weight_other = other.num_images as f32;
        match mode {
            MergeMode::Maximize => self.weighted_merge(other, 1.0, 1.0, mode),
            MergeMode::WeightedAverage => self.weighted_merge(other, weight_self, weight_other, mode),
        }
    }
}

impl Mergable for RawImage {
    fn weighted_merge(self, other: Self, weight_self: f32, weight_other: f32, mode: MergeMode) -> anyhow::Result<Self> {
        anyhow::ensure!(
            self.width == other.width && self.height == other.height && self.cpp == other.cpp,
            "Images to merge have different dimensions"
        );

        let data_self = match self.data {
            RawImageData::Integer(d) => d,
            RawImageData::Float(_) => anyhow::bail!("Floating point RAWs are not supported."),
        };

        let data_other = match other.data {
            RawImageData::Integer(d) => d,
            RawImageData::Float(_) => anyhow::bail!("Floating point RAWs are not supported."),
        };

        let res = data_self
            .iter()
            .zip(data_other)
            .map(|(x, y)| match mode {
                MergeMode::Maximize => max(*x, y),
                MergeMode::WeightedAverage => {
                    ((*x as f32 * weight_self + y as f32 * weight_other) / (weight_self + weight_other)) as u16
                }
            })
            .collect();

        Ok(RawImage {
            camera: self.camera,
            make: self.make,
            model: self.model,
            clean_make: self.clean_make,
            clean_model: self.clean_model,
            width: self.width,
            height: self.height,
            cpp: self.cpp,
            bps: self.bps,
            wb_coeffs: self.wb_coeffs,
            whitelevel: self.whitelevel,
            blacklevel: self.blacklevel,
            xyz_to_cam: self.xyz_to_cam,
            cfa: self.cfa,
            active_area: self.active_area,
            crop_area: self.crop_area,
            blackareas: self.blackareas,
            orientation: self.orientation,
            data: RawImageData::Integer(res),
            color_matrix: self.color_matrix,
            dng_tags: self.dng_tags,
        })
    }

    fn merge(self, other: Self, mode: MergeMode) -> anyhow::Result<Self> {
        self.weighted_merge(other, 1., 1., mode)
    }
}

impl Mergable for Exif {
    fn weighted_merge(
        self,
        other: Self,
        _weight_self: f32,
        _weight_other: f32,
        _mode: MergeMode,
    ) -> anyhow::Result<Self> {
        Ok(Exif {
            orientation: equal_exif_entry(self.orientation, other.orientation),
            copyright: equal_exif_entry(self.copyright, other.copyright),
            artist: equal_exif_entry(self.artist, other.artist),
            lens_spec: equal_exif_entry(self.lens_spec, other.lens_spec),
            exposure_time: add_tiff_rational(self.exposure_time, other.exposure_time),
            fnumber: equal_exif_entry(self.fnumber, other.fnumber),
            aperture_value: equal_exif_entry(self.aperture_value, other.aperture_value),
            brightness_value: equal_exif_entry(self.brightness_value, other.brightness_value),
            iso_speed_ratings: equal_exif_entry(self.iso_speed_ratings, other.iso_speed_ratings),
            iso_speed: equal_exif_entry(self.iso_speed, other.iso_speed),
            recommended_exposure_index: equal_exif_entry(
                self.recommended_exposure_index,
                other.recommended_exposure_index,
            ),
            sensitivity_type: equal_exif_entry(self.sensitivity_type, other.sensitivity_type),
            exposure_bias: equal_exif_entry(self.exposure_bias, other.exposure_bias),
            date_time_original: min_exif_entry(self.date_time_original, other.date_time_original),
            create_date: min_exif_entry(self.create_date, other.create_date),
            modify_date: max_exif_entry(self.modify_date, other.modify_date),
            exposure_program: equal_exif_entry(self.exposure_program, other.exposure_program),
            timezone_offset: equal_exif_entry(self.timezone_offset, other.timezone_offset),
            offset_time: equal_exif_entry(self.offset_time, other.offset_time),
            offset_time_original: equal_exif_entry(self.offset_time_original, other.offset_time_original),
            offset_time_digitized: equal_exif_entry(self.offset_time_digitized, other.offset_time_digitized),
            sub_sec_time: None,
            sub_sec_time_original: None,
            sub_sec_time_digitized: None,
            shutter_speed_value: equal_exif_entry(self.shutter_speed_value, other.shutter_speed_value),
            max_aperture_value: equal_exif_entry(self.max_aperture_value, other.max_aperture_value),
            subject_distance: equal_exif_entry(self.subject_distance, other.subject_distance),
            metering_mode: equal_exif_entry(self.metering_mode, other.metering_mode),
            light_source: equal_exif_entry(self.light_source, other.light_source),
            flash: equal_exif_entry(self.flash, other.flash),
            focal_length: equal_exif_entry(self.focal_length, other.focal_length),
            image_number: equal_exif_entry(self.image_number, other.image_number),
            color_space: equal_exif_entry(self.color_space, other.color_space),
            flash_energy: equal_exif_entry(self.flash_energy, other.flash_energy),
            exposure_mode: equal_exif_entry(self.exposure_mode, other.exposure_mode),
            white_balance: equal_exif_entry(self.white_balance, other.white_balance),
            scene_capture_type: equal_exif_entry(self.scene_capture_type, other.scene_capture_type),
            subject_distance_range: equal_exif_entry(self.subject_distance_range, other.subject_distance_range),
            owner_name: equal_exif_entry(self.owner_name, other.owner_name),
            serial_number: equal_exif_entry(self.serial_number, other.serial_number),
            lens_serial_number: equal_exif_entry(self.lens_serial_number, other.lens_serial_number),
            lens_make: equal_exif_entry(self.lens_make, other.lens_make),
            lens_model: equal_exif_entry(self.lens_model, other.lens_model),
            gps: None,
        })
    }

    fn merge(self, other: Self, mode: MergeMode) -> anyhow::Result<Self> {
        self.weighted_merge(other, 0., 0., mode)
    }
}

fn add_tiff_rational(entry1: Option<tiff::Rational>, entry2: Option<tiff::Rational>) -> Option<tiff::Rational> {
    if entry1.is_none() || entry2.is_none() {
        return None;
    }

    let x1 = entry1.unwrap();
    let x2 = entry2.unwrap();

    let res = Ratio::new(x1.n, x1.d) + Ratio::new(x2.n, x2.d);
    let res = res.reduced();

    Some(tiff::Rational::new(res.numer().to_u32().unwrap(), res.denom().to_u32().unwrap()))
}

fn equal_exif_entry<T: std::cmp::PartialEq>(entry1: Option<T>, entry2: Option<T>) -> Option<T> {
    // if-let syntax does not work due to bug: https://github.com/rust-lang/rust/issues/53667

    if entry1.is_some() && entry2.is_some() {
        let x1 = entry1.unwrap();
        let x2 = entry2.unwrap();

        if x1 == x2 {
            return Some(x1);
        }
    }

    None
}

fn min_exif_entry<T: std::cmp::Ord>(entry1: Option<T>, entry2: Option<T>) -> Option<T> {
    if entry1.is_none() || entry2.is_none() {
        return None;
    }

    let x1 = entry1.unwrap();
    let x2 = entry2.unwrap();

    Some(std::cmp::min(x1, x2))
}

fn max_exif_entry<T: std::cmp::Ord>(entry1: Option<T>, entry2: Option<T>) -> Option<T> {
    if entry1.is_none() || entry2.is_none() {
        return None;
    }

    let x1 = entry1.unwrap();
    let x2 = entry2.unwrap();

    Some(std::cmp::max(x1, x2))
}
