use std::{thread, time};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;
use crate::processing::image::{Image, Mergable};
use libdng::image_info::DNGWriting;

mod image;
pub mod status;

#[derive(Copy, Clone)]
pub enum CometMode {
    Falling,
    Raising,
    Normal,
}

pub fn run_merge(lightframe_files: Vec<PathBuf>, out_path: PathBuf, mode: CometMode, state: status::Status) {
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
    println!("Processing done");
    raw_image.exif.print_mapped();

    let writer = raw_image.get_dng_writer();
    //writer.write_jpg(Path::new("/home/chris/test.jpg"));
    writer.write_dng(&out_path);
}


fn queue_worker(queue: Arc<Mutex<Vec<Image>>>, state: status::Status) {
    loop {
        let mut q = queue.lock().unwrap();
        if q.len() <= 1 {
            if state.loading_done() { return; } else {
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