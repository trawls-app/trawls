use log::{debug, warn};
use serde_json::{json, Map};
use std::cmp::max;
use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::Window;

use crate::processing::cli_progress::ProcessingStatusCli;

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

pub struct ProcessingStepStatus {
    pub str: String,
    pub emoji: char,
    started_at: Instant,
    ended_at: Option<Instant>,
}

impl ProcessingStepStatus {
    pub fn new(str: String, emoji: char) -> ProcessingStepStatus {
        ProcessingStepStatus {
            str,
            emoji,
            started_at: Instant::now(),
            ended_at: None,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.ended_at.is_some()
    }

    fn finish(&mut self) {
        self.ended_at = Some(Instant::now())
    }

    pub fn runtime(&self) -> Duration {
        match self.ended_at {
            Some(x) => x - self.started_at,
            None => Instant::now() - self.started_at,
        }
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
    steps: Arc<Mutex<Vec<ProcessingStepStatus>>>,
    cli_progress: ProcessingStatusCli,
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
            steps: Arc::new(Mutex::new(vec![])),
            cli_progress: ProcessingStatusCli::new(count_lights as u64, count_darks as u64),
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

    pub fn start_loading(&mut self) {
        self.count_loading_lights.fetch_add(1, Relaxed);
        self.print_status();
    }

    pub fn finish_loading(&mut self) {
        self.count_loaded_lights.fetch_add(1, Relaxed);
        self.count_loading_lights.fetch_sub(1, Relaxed);
        self.print_status();
    }

    pub fn start_merging(&mut self) {
        self.count_merging.fetch_add(1, Relaxed);
        self.print_status();
    }

    pub fn finish_merging(&mut self) {
        self.count_merge_completed.fetch_add(1, Relaxed);
        self.count_merging.fetch_sub(1, Relaxed);
        self.print_status();
    }

    pub fn start_step(&mut self, str: String, emoji: char) -> usize {
        let mut steps = self.steps.lock().unwrap();
        steps.push(ProcessingStepStatus::new(str, emoji));

        let index = steps.len() - 1;
        drop(steps);

        self.print_status();
        index
    }

    pub fn finish_step(&mut self, step: usize) {
        let mut steps = self.steps.lock().unwrap();

        steps[step].finish();
        drop(steps);

        self.print_status();
    }

    fn print_status(&mut self) {
        self.cli_progress.update(
            self.count_loaded_lights.load(Relaxed) as u64,
            self.count_merge_completed.load(Relaxed) as u64,
            self.count_loading_lights.load(Relaxed) as u64,
            self.count_merging.load(Relaxed) as u64,
            self.steps.lock().unwrap().as_ref(),
        );

        if self.loading_done() {
            self.cli_progress.finish_loading();
        }

        if self.merging_done() {
            self.cli_progress.finish_merging();
        }
    }
}
