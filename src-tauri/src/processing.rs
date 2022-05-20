use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{fs, thread, time};

use itertools::chain;
use num::rational::Ratio;
use rayon::prelude::*;
use serde::Serialize;
use tempfile::tempdir;

use crate::processing::image::MergeMode::{Maximize, WeightedAverage};
use crate::processing::image::{Image, Mergable, MergeMode};
use libdng::bindings::{
    ExifTag_Photo_ExposureTime, ExifTag_Photo_FNumber, ExifTag_Photo_ISOSpeedRatings,
};
use libdng::exif::ExifExtractable;
use libdng::image_info::DNGWriting;

mod image;
pub mod status;

#[derive(Copy, Clone)]
pub enum Comets {
    Falling,
    Raising,
    Normal,
}

enum FrameType {
    Lightframe(usize),
    Darkframe,
}

struct LoadTask {
    frame_type: FrameType,
    path: PathBuf,
}

#[derive(Serialize)]
pub struct RenderedPreview {
    pub encoded: String,
    pub aperture: String,
    pub exposure: String,
    pub isospeed: String,
}

impl RenderedPreview {
    fn new(image_path: &Path, exif: Box<dyn ExifExtractable>) -> RenderedPreview {
        let image_bytes = fs::read(image_path).unwrap();
        let encoded = base64::encode(image_bytes);
        let aperture_ratio = exif
            .get_urational(ExifTag_Photo_FNumber, 0)
            .unwrap_or_else(|| Ratio::new(0, 1));
        let exposure_ratio = exif
            .get_urational(ExifTag_Photo_ExposureTime, 0)
            .unwrap_or_else(|| Ratio::new(0, 1));
        let isospeed = exif.get_uint(ExifTag_Photo_ISOSpeedRatings, 0).unwrap_or(0);

        let aperture = *aperture_ratio.numer() as f32 / *aperture_ratio.denom() as f32;
        let exposure = time::Duration::from_secs_f64(
            *exposure_ratio.numer() as f64 / *exposure_ratio.denom() as f64,
        );

        let seconds = exposure.as_secs() % 60;
        let minutes = (exposure.as_secs() / 60) % 60;
        let hours = (exposure.as_secs() / 60) / 60;

        RenderedPreview {
            encoded,
            aperture: format!("f/{}", aperture),
            exposure: format!("{}h{}m{}s", hours, minutes, seconds),
            isospeed: format!("ISO{}", isospeed),
        }
    }
}

pub fn run_merge(
    lightframe_files: Vec<PathBuf>,
    darkframe_files: Vec<PathBuf>,
    out_path: PathBuf,
    mode: Comets,
    state: status::Status,
) -> RenderedPreview {
    let num_threads = num_cpus::get();
    println!(
        "System has {} cores and {} threads. Using {} worker threads.",
        num_cpus::get_physical(),
        num_threads,
        num_threads
    );

    let queue_lights = Arc::new(Mutex::new(vec![]));
    let queue_darks = Arc::new(Mutex::new(vec![]));
    let mut tasks: Vec<LoadTask> = lightframe_files
        .iter()
        .enumerate()
        .map(|(i, p)| LoadTask {
            frame_type: FrameType::Lightframe(i),
            path: p.to_path_buf(),
        })
        .collect();
    tasks.append(
        &mut darkframe_files
            .iter()
            .map(|p| LoadTask {
                frame_type: FrameType::Darkframe,
                path: p.to_path_buf(),
            })
            .collect(),
    );

    // Start workers
    let thread_handles_lights =
        spawn_workers(num_threads, Arc::clone(&queue_lights), Maximize, state.clone());
    let thread_handles_darks =
        spawn_workers(num_threads, Arc::clone(&queue_darks), WeightedAverage, state.clone());

    // Load light- and darkframes
    tasks.par_iter().for_each(|t| {
        load_image(t, Arc::clone(&queue_lights), Arc::clone(&queue_darks), mode, state.clone())
    });

    for t in chain(thread_handles_lights, thread_handles_darks) {
        t.join().unwrap_or(());
    }

    let lightframe = queue_lights.lock().unwrap().pop().unwrap();
    let raw_image = if darkframe_files.is_empty() {
        println!("No darkframes selected");
        lightframe
    } else {
        println!("Subtracting averaged darkframe");
        let darkframe = queue_darks.lock().unwrap().pop().unwrap();
        lightframe.apply_darkframe(darkframe)
    };

    state.update_status(true);
    println!("Processing done");
    raw_image.exif.print_all();

    // Create a temporary directory to store the result and preview for further handling
    let dir = tempdir().unwrap();

    // Write the result
    let writer = raw_image.get_dng_writer();
    let result_path = dir.path().join("result.dng");
    writer.write_dng(&result_path);

    println!("Copying from '{}' to '{}'", result_path.display(), out_path.display());
    fs::copy(result_path, out_path).unwrap();

    // Create a preview to show in the UI
    let preview_path = dir.path().join("preview.jpg");
    writer.write_jpg(preview_path.as_path());

    RenderedPreview::new(preview_path.as_path(), Box::new(raw_image.exif))
}

fn spawn_workers(
    num_threads: usize,
    queue: Arc<Mutex<Vec<Image>>>,
    merge_mode: MergeMode,
    state: status::Status,
) -> Vec<JoinHandle<()>> {
    let mut thread_handles = vec![];

    for _ in 0..num_threads {
        let q = Arc::clone(&queue);
        let s = state.clone();
        thread_handles.push(thread::spawn(move || {
            queue_worker(q, merge_mode, s);
        }));
    }

    thread_handles
}

fn queue_worker(queue: Arc<Mutex<Vec<Image>>>, merge_mode: MergeMode, state: status::Status) {
    loop {
        let mut q = queue.lock().unwrap();
        if q.len() <= 1 {
            if state.loading_done() {
                state.update_status(true);
                return;
            } else {
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

        let res = v1.merge(v2, merge_mode);
        queue.lock().unwrap().push(res);
        state.finish_merging();
    }
}

fn load_image(
    task: &LoadTask,
    queue_lights: Arc<Mutex<Vec<Image>>>,
    queue_darks: Arc<Mutex<Vec<Image>>>,
    comets: Comets,
    state: status::Status,
) {
    state.start_loading();

    match task.frame_type {
        FrameType::Lightframe(index) => {
            load_lightframe(task.path.as_path(), queue_lights, index, state.count_lights, comets)
        }
        FrameType::Darkframe => load_darkframe(task.path.as_path(), queue_darks),
    }

    state.finish_loading();
}

fn load_lightframe(
    entry: &Path,
    queue: Arc<Mutex<Vec<Image>>>,
    index: usize,
    num_images: usize,
    comets: Comets,
) {
    let intensity = match comets {
        Comets::Falling => 1.0 - index as f32 / num_images as f32,
        Comets::Raising => index as f32 / num_images as f32,
        Comets::Normal => 1.0,
    };

    let img = Image::load_from_raw(entry, intensity).unwrap();
    queue.lock().unwrap().push(img);
}

fn load_darkframe(entry: &Path, queue: Arc<Mutex<Vec<Image>>>) {
    let img = Image::load_from_raw(entry, 1.0).unwrap();
    queue.lock().unwrap().push(img);
}
