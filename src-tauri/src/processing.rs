use std::{io, thread, time};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use rayon::prelude::*;

mod image;

#[derive(Copy, Clone)]
pub enum CometMode {
    Falling,
    Raising,
    Normal,
}

pub fn run_merge(lightframe_files: Vec<PathBuf>, mode: CometMode) {
    let num_threads = num_cpus::get();
    println!("System has {} cores and {} threads. Using {} worker threads.", num_cpus::get_physical(), num_threads, num_threads);


    let result = Arc::new(Mutex::new(vec![]));
    let done = Arc::new(AtomicBool::new(false));
    let mut thread_handles = vec![];

    for _ in 0..num_threads {
        let q = Arc::clone(&result);
        let d = Arc::clone(&done);
        thread_handles.push(thread::spawn(move || {
            queue_worker(q, d);
        }));
    }

    lightframe_files.par_iter()
        .zip(0..lightframe_files.len())
        .for_each(|(e, i)| process_image(e, Arc::clone(&result), i, lightframe_files.len(), mode));

    done.store(true, Ordering::Relaxed);
    for t in thread_handles {
        t.join().unwrap_or(());
    }

    let mut data = result.lock().unwrap();
    let raw_image = data.pop().unwrap();
}


fn queue_worker(queue: Arc<Mutex<Vec<image::Image>>>, done: Arc<AtomicBool>) {
    loop {
        let mut q = queue.lock().unwrap();
        if q.len() <= 1 {
            if done.load(Ordering::Relaxed) { return; } else {
                // Queue is empty but work is not done yet => Wait.
                drop(q);
                thread::sleep(time::Duration::from_millis(20));
                continue;
            }
        }

        let v1 = q.pop().unwrap();
        let v2 = q.pop().unwrap();
        drop(q);

        let res = v1.merge(v2);
        queue.lock().unwrap().push(res);
    }
}


fn process_image(entry: &PathBuf, queue: Arc<Mutex<Vec<image::Image>>>, index: usize, num_images: usize, mode: CometMode) {
    let intensity = match mode {
        CometMode::Falling => 1.0 - index as f32 / num_images as f32,
        CometMode::Raising => index as f32 / num_images as f32,
        CometMode::Normal => 1.0,
    };

    let img = image::Image::load_from_raw(entry.as_path(), intensity).unwrap();
    queue.lock().unwrap().push(img);
}