#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![allow(non_upper_case_globals)]

#[macro_use]
extern crate version;

mod fileinfo;
mod processing;

use crate::processing::status::Status;
use std::fs;
use std::path::Path;
use std::time::Instant;

use rayon::prelude::*;

#[tauri::command]
fn get_app_version() -> String {
    version!().to_string()
}

#[tauri::command]
async fn load_images(paths: Vec<String>) -> Result<serde_json::Value, String> {
    println!("Loading exifs");
    let res: Vec<serde_json::Value> = paths
        .par_iter()
        .map(|x| {
            let p = Path::new(x);
            let metadata = fs::metadata(p).unwrap();
            assert!(metadata.is_file());

            let candidate = fileinfo::ImageCandidate::load(p).unwrap();
            candidate.json()
        })
        .collect();

    Ok(serde_json::json!(res))
}

#[tauri::command]
async fn run_merge(
    window: tauri::Window,
    lightframes: Vec<String>,
    darkframes: Vec<String>,
    mode_str: String,
    out_path: String,
) -> Result<serde_json::Value, String> {
    let state = Status::new(lightframes.len(), darkframes.len(), window);
    let paths_light = lightframes
        .into_iter()
        .map(|x| Path::new(&x).to_path_buf())
        .collect();
    let paths_dark = darkframes
        .into_iter()
        .map(|x| Path::new(&x).to_path_buf())
        .collect();
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
        .invoke_handler(tauri::generate_handler![get_app_version, load_images, run_merge])
        .run(tauri::generate_context!())
        .unwrap();
}
