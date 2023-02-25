use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time;

use anyhow::{self, Context};
use clap::ValueEnum;
use log::info;
use rawler::exif::Exif;
use rayon::prelude::*;
use serde::Serialize;

use crate::processing::image::{Frame, Image};

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
    pub fn new(preview_bytes: Vec<u8>, exif: Exif) -> RenderedPreview {
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
    mode: Comets,
    state: Arc<Mutex<status::ProcessingStatus>>,
) -> anyhow::Result<Image> {
    let num_threads = num_cpus::get();
    info!(
        "System has {} cores and {} threads. Using {} worker threads.",
        num_cpus::get_physical(),
        num_threads,
        num_threads
    );

    // Create loading tasks for lightframes
    let mut tasks: Vec<LoadTask> = lightframe_files
        .iter()
        .enumerate()
        .map(|(i, p)| LoadTask {
            frame_type: FrameType::Lightframe(i),
            path: p.to_path_buf(),
        })
        .collect();

    // Add darkframes to the tasklist
    tasks.append(
        &mut darkframe_files
            .iter()
            .map(|p| LoadTask {
                frame_type: FrameType::Darkframe,
                path: p.to_path_buf(),
            })
            .collect(),
    );

    // Loading and merging
    let frame = tasks
        .par_iter()
        .map(|t| load_image(t, mode, state.clone()))
        .reduce(|| Ok(Box::new(Frame::identity())), |x, y| x?.merge(*y?, state.clone()));

    if frame.is_err() {
        state.lock().unwrap().abort();
    }

    frame?.get_image()
}

fn load_image(
    task: &LoadTask,
    comets: Comets,
    state: Arc<Mutex<status::ProcessingStatus>>,
) -> anyhow::Result<Box<Frame>> {
    let count_lights = state.lock().unwrap().count_lights;
    let intensity = match task.frame_type {
        FrameType::Lightframe(index) => match comets {
            Comets::Falling => 1.0 - index as f32 / count_lights as f32,
            Comets::Raising => index as f32 / count_lights as f32,
            Comets::Normal => 1.0,
        },
        FrameType::Darkframe => 1.0,
    };

    state.lock().unwrap().start_loading();
    let img = Image::from_raw_file(task.path.as_path(), intensity)
        .with_context(|| format!("Could not load file {:#?}", task.path))?;
    state.lock().unwrap().finish_loading();

    let frame = match task.frame_type {
        FrameType::Lightframe(_) => Frame::from_lightframe(img),
        FrameType::Darkframe => Frame::from_darkframe(img),
    };

    Ok(Box::new(frame))
}
