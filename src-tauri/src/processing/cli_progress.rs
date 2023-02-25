use std::cmp::max;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct ProcessingStatusCli {
    pb_loading: ProgressBar,
    pb_merging: ProgressBar,
}

impl ProcessingStatusCli {
    pub fn new(count_lights: u64, count_darks: u64) -> ProcessingStatusCli {
        let bars = MultiProgress::new();
        let style = ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7} {msg} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-");

        let pb_loading = bars.add(ProgressBar::new(count_lights + count_darks));
        pb_loading.set_style(style.clone());

        let count_merge_tasks = max(count_lights as i64 - 1, 0) as u64 + max(count_darks as i64 - 1, 0) as u64;
        let pb_merging = bars.insert_after(&pb_loading, ProgressBar::new(count_merge_tasks));
        pb_merging.set_style(style);

        ProcessingStatusCli { pb_loading, pb_merging }
    }

    pub fn update(&self, loaded: u64, merged: u64, loading: u64, merging: u64) {
        self.pb_loading.set_position(loaded);
        self.pb_merging.set_position(merged);

        self.pb_loading.set_message(format!("Loading {}", loading));
        self.pb_merging.set_message(format!("Merging {}", merging));
    }

    pub fn finish_loading(&self) {
        self.pb_loading.finish();
    }

    pub fn finish_merging(&self) {
        self.pb_merging.finish();
    }
}
