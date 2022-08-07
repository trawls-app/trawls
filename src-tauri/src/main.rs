#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
#![allow(non_upper_case_globals)]

extern crate pretty_env_logger;

#[macro_use]
extern crate version;
extern crate anyhow;
extern crate log;

mod fileinfo;
mod processing;

use crate::processing::status::{InfoLoadingStatus, ProcessingStatus};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use fileinfo::ImageCandidate;
use log::{error, info};
use rayon::prelude::*;
use serde_json::json;

#[tauri::command]
fn get_app_version() -> String {
    version!().to_string()
}

fn fetch_exif(path: PathBuf) -> anyhow::Result<ImageCandidate> {
    let metadata = fs::metadata(&path)?;
    anyhow::ensure!(metadata.is_file());

    let candidate = fileinfo::ImageCandidate::load(&path)?;
    Ok(candidate)
}

#[tauri::command]
async fn load_image_infos(
    window: tauri::Window,
    paths: Vec<String>,
    selector_reference: String,
) -> Result<serde_json::Value, String> {
    let reference = format!("loaded_image_info_{}", selector_reference);
    let state = InfoLoadingStatus::new(paths.clone(), reference, window);

    info!("Start loading exifs of {} files", paths.len());
    paths.par_iter().for_each(|x| {
        let p = PathBuf::from(x);
        let candidate = fetch_exif(p);

        match candidate {
            Ok(c) => {
                state.lock().unwrap().add_image_info(x.to_string(), c.json());
            }
            Err(error) => {
                state.lock().unwrap().add_loading_error(x.to_string(), error);
            }
        }
    });

    info!("Finished loading exifs");

    Ok(serde_json::json!({}))
}

#[tauri::command]
async fn run_merge(
    window: tauri::Window,
    lightframes: Vec<String>,
    darkframes: Vec<String>,
    mode_str: String,
    out_path: String,
) -> Result<serde_json::Value, serde_json::Value> {
    let state =
        ProcessingStatus::new(lightframes.len(), darkframes.len(), String::from("processing_state_change"), window);

    let paths_light = lightframes.into_iter().map(|x| Path::new(&x).to_path_buf()).collect();
    let paths_dark = darkframes.into_iter().map(|x| Path::new(&x).to_path_buf()).collect();

    let output = Path::new(&out_path).to_path_buf();
    let mode = match mode_str.as_str() {
        "falling" => processing::Comets::Falling,
        "raising" => processing::Comets::Raising,
        _ => processing::Comets::Normal,
    };
    info!("Running merge in '{}' mode", mode_str);

    let start = Instant::now();
    let preview = processing::run_merge(paths_light, paths_dark, output, mode, state);

    info!("Processing took {} seconds", start.elapsed().as_secs());

    match preview {
        Ok(x) => Ok(json!(x)),
        Err(err) => {
            error!("Merging failed\n\n-----------------\n{:?}\n-----------------\n", err);
            Err(json!({ "message": err.to_string(), "trace": format!("{:?}", err) }))
        }
    }
}

fn main() {
    pretty_env_logger::init();
    info!("Running Trawls v{}", version!());

    tauri::Builder::new()
        .invoke_handler(tauri::generate_handler![get_app_version, load_image_infos, run_merge])
        .run(tauri::generate_context!())
        .unwrap();
}
