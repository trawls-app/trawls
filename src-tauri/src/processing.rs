use std::{thread, time};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;
use crate::processing::image::{Image, Mergable};

mod image;
pub mod status;

#[derive(Copy, Clone)]
pub enum CometMode {
    Falling,
    Raising,
    Normal,
}

pub fn run_merge(lightframe_files: Vec<PathBuf>, mode: CometMode, state: status::Status) {
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

    //write_ppm(&raw_image, Path::new("/home/chris/test.ppm"));

    /*let writer = libdng::DNGWriter::new(raw_image.width.try_into().unwrap(), raw_image.height.try_into().unwrap());
    writer.dummy();
    writer.build_negative(raw_image.raw_image_data);
    writer.write_tif(Path::new("/home/chris/test.tif"));
    writer.write_jpg(Path::new("/home/chris/test.jpg"));
    writer.write_dng(Path::new("/home/chris/test.dng"));*/
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

/*
fn write_ppm(image: &Image, output: &Path) {
    println!("Writing debug PPM to '{}'", output.display());
    // Write out the image as a grayscale PPM
    let mut f = BufWriter::new(File::create("/home/chris/test.ppm").unwrap());
    let preamble = format!("P6 {} {} {}\n", image.width, image.height, 65535).into_bytes();
    f.write_all(&preamble).unwrap();

    for pix in &image.raw_image_data {
        // Do an extremely crude "demosaic" by setting R=G=B
        let pixhigh = (pix>>8) as u8;
        let pixlow  = (pix&0x0f) as u8;
        f.write_all(&[pixhigh, pixlow, pixhigh, pixlow, pixhigh, pixlow]).unwrap()
    }
}*/