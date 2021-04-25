use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Instant;
use serde_json::json;
use tauri::event::emit;
use tauri::WebviewMut;


#[derive(Clone)]
pub struct Status {
    count_lights: Arc<AtomicUsize>,
    count_loaded_lights: Arc<AtomicUsize>,
    count_loading_lights: Arc<AtomicUsize>,
    count_merge_completed: Arc<AtomicUsize>,
    count_merging: Arc<AtomicUsize>,
    last_update: Arc<Mutex<Instant>>,
    webview: WebviewMut
}

impl Status {
    pub fn new(num_lights: usize, webview: WebviewMut) -> Status {
        Status {
            count_lights: Arc::new(AtomicUsize::new(num_lights)),
            count_loaded_lights: Arc::new(AtomicUsize::new(0)),
            count_loading_lights: Arc::new(AtomicUsize::new(0)),
            count_merge_completed: Arc::new(AtomicUsize::new(0)),
            count_merging: Arc::new(AtomicUsize::new(0)),
            last_update: Arc::new(Mutex::new(Instant::now())),
            webview
        }
    }

    pub fn update_status(&self, force: bool) {
        println!("Total {}, Loaded {}, Loading {}, Merged {}, Merging {}, loading_done = {}, merging_done = {}",
                 self.count_lights.load(Relaxed),
                 self.count_loaded_lights.load(Relaxed),
                 self.count_loading_lights.load(Relaxed),
                 self.count_merge_completed.load(Relaxed),
                 self.count_merging.load(Relaxed),
                 self.loading_done(),
                 self.merging_done()
        );

        let mut lu = self.last_update.lock().unwrap();
        if force || (*lu).elapsed().as_millis() > 100 {
            *lu = Instant::now();
            emit(&mut self.webview.clone(), "state_change", Some(self.json())).expect("Could not emit status update");
        }
    }

    pub fn loading_done(&self) -> bool {
        self.count_lights.load(Relaxed) == self.count_loaded_lights.load(Relaxed)
    }

    pub fn merging_done(&self) -> bool {
        self.count_lights.load(Relaxed) - 1 == self.count_merge_completed.load(Relaxed)
    }

    pub fn start_loading(&self) {
        self.count_loading_lights.fetch_add(1, Relaxed);
        self.update_status(false);
    }

    pub fn finish_loading(&self) {
        self.count_loaded_lights.fetch_add(1, Relaxed);
        self.count_loading_lights.fetch_sub(1, Relaxed);
        self.update_status(false);
    }

    pub fn start_merging(&self) {
        self.count_merging.fetch_add(1, Relaxed);
        self.update_status(false);
    }

    pub fn finish_merging(&self) {
        self.count_merge_completed.fetch_add(1, Relaxed);
        self.count_merging.fetch_sub(1, Relaxed);
        self.update_status(false);
    }

    pub fn json(&self) -> serde_json::Value {
        json!({
            "count_lights": self.count_lights.load(Relaxed),
            "count_loaded_lights": self.count_loaded_lights.load(Relaxed),
            "count_loading_lights": self.count_loading_lights.load(Relaxed),
            "count_merged": self.count_merge_completed.load(Relaxed),
            "count_merging": self.count_merging.load(Relaxed),
            "loading_done": self.loading_done(),
            "merging_done": self.merging_done()
        })
    }
}