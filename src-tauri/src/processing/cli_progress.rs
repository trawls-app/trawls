use std::{
    cmp::max,
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use console::{style, Emoji};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;

use super::status::ProcessingStepStatus;

static LIGHT: Emoji<'_, '_> = Emoji("🌕", "");
static DARK: Emoji<'_, '_> = Emoji("🌑", "");
static TRUCK: Emoji<'_, '_> = Emoji("🚚", "");
static SPARKLE: Emoji<'_, '_> = Emoji("✨", ":-)");

pub struct ProcessingStatusCli {
    bars: MultiProgress,
    pb_loading: ProgressBar,
    pb_merging: ProgressBar,
    pb_steps: HashMap<usize, ProgressBar>,
    count_lights: u64,
    count_darks: u64,
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

        ProcessingStatusCli {
            bars,
            pb_loading,
            pb_merging,
            pb_steps: HashMap::new(),
            count_lights,
            count_darks,
        }
    }

    pub fn update(&mut self, loaded: u64, merged: u64, loading: u64, merging: u64, steps: &Vec<ProcessingStepStatus>) {
        if !self.pb_loading.is_finished() {
            self.pb_loading.set_position(loaded);
            self.pb_loading.set_message(format!("{} Loading {}", TRUCK, loading));
        }

        if !self.pb_merging.is_finished() {
            self.pb_merging.set_position(merged);
            self.pb_merging.set_message(format!("{} Merging {}", SPARKLE, merging));
        }

        for (index, step) in steps.iter().enumerate() {
            match self.pb_steps.get(&index) {
                Some(pb) => {
                    self.update_step(step, pb);
                }
                None => {
                    let pb = self.start_step(step);
                    self.pb_steps.insert(index, pb);
                }
            }
        }
    }

    fn start_step(&mut self, step: &ProcessingStepStatus) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::with_template("{msg} {elapsed}").unwrap());
        pb.set_message(format!("{} {}", step.emoji, style(step.str.clone()).bold()));
        pb.enable_steady_tick(Duration::from_secs_f32(0.1));

        //self.bars.add(pb)
        pb
    }

    fn update_step(&self, step: &ProcessingStepStatus, pb: &ProgressBar) {
        if step.is_finished() && !pb.is_finished() {
            pb.finish_and_clear();
            self.bars
                .println(format!(
                    "{} {} {}",
                    step.emoji,
                    step.str,
                    style(format!("{:.2}s", step.runtime().as_secs_f32())).dim()
                ))
                .unwrap();
        }
    }

    pub fn finish_loading(&self) {
        if self.pb_loading.is_finished() {
            return;
        }

        self.pb_loading.finish_and_clear();
        self.bars
            .println(format!("{} Loaded {} lightframes\n{} and {} darkframes", LIGHT, self.count_lights, DARK, self.count_darks))
            .unwrap();
    }

    pub fn finish_merging(&self) {
        if self.pb_merging.is_finished() {
            return;
        }

        self.pb_merging.finish_and_clear();
        println!("{} Merged {} images in total", SPARKLE, self.count_lights + self.count_darks - 1);
    }
}
