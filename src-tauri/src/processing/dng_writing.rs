use crate::anyhow::Context;
use crate::program_description;
use std::{fs::File, path::PathBuf};

use image::{DynamicImage, ImageBuffer};
use log::info;
use rawler::{
    dng::{
        dngwriter::{
            dng_put_preview, dng_put_raw_ljpeg, dng_put_thumbnail,
            fill_exif_ifd, fill_exif_root, matrix_to_tiff_value, wbcoeff_to_tiff_value,
        },
        rect_to_dng_area,
    },
    exif::Exif,
    formats::tiff::{CompressionMethod, DirectoryWriter, PhotometricInterpretation, Rational, TiffWriter},
    imgop::{raw::develop_raw_srgb, rescale_f32_to_u16, xyz::Illuminant, Dim2, Point, Rect},
    tags::{DngTag, ExifTag, TiffCommonTag},
    RawImage, RawImageData,
};
use uuid::Uuid;

const LJ92_PREDICTOR: u8 = 7;
const DNG_VERSION_V1_1: [u8; 4] = [1, 1, 0, 0];
const DNG_VERSION_V1_6: [u8; 4] = [1, 6, 0, 0];

pub struct ImageWriter {
    raw_image: RawImage,
    preview: DynamicImage,
    exif: Exif,
}

impl ImageWriter {
    pub fn new(raw_image: RawImage, exif: Exif) -> anyhow::Result<Self> {
        let params = raw_image.develop_params()?;
        let buf = match &raw_image.data {
            RawImageData::Integer(buf) => buf,
            RawImageData::Float(_) => todo!(),
        };

        // Generate preview image
        info!("Rendering preview of image...");
        let (srgbf, dim) = develop_raw_srgb(buf, &params).unwrap(); // Use anyhow
        let output = rescale_f32_to_u16(&srgbf, 0, u16::MAX);
        let preview = DynamicImage::ImageRgb16(
            ImageBuffer::from_raw(dim.w as u32, dim.h as u32, output).expect("Invalid ImageBuffer size"),
        );

        Ok(Self {
            raw_image,
            preview,
            exif,
        })
    }

    pub fn write_dng(&self, path: PathBuf) -> anyhow::Result<()> {
        info!("Writing DNG to {:?}...", path);

        let mut output =
            File::create(path.clone()).with_context(|| format!("Error while opening {:#?} for writing.", path))?;

        let wb_coeff = wbcoeff_to_tiff_value(&self.raw_image);

        let mut dng = TiffWriter::new(&mut output)?;
        let mut root_ifd = dng.new_directory();

        fill_exif_root(&mut root_ifd, &self.exif)?;

        // Create a unique image id
        let uiid = Uuid::new_v4().to_bytes_le();
        root_ifd.add_tag(DngTag::RawDataUniqueID, uiid)?;

        // Add a thumbnail
        root_ifd.add_tag(TiffCommonTag::NewSubFileType, 1_u16)?;
        dng_put_thumbnail(&mut root_ifd, &self.preview).unwrap();

        // Add basic info
        root_ifd.add_tag(TiffCommonTag::Software, &program_description())?;
        root_ifd.add_tag(DngTag::DNGVersion, &DNG_VERSION_V1_6[..])?;
        root_ifd.add_tag(DngTag::DNGBackwardVersion, &DNG_VERSION_V1_1[..])?;
        root_ifd.add_tag(TiffCommonTag::Make, self.raw_image.clean_make.as_str())?;
        root_ifd.add_tag(TiffCommonTag::Model, self.raw_image.clean_model.as_str())?;
        let uq_model = format!("{} {}", self.raw_image.clean_make, self.raw_image.clean_model);
        root_ifd.add_tag(DngTag::UniqueCameraModel, uq_model.as_str())?;
        root_ifd.add_tag(ExifTag::ModifyDate, chrono::Local::now().format("%Y:%m:%d %H:%M:%S").to_string())?;

        // Add matrix and illumninant
        let mut available_matrices = self.raw_image.color_matrix.clone();
        if let Some(first_key) = available_matrices.keys().next().cloned() {
            let first_matrix = available_matrices
                .remove_entry(&Illuminant::A)
                .or_else(|| available_matrices.remove_entry(&Illuminant::A))
                .or_else(|| available_matrices.remove_entry(&first_key))
                .expect("No matrix found");
            root_ifd.add_tag(DngTag::CalibrationIlluminant1, u16::from(first_matrix.0))?;
            root_ifd.add_tag(DngTag::ColorMatrix1, matrix_to_tiff_value(&first_matrix.1, 10_000).as_slice())?;

            if let Some(second_matrix) = available_matrices
                .remove_entry(&Illuminant::D65)
                .or_else(|| available_matrices.remove_entry(&Illuminant::D50))
            {
                root_ifd.add_tag(DngTag::CalibrationIlluminant2, u16::from(second_matrix.0))?;
                root_ifd.add_tag(DngTag::ColorMatrix2, matrix_to_tiff_value(&second_matrix.1, 10_000).as_slice())?;
            }
        }

        // Add White balance info
        root_ifd.add_tag(DngTag::AsShotNeutral, &wb_coeff[..])?;

        // Add EXIF information
        let exif_offset = {
            let mut exif_ifd = root_ifd.new_directory();
            // Add EXIF version 0220
            exif_ifd.add_tag_undefined(ExifTag::ExifVersion, vec![48, 50, 50, 48])?;
            fill_exif_ifd(&mut exif_ifd, &self.exif)?;
            //decoder.populate_dng_exif(&mut exif_ifd).unwrap();
            exif_ifd.build()?
        };
        root_ifd.add_tag(TiffCommonTag::ExifIFDPointer, exif_offset)?;

        // Create SubIFDs for the raw image data and preview image
        let mut sub_ifds = Vec::new();

        // Add raw image
        let raw_offset = {
            let mut raw_ifd = root_ifd.new_directory();
            self.put_raw(&mut raw_ifd)?;
            raw_ifd.build()?
        };
        sub_ifds.push(raw_offset);

        // Add preview image
        let preview_offset = {
            let mut prev_image_ifd = root_ifd.new_directory();
            dng_put_preview(&mut prev_image_ifd, &self.preview)?;
            prev_image_ifd.build()?
        };
        sub_ifds.push(preview_offset);

        // Finalize DNG file by updating IFD0 offset
        root_ifd.add_tag(TiffCommonTag::SubIFDs, &sub_ifds)?;
        let ifd0_offset = root_ifd.build()?;
        dng.build(ifd0_offset)?;

        Ok(())
    }

    fn put_raw(&self, raw_ifd: &mut DirectoryWriter<'_, '_>) -> anyhow::Result<()> {
        let full_size = Rect::new(Point::new(0, 0), Dim2::new(self.raw_image.width, self.raw_image.height));
        let active_area = self.raw_image.active_area.unwrap_or(full_size);

        assert!(active_area.p.x + active_area.d.w <= self.raw_image.width);
        assert!(active_area.p.y + active_area.d.h <= self.raw_image.height);

        raw_ifd.add_tag(TiffCommonTag::NewSubFileType, 0_u16)?; // Raw
        raw_ifd.add_tag(TiffCommonTag::ImageWidth, self.raw_image.width as u32)?;
        raw_ifd.add_tag(TiffCommonTag::ImageLength, self.raw_image.height as u32)?;

        raw_ifd.add_tag(DngTag::ActiveArea, rect_to_dng_area(&active_area))?;

        raw_ifd.add_tag(ExifTag::PlanarConfiguration, 1_u16)?;

        raw_ifd.add_tag(
            DngTag::DefaultScale,
            [
                Rational::new(self.raw_image.camera.default_scale[0][0], self.raw_image.camera.default_scale[0][1]),
                Rational::new(self.raw_image.camera.default_scale[1][0], self.raw_image.camera.default_scale[1][1]),
            ],
        )?;
        raw_ifd.add_tag(
            DngTag::BestQualityScale,
            Rational::new(self.raw_image.camera.best_quality_scale[0], self.raw_image.camera.best_quality_scale[1]),
        )?;

        // Whitelevel
        assert_eq!(self.raw_image.whitelevel.len(), self.raw_image.cpp, "Whitelevel sample count must match cpp");
        raw_ifd.add_tag(DngTag::WhiteLevel, &self.raw_image.whitelevel)?;

        // Blacklevel
        let blacklevel = self.raw_image.blacklevel.shift(active_area.p.x, active_area.p.y);

        raw_ifd.add_tag(DngTag::BlackLevelRepeatDim, [blacklevel.height as u16, blacklevel.width as u16])?;

        assert!(
            blacklevel.sample_count() == self.raw_image.cpp
                || blacklevel.sample_count()
                    == self.raw_image.cfa.width * self.raw_image.cfa.height * self.raw_image.cpp
        );
        if blacklevel.levels.iter().all(|x| x.d == 1) {
            let payload: Vec<u16> = blacklevel.levels.iter().map(|x| x.n as u16).collect();
            raw_ifd.add_tag(DngTag::BlackLevel, &payload)?;
        } else {
            raw_ifd.add_tag(DngTag::BlackLevel, blacklevel.levels.as_slice())?;
        }

        if !self.raw_image.blackareas.is_empty() {
            let data: Vec<u16> = self.raw_image.blackareas.iter().flat_map(rect_to_dng_area).collect();
            raw_ifd.add_tag(DngTag::MaskedAreas, &data)?;
        }
        raw_ifd.add_tag(TiffCommonTag::PhotometricInt, PhotometricInterpretation::CFA)?;
        raw_ifd.add_tag(TiffCommonTag::SamplesPerPixel, 1_u16)?;
        raw_ifd.add_tag(TiffCommonTag::BitsPerSample, [16_u16])?;

        let cfa = self.raw_image.cfa.shift(active_area.p.x, active_area.p.y);

        raw_ifd.add_tag(TiffCommonTag::CFARepeatPatternDim, [cfa.width as u16, cfa.height as u16])?;
        raw_ifd.add_tag(TiffCommonTag::CFAPattern, &cfa.flat_pattern()[..])?;

        //raw_ifd.add_tag(DngTag::CFAPlaneColor, [0u8, 1u8, 2u8])?; // RGB

        //raw_ifd.add_tag(DngTag::CFAPlaneColor, [1u8, 4u8, 3u8, 5u8])?; // RGB

        raw_ifd.add_tag(DngTag::CFALayout, 1_u16)?; // Square layout

        //raw_ifd.add_tag(LegacyTiffRootTag::CFAPattern, [0u8, 1u8, 1u8, 2u8])?; // RGGB
        //raw_ifd.add_tag(LegacyTiffRootTag::CFARepeatPatternDim, [2u16, 2u16])?;
        //raw_ifd.add_tag(DngTag::CFAPlaneColor, [0u8, 1u8, 2u8])?; // RGGB

        raw_ifd.add_tag(TiffCommonTag::Compression, CompressionMethod::ModernJPEG)?;
        dng_put_raw_ljpeg(raw_ifd, &self.raw_image, LJ92_PREDICTOR)?;
        //raw_ifd.add_tag(TiffCommonTag::Compression, CompressionMethod::None)?;
        //dng_put_raw_uncompressed(raw_ifd, &self.raw_image)?;

        for (tag, value) in self.raw_image.dng_tags.iter() {
            raw_ifd.add_untyped_tag(*tag, value.clone())?;
        }

        Ok(())
    }

    pub fn write_preview_jpg(&self, path: PathBuf) -> anyhow::Result<()> {
        info!("Writing preview to {:?}...", path);
        let img = self.preview.clone().into_rgb8();
        img.save(path)?;

        Ok(())
    }
}
