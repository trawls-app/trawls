use log::{debug, warn};
use serde_json::{json, Map};
use std::cmp::max;
use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicUsize};
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
    fn aborted(&self) -> bool;
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
        debug!("Starting update emitter for '{}'", self.callback_event.as_str());

        loop {
            if self.status.lock().unwrap().aborted() {
                break;
            }

            self.window
                .lock()
                .unwrap()
                .emit(self.callback_event.as_str(), Some(self.status.lock().unwrap().json()))
                .expect(format!("Failed to emit status for '{}'", self.callback_event).as_str());

            if self.status.lock().unwrap().finished() {
                break;
            }
            thread::sleep(Duration::from_millis(500))
        }

        debug!("Stopped update emitter for '{}'", self.callback_event.as_str());
    }
}

pub struct InfoLoadingStatus {
    count_total: usize,
    count_loaded: Arc<AtomicUsize>,
    image_infos: Arc<Mutex<serde_json::Map<String, serde_json::Value>>>,
}

impl Status for InfoLoadingStatus {
    fn json(&self) -> serde_json::Value {
        let mut infos = self.image_infos.lock().unwrap();

        let json_value = json!({
            "count_total": self.count_total,
            "count_loaded": self.count_loaded.load(Relaxed),
            "image_infos": infos.clone(),
        });

        infos.clear();
        json_value
    }

    fn aborted(&self) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.count_total == self.count_loaded.load(Relaxed)
    }

    fn start_update_emitter(status: Arc<Mutex<Self>>, callback_event: String, window: Window) {
        let emitter = StatusEmitter::new(status, callback_event.clone(), window);

        thread::spawn(move || {
            emitter.run_update_worker();
        });
    }
}

impl InfoLoadingStatus {
    pub fn new(paths: Vec<String>, callback_event: String, window: Option<Window>) -> Arc<Mutex<Self>> {
        let status = Arc::new(Mutex::new(InfoLoadingStatus {
            count_total: paths.len(),
            count_loaded: Arc::new(AtomicUsize::new(0)),
            image_infos: Arc::new(Mutex::new(Map::new())),
        }));

        if let Some(w) = window {
            Self::start_update_emitter(status.clone(), callback_event, w);
        }
        status
    }

    pub fn add_image_info(&self, image_path: String, info: serde_json::Value) {
        debug!("Successfully loaded info of '{}'", image_path);

        self.count_loaded.fetch_add(1, Relaxed);
        self.image_infos.lock().unwrap().insert(image_path, info);
    }

    pub fn add_loading_error(&self, image_path: String, error: anyhow::Error) {
        warn!("Could not load info of '{}':\n{:?}\n", image_path, error);

        let filename = match PathBuf::from(&image_path).file_name() {
            Some(x) => String::from(x.to_str().unwrap()),
            None => image_path.clone(),
        };

        self.count_loaded.fetch_add(1, Relaxed);
        self.image_infos.lock().unwrap().insert(
            image_path.clone(),
            json!({
                "path": image_path,
                "filename": filename,
                "error": format!("{:?}", error),
            }),
        );
    }
}

pub struct ProcessingStatus {
    pub count_lights: usize,
    pub count_darks: usize,
    aborted_status: Arc<AtomicBool>,
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

    fn aborted(&self) -> bool {
        self.aborted_status.load(Relaxed)
    }

    fn finished(&self) -> bool {
        self.loading_done() && self.merging_done() || self.aborted()
    }

    fn start_update_emitter(status: Arc<Mutex<Self>>, callback_event: String, window: Window) {
        let emitter = StatusEmitter::new(status, callback_event.clone(), window);

        thread::spawn(move || {
            emitter.run_update_worker();
        });
    }
}

impl ProcessingStatus {
    pub fn new(
        count_lights: usize,
        count_darks: usize,
        callback_event: String,
        window: Option<Window>,
    ) -> Arc<Mutex<Self>> {
        let status = Arc::new(Mutex::new(ProcessingStatus {
            count_lights,
            count_darks,
            aborted_status: Arc::new(AtomicBool::new(false)),
            count_loaded_lights: Arc::new(AtomicUsize::new(0)),
            count_loading_lights: Arc::new(AtomicUsize::new(0)),
            count_merge_completed: Arc::new(AtomicUsize::new(0)),
            count_merging: Arc::new(AtomicUsize::new(0)),
        }));

        if let Some(w) = window {
            Self::start_update_emitter(status.clone(), callback_event, w);
        }
        status
    }

    pub fn abort(&self) {
        warn!("Aborting processing");
        self.aborted_status.store(true, Relaxed)
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
        debug!(
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
