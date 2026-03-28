use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use image::GenericImageView;
use image::imageops::FilterType;
use rawler::decoders::RawDecodeParams;

const THUMBNAIL_MAX_WIDTH: u32 = 60;
const THUMBNAIL_MAX_HEIGHT: u32 = 44;

pub fn extract_thumbnail(path: &Path) -> anyhow::Result<Arc<gpui::Image>> {
    let dynamic_image = rawler::analyze::extract_thumbnail_pixels(path, RawDecodeParams::default())
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let resized = dynamic_image.resize(
        THUMBNAIL_MAX_WIDTH,
        THUMBNAIL_MAX_HEIGHT,
        FilterType::Triangle,
    );

    let mut jpeg_bytes = Vec::new();
    resized.write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)?;

    Ok(Arc::new(gpui::Image::from_bytes(
        gpui::ImageFormat::Jpeg,
        jpeg_bytes,
    )))
}

pub struct PreviewResult {
    pub image: Arc<gpui::Image>,
    pub width_px: u32,
    pub height_px: u32,
}

pub fn extract_preview(path: &Path) -> anyhow::Result<PreviewResult> {
    let dynamic_image = rawler::analyze::extract_preview_pixels(path, RawDecodeParams::default())
        .or_else(|_| rawler::analyze::extract_thumbnail_pixels(path, RawDecodeParams::default()))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let (width_px, height_px) = dynamic_image.dimensions();

    let mut jpeg_bytes = Vec::new();
    dynamic_image.write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)?;

    Ok(PreviewResult {
        image: Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Jpeg, jpeg_bytes)),
        width_px,
        height_px,
    })
}
