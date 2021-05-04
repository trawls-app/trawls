use std::{thread, time};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::fs;
use std::time::Duration;
use serde::Serialize;
use rayon::prelude::*;
use tempfile::tempdir;
use base64;
use libdng::image_info::DNGWriting;
use libdng::exif::ExifExtractable;
use libdng::bindings::{ExifTag_Photo_FNumber, ExifTag_Photo_ExposureTime, ExifTag_Photo_ISOSpeedRatings};
use crate::processing::image::{Image, Mergable};
use num::rational::Ratio;
use std::cmp::min;


mod image;
pub mod status;

#[derive(Copy, Clone)]
pub enum CometMode {
    Falling,
    Raising,
    Normal,
}

#[derive(Serialize)]
pub struct RenderedPreview {
    pub encoded: String,
    pub aperture: String,
    pub exposure: String,
    pub isospeed: String
}

impl RenderedPreview {
    fn new(image_path: &Path, exif: Box<dyn ExifExtractable>) -> RenderedPreview {
        let image_bytes = fs::read(image_path).unwrap();
        let encoded = base64::encode(image_bytes);
        let aperture_ratio = exif.get_urational(ExifTag_Photo_FNumber, 0).unwrap_or(Ratio::new(0, 1));
        let exposure_ratio = exif.get_urational(ExifTag_Photo_ExposureTime, 0).unwrap_or(Ratio::new(0, 1));
        let isospeed = exif.get_uint(ExifTag_Photo_ISOSpeedRatings, 0).unwrap_or(0);

        let aperture = *aperture_ratio.numer() as f32 / *aperture_ratio.denom() as f32;
        let exposure = Duration::from_secs_f64(*exposure_ratio.numer() as f64 / *exposure_ratio.denom() as f64);

        let seconds = exposure.as_secs() % 60;
        let minutes = (exposure.as_secs() / 60) % 60;
        let hours = (exposure.as_secs() / 60) / 60;

        RenderedPreview {
            encoded,
            aperture: format!("f/{}", aperture),
            exposure: format!("{}h{}m{}s", hours, minutes, seconds),
            isospeed: format!("ISO{}", isospeed)
        }
    }
}

pub fn run_merge(lightframe_files: Vec<PathBuf>, out_path: PathBuf, mode: CometMode, state: status::Status) -> RenderedPreview {
    let num_threads = num_cpus::get();
    println!("System has {} cores and {} threads. Using {} worker threads.", num_cpus::get_physical(), num_threads, num_threads);


    let result = Arc::new(Mutex::new(vec![]));
    let mut thread_handles = vec![];

    for _ in 0..num_threads {
        let q = Arc::clone(&result);
        let s = state.clone();
        thread_handles.push(thread::spawn(move || {
            queue_worker(q, s);
        }));
    }

    lightframe_files.par_iter()
        .zip(0..lightframe_files.len())
        .for_each(|(e, i)| process_image(e.as_path(), Arc::clone(&result), i, lightframe_files.len(), mode, state.clone()));

    for t in thread_handles {
        t.join().unwrap_or(());
    }

    let mut data = result.lock().unwrap();
    let raw_image = data.pop().unwrap();

    state.update_status(true);
    println!("Processing done");
    raw_image.exif.print_all();

    // Write the result
    let writer = raw_image.get_dng_writer();
    writer.write_dng(&out_path);

    // Create a preview to show in the UI
    let dir = tempdir().unwrap();
    let preview_path = dir.path().join("preview.jpg");
    writer.write_jpg(preview_path.as_path());
    
    RenderedPreview::new(preview_path.as_path(), Box::new(raw_image.exif))
}


fn queue_worker(queue: Arc<Mutex<Vec<Image>>>, state: status::Status) {
    loop {
        let mut q = queue.lock().unwrap();
        if q.len() <= 1 {
            if state.loading_done() { state.update_status(true); return; } else {
                // Queue is empty but work is not done yet => Wait.
                drop(q);
                thread::sleep(time::Duration::from_millis(20));
                continue;
            }
        }
        state.start_merging();

        let v1 = q.pop().unwrap();
        let v2 = q.pop().unwrap();
        drop(q);

        let res = v1.merge(v2);
        queue.lock().unwrap().push(res);
        state.finish_merging();
    }
}


fn process_image(entry: &Path, queue: Arc<Mutex<Vec<Image>>>, index: usize, num_images: usize, mode: CometMode, state: status::Status) {
    state.start_loading();

    let intensity = match mode {
        CometMode::Falling => 1.0 - index as f32 / num_images as f32,
        CometMode::Raising => index as f32 / num_images as f32,
        CometMode::Normal => 1.0,
    };

    let img = Image::load_from_raw(entry, intensity).unwrap();
    queue.lock().unwrap().push(img);

    state.finish_loading();
}