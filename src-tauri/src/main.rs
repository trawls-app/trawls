#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
#![allow(non_upper_case_globals)]

#[macro_use]
extern crate version;
extern crate anyhow;

mod fileinfo;
mod processing;

use crate::processing::status::ProcessingStatus;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use fileinfo::ImageCandidate;
use rayon::prelude::*;

#[tauri::command]
fn get_app_version() -> String {
    version!().to_string()
}

fn fetch_exif(path: PathBuf) -> anyhow::Result<ImageCandidate> {
    let metadata = fs::metadata(&path)?;
    anyhow::ensure!(metadata.is_file());

    // Todo: Refactor to remove unwrap
    let candidate = fileinfo::ImageCandidate::load(&path).unwrap();
    Ok(candidate)
}

#[tauri::command]
async fn load_image_infos(
    window: tauri::Window,
    paths: Vec<String>,
    selector_reference: String,
) -> Result<serde_json::Value, String> {
    println!("Loading exifs");
    let window = Mutex::new(window);

    paths.par_iter().for_each(|x| {
        let p = Path::new(x).to_path_buf();
        let candidate = fetch_exif(p);

        match candidate {
            Ok(c) => {
                let reference = format!("loaded_image_info_{}", selector_reference);
                window.lock().unwrap().emit(reference.as_str(), c.json()).unwrap();
                println!("{}", x);
            }
            Err(_err) => todo!(),
        }
    });

    println!("Finished loading exifs");

    Ok(serde_json::json!({}))
}

#[tauri::command]
async fn run_merge(
    window: tauri::Window,
    lightframes: Vec<String>,
    darkframes: Vec<String>,
    mode_str: String,
    out_path: String,
) -> Result<serde_json::Value, String> {
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
    println!("Running merge in '{}' mode.", mode_str);

    let start = Instant::now();
    let preview = processing::run_merge(paths_light, paths_dark, output, mode, state);

    println!("Processing took {} seconds.", start.elapsed().as_secs());
    Ok(serde_json::json!(preview))
}

fn main() {
    println!("Running Trawls v{}", version!());
    tauri::Builder::new()
        .invoke_handler(tauri::generate_handler![get_app_version, load_image_infos, run_merge])
        .run(tauri::generate_context!())
        .unwrap();
}
