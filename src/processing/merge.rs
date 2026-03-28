use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{self, Context};
use log::info;
use rayon::prelude::*;

use crate::processing::{
    image::{Frame, Image},
    status::{ProcessingCounters, ProcessingStatus},
};

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

pub fn run_merge<S: ProcessingStatus + Send>(
    lightframe_files: Vec<PathBuf>,
    darkframe_files: Vec<PathBuf>,
    mode: Comets,
    status: Arc<Mutex<S>>,
) -> anyhow::Result<Image> {
    let num_threads = rayon::current_num_threads();
    info!("Using {} worker threads for merge.", num_threads);

    let mut tasks: Vec<LoadTask> = lightframe_files
        .iter()
        .enumerate()
        .map(|(i, p)| LoadTask {
            frame_type: FrameType::Lightframe(i),
            path: p.to_path_buf(),
        })
        .collect();
    tasks.extend(darkframe_files.iter().map(|p| LoadTask {
        frame_type: FrameType::Darkframe,
        path: p.to_path_buf(),
    }));

    let merge_total =
        lightframe_files.len().saturating_sub(1) + darkframe_files.len().saturating_sub(1);
    let counters = Arc::new(Mutex::new(ProcessingCounters {
        count_lights: lightframe_files.len(),
        count_darks: darkframe_files.len(),
        count_loaded: 0,
        count_loading: 0,
        count_merged: 0,
        count_merging: 0,
    }));
    emit_status(&status, &counters);

    let frame = tasks
        .par_iter()
        .map(|task| load_image(task, mode, &status, &counters))
        .reduce(
            || Ok(Box::new(Frame::identity())),
            |left, right| {
                let left = left?;
                let right = right?;
                start_merge(&status, &counters);
                let merged = left.merge(*right);
                finish_merge(&status, &counters);
                merged
            },
        );

    let mut counters_guard = counters.lock().expect("merge counters mutex poisoned");
    counters_guard.count_merging = 0;
    counters_guard.count_merged = counters_guard.count_merged.min(merge_total);
    status
        .lock()
        .expect("processing status mutex poisoned")
        .processing_updated(*counters_guard);
    drop(counters_guard);

    frame?.get_image()
}

fn comet_intensity(mode: Comets, index: usize, count: usize) -> f32 {
    let count = count.max(1);
    match mode {
        Comets::Falling => 1.0 - index as f32 / count as f32,
        Comets::Raising => index as f32 / count as f32,
        Comets::Normal => 1.0,
    }
}

fn load_image<S: ProcessingStatus + Send>(
    task: &LoadTask,
    comets: Comets,
    status: &Arc<Mutex<S>>,
    counters: &Arc<Mutex<ProcessingCounters>>,
) -> anyhow::Result<Box<Frame>> {
    let count_lights = counters
        .lock()
        .expect("merge counters mutex poisoned")
        .count_lights
        .max(1);
    let intensity = match task.frame_type {
        FrameType::Lightframe(index) => comet_intensity(comets, index, count_lights),
        FrameType::Darkframe => 1.0,
    };

    start_load(status, counters);
    let img = Image::from_raw_file(task.path.as_path(), intensity)
        .with_context(|| format!("Could not load file {:?}", task.path))?;
    finish_load(status, counters);

    let frame = match task.frame_type {
        FrameType::Lightframe(_) => Frame::from_lightframe(img),
        FrameType::Darkframe => Frame::from_darkframe(img),
    };
    Ok(Box::new(frame))
}

fn start_load<S: ProcessingStatus + Send>(
    status: &Arc<Mutex<S>>,
    counters: &Arc<Mutex<ProcessingCounters>>,
) {
    let mut counters_guard = counters.lock().expect("merge counters mutex poisoned");
    counters_guard.count_loading += 1;
    status
        .lock()
        .expect("processing status mutex poisoned")
        .processing_updated(*counters_guard);
}

fn finish_load<S: ProcessingStatus + Send>(
    status: &Arc<Mutex<S>>,
    counters: &Arc<Mutex<ProcessingCounters>>,
) {
    let mut counters_guard = counters.lock().expect("merge counters mutex poisoned");
    counters_guard.count_loading = counters_guard.count_loading.saturating_sub(1);
    counters_guard.count_loaded += 1;
    status
        .lock()
        .expect("processing status mutex poisoned")
        .processing_updated(*counters_guard);
}

fn start_merge<S: ProcessingStatus + Send>(
    status: &Arc<Mutex<S>>,
    counters: &Arc<Mutex<ProcessingCounters>>,
) {
    let mut counters_guard = counters.lock().expect("merge counters mutex poisoned");
    counters_guard.count_merging += 1;
    status
        .lock()
        .expect("processing status mutex poisoned")
        .processing_updated(*counters_guard);
}

fn finish_merge<S: ProcessingStatus + Send>(
    status: &Arc<Mutex<S>>,
    counters: &Arc<Mutex<ProcessingCounters>>,
) {
    let mut counters_guard = counters.lock().expect("merge counters mutex poisoned");
    counters_guard.count_merging = counters_guard.count_merging.saturating_sub(1);
    counters_guard.count_merged += 1;
    status
        .lock()
        .expect("processing status mutex poisoned")
        .processing_updated(*counters_guard);
}

fn emit_status<S: ProcessingStatus + Send>(
    status: &Arc<Mutex<S>>,
    counters: &Arc<Mutex<ProcessingCounters>>,
) {
    let counters_guard = counters.lock().expect("merge counters mutex poisoned");
    status
        .lock()
        .expect("processing status mutex poisoned")
        .processing_updated(*counters_guard);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_mode_always_one() {
        for i in 0..10 {
            assert!((comet_intensity(Comets::Normal, i, 10) - 1.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn falling_mode_decreases() {
        let intensities: Vec<f32> = (0..5).map(|i| comet_intensity(Comets::Falling, i, 5)).collect();
        assert!((intensities[0] - 1.0).abs() < f32::EPSILON);
        for w in intensities.windows(2) {
            assert!(w[0] > w[1]);
        }
    }

    #[test]
    fn raising_mode_increases() {
        let intensities: Vec<f32> = (0..5).map(|i| comet_intensity(Comets::Raising, i, 5)).collect();
        assert!((intensities[0] - 0.0).abs() < f32::EPSILON);
        for w in intensities.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn falling_and_raising_complement() {
        for i in 0..10 {
            let f = comet_intensity(Comets::Falling, i, 10);
            let r = comet_intensity(Comets::Raising, i, 10);
            assert!((f + r - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn zero_count_does_not_panic() {
        let _ = comet_intensity(Comets::Normal, 0, 0);
        let _ = comet_intensity(Comets::Falling, 0, 0);
        let _ = comet_intensity(Comets::Raising, 0, 0);
    }

    #[test]
    fn single_frame() {
        assert!((comet_intensity(Comets::Normal, 0, 1) - 1.0).abs() < f32::EPSILON);
        assert!((comet_intensity(Comets::Falling, 0, 1) - 1.0).abs() < f32::EPSILON);
        assert!((comet_intensity(Comets::Raising, 0, 1) - 0.0).abs() < f32::EPSILON);
    }
}
