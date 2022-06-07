use serde_json::json;
use std::cmp::max;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Window;

pub struct StatusEmitter<T: Status> {
    pub status: Arc<Mutex<T>>,
    window: Arc<Mutex<Window>>,
    callback_event: String,
}

pub trait Status {
    fn json(&self) -> serde_json::Value;
    fn finished(&self) -> bool;
    fn start_update_emitter(status: Arc<Mutex<Self>>, callback_event: String, window: Window);
}

impl<T: Status> StatusEmitter<T> {
    pub fn new(status: Arc<Mutex<T>>, callback_event: String, window: Window) -> Self {
        Self {
            status,
            callback_event,
            window: Arc::new(Mutex::new(window)),
        }
    }

    fn run_update_worker(&self) {
        println!("Starting update emitter for '{}'", self.callback_event.as_str());

        loop {
            self.window
                .lock()
                .unwrap()
                .emit(self.callback_event.as_str(), Some(self.status.lock().unwrap().json()))
                .expect(format!("Failed to emit status for '{}'", self.callback_event).as_str());

            if self.status.lock().unwrap().finished() {
                break;
            }
            thread::sleep(Duration::from_millis(200))
        }

        println!("Stopped update emitter for '{}'", self.callback_event.as_str());
    }
}

#[derive(Clone)]
pub struct ProcessingStatus {
    pub count_lights: usize,
    pub count_darks: usize,
    count_loaded_lights: Arc<AtomicUsize>,
    count_loading_lights: Arc<AtomicUsize>,
    count_merge_completed: Arc<AtomicUsize>,
    count_merging: Arc<AtomicUsize>,
}

impl Status for ProcessingStatus {
    fn json(&self) -> serde_json::Value {
        json!({
            "count_lights": self.count_lights,
            "count_darks": self.count_darks,
            "count_loaded_lights": self.count_loaded_lights.load(Relaxed),
            "count_loading_lights": self.count_loading_lights.load(Relaxed),
            "count_merged": self.count_merge_completed.load(Relaxed),
            "count_merging": self.count_merging.load(Relaxed),
            "loading_done": self.loading_done(),
            "merging_done": self.merging_done()
        })
    }

    fn finished(&self) -> bool {
        self.loading_done() && self.merging_done()
    }

    fn start_update_emitter(status: Arc<Mutex<Self>>, callback_event: String, window: Window) {
        let emitter = StatusEmitter::new(status, callback_event.clone(), window);

        thread::spawn(move || {
            emitter.run_update_worker();
        });
    }
}

impl ProcessingStatus {
    pub fn new(count_lights: usize, count_darks: usize, callback_event: String, window: Window) -> Arc<Mutex<Self>> {
        let status = Arc::new(Mutex::new(ProcessingStatus {
            count_lights,
            count_darks,
            count_loaded_lights: Arc::new(AtomicUsize::new(0)),
            count_loading_lights: Arc::new(AtomicUsize::new(0)),
            count_merge_completed: Arc::new(AtomicUsize::new(0)),
            count_merging: Arc::new(AtomicUsize::new(0)),
        }));

        Self::start_update_emitter(status.clone(), callback_event, window);
        status
    }

    pub fn loading_done(&self) -> bool {
        self.count_lights + self.count_darks == self.count_loaded_lights.load(Relaxed)
    }

    pub fn merging_done(&self) -> bool {
        self.count_lights - 1 + max(0isize, self.count_darks as isize - 1) as usize
            == self.count_merge_completed.load(Relaxed)
    }

    pub fn start_loading(&self) {
        self.count_loading_lights.fetch_add(1, Relaxed);
        self.print_status();
    }

    pub fn finish_loading(&self) {
        self.count_loaded_lights.fetch_add(1, Relaxed);
        self.count_loading_lights.fetch_sub(1, Relaxed);
        self.print_status();
    }

    pub fn start_merging(&self) {
        self.count_merging.fetch_add(1, Relaxed);
        self.print_status();
    }

    pub fn finish_merging(&self) {
        self.count_merge_completed.fetch_add(1, Relaxed);
        self.count_merging.fetch_sub(1, Relaxed);
        self.print_status();
    }

    fn print_status(&self) {
        println!(
            "Total {}, Loaded {}, Loading {}, Merged {}, Merging {}, loading_done = {}, merging_done = {}",
            self.count_lights,
            self.count_loaded_lights.load(Relaxed),
            self.count_loading_lights.load(Relaxed),
            self.count_merge_completed.load(Relaxed),
            self.count_merging.load(Relaxed),
            self.loading_done(),
            self.merging_done()
        );
    }
}
