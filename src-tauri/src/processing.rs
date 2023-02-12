use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::{thread, time};

use anyhow::{self, Context};
use clap::ValueEnum;
use itertools::chain;
use log::{error, info};
use rawler::exif::Exif;
use rayon::prelude::*;
use serde::Serialize;

use crate::processing::image::MergeMode::{Maximize, WeightedAverage};
use crate::processing::image::{Image, Mergable, MergeMode};

use self::status::Status;

pub mod cli_progress;
mod dng_writing;
mod image;
pub mod status;

#[derive(Copy, Clone, ValueEnum)]
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
    fn new(preview_bytes: Vec<u8>, exif: Exif) -> RenderedPreview {
        let encoded = base64::encode(preview_bytes);

        let exposure_time = exif.exposure_time.unwrap_or_default();
        let exposure = time::Duration::from_secs_f64(exposure_time.n as f64 / exposure_time.d as f64);

        let seconds = exposure.as_secs() % 60;
        let minutes = (exposure.as_secs() / 60) % 60;
        let hours = (exposure.as_secs() / 60) / 60;

        RenderedPreview {
            encoded,
            aperture: format!("f/{:.1}", exif.fnumber.unwrap_or_default().as_f32()),
            exposure: format!("{}h{}m{}s", hours, minutes, seconds),
            isospeed: format!("ISO{}", exif.iso_speed_ratings.unwrap_or_default()),
        }
    }
}

pub fn run_merge(
    lightframe_files: Vec<PathBuf>,
    darkframe_files: Vec<PathBuf>,
    out_path_dng: Option<PathBuf>,
    out_path_preview: Option<PathBuf>,
    mode: Comets,
    state: Arc<Mutex<status::ProcessingStatus>>,
) -> anyhow::Result<RenderedPreview> {
    let num_threads = num_cpus::get();
    info!(
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
    let thread_handles_lights = spawn_workers(num_threads, Arc::clone(&queue_lights), Maximize, state.clone());
    let thread_handles_darks = spawn_workers(num_threads, Arc::clone(&queue_darks), WeightedAverage, state.clone());

    // Load light- and darkframes
    tasks
        .par_iter()
        .map(|t| {
            if state.lock().unwrap().aborted() {
                return Ok(());
            }
            if let Err(e) = load_image(t, Arc::clone(&queue_lights), Arc::clone(&queue_darks), mode, state.clone()) {
                state.lock().unwrap().abort();
                return Err(e);
            }

            Ok(())
        })
        .collect::<anyhow::Result<()>>()?;

    for t in chain(thread_handles_lights, thread_handles_darks) {
        t.join().unwrap()?;
    }

    let lightframe = queue_lights.lock().unwrap().pop().unwrap();
    let raw_image = if darkframe_files.is_empty() {
        info!("No darkframes selected");
        lightframe
    } else {
        info!("Subtracting averaged darkframe");
        let darkframe = queue_darks.lock().unwrap().pop().unwrap();
        lightframe.apply_darkframe(darkframe)?
    };

    info!("Processing done");

    // Write the result
    let exif = raw_image.exif.clone();
    let writer = raw_image.get_image_writer()?;
    if out_path_dng.is_some() {
        writer.write_dng(out_path_dng.unwrap())?;
    }

    if out_path_preview.is_some() {
        writer.write_preview_jpg(out_path_preview.unwrap())?;
    }

    // Create a preview to show in the UI
    let preview_bytes = writer.get_preview_bytes()?;

    Ok(RenderedPreview::new(preview_bytes, exif))
}

fn spawn_workers(
    num_threads: usize,
    queue: Arc<Mutex<Vec<Image>>>,
    merge_mode: MergeMode,
    state: Arc<Mutex<status::ProcessingStatus>>,
) -> Vec<JoinHandle<anyhow::Result<()>>> {
    let mut thread_handles = vec![];

    for _ in 0..num_threads {
        let q = Arc::clone(&queue);
        let s = state.clone();
        thread_handles.push(thread::spawn(move || -> anyhow::Result<()> { queue_worker(q, merge_mode, s) }));
    }

    thread_handles
}

fn queue_worker(
    queue: Arc<Mutex<Vec<Image>>>,
    merge_mode: MergeMode,
    state: Arc<Mutex<status::ProcessingStatus>>,
) -> anyhow::Result<()> {
    loop {
        // Stop if any other thread exited preliminary
        if state.lock().unwrap().aborted() {
            return Ok(());
        }

        let mut q = queue.lock().unwrap();
        if q.len() <= 1 {
            if state.lock().unwrap().loading_done() {
                return Ok(());
            } else {
                // Queue is empty but work is not done yet => Wait.
                drop(q);
                thread::sleep(time::Duration::from_millis(20));
                continue;
            }
        }
        state.lock().unwrap().start_merging();

        let v1 = q.pop().unwrap();
        let v2 = q.pop().unwrap();
        drop(q);

        let res = match v1.merge(v2, merge_mode).with_context(|| "Failed to merge images.") {
            Ok(x) => x,
            Err(err) => {
                error!("{:?}", err);
                state.lock().unwrap().abort();
                return Err(err);
            }
        };

        queue.lock().unwrap().push(res);
        state.lock().unwrap().finish_merging();
    }
}

fn load_image(
    task: &LoadTask,
    queue_lights: Arc<Mutex<Vec<Image>>>,
    queue_darks: Arc<Mutex<Vec<Image>>>,
    comets: Comets,
    state: Arc<Mutex<status::ProcessingStatus>>,
) -> anyhow::Result<()> {
    let count_lights = state.lock().unwrap().count_lights;
    state.lock().unwrap().start_loading();

    match task.frame_type {
        FrameType::Lightframe(index) => load_lightframe(task.path.as_path(), queue_lights, index, count_lights, comets)
            .with_context(|| format!("Could not load lightframe '{}'", task.path.display()))?,
        FrameType::Darkframe => load_darkframe(task.path.as_path(), queue_darks)
            .with_context(|| format!("Could not load darkframe '{}'", task.path.display()))?,
    }

    state.lock().unwrap().finish_loading();
    Ok(())
}

fn load_lightframe(
    entry: &Path,
    queue: Arc<Mutex<Vec<Image>>>,
    index: usize,
    num_images: usize,
    comets: Comets,
) -> anyhow::Result<()> {
    let intensity = match comets {
        Comets::Falling => 1.0 - index as f32 / num_images as f32,
        Comets::Raising => index as f32 / num_images as f32,
        Comets::Normal => 1.0,
    };

    let img = Image::from_raw_file(entry, intensity)?;
    queue.lock().unwrap().push(img);
    Ok(())
}

fn load_darkframe(entry: &Path, queue: Arc<Mutex<Vec<Image>>>) -> anyhow::Result<()> {
    let img = Image::from_raw_file(entry, 1.0)?;
    queue.lock().unwrap().push(img);
    Ok(())
}
