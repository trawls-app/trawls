use crate::fileinfo;
use crate::fileinfo::ImageCandidate;
use crate::processing;
use crate::processing::status::{InfoLoadingStatus, ProcessingStatus};
use crate::processing::RenderedPreview;
use log::{error, info};

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;
use serde_json::json;

#[tauri::command]
pub fn get_app_version() -> String {
    version!().to_string()
}

#[tauri::command]
pub async fn load_image_infos(
    window: tauri::Window,
    paths: Vec<String>,
    selector_reference: String,
) -> Result<serde_json::Value, String> {
    let reference = format!("loaded_image_info_{}", selector_reference);
    let state = InfoLoadingStatus::new(paths.clone(), reference, Some(window));

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
pub async fn run_merge(
    window: tauri::Window,
    lightframes: Vec<String>,
    darkframes: Vec<String>,
    mode_str: String,
    out_path: String,
) -> Result<serde_json::Value, serde_json::Value> {
    let state = ProcessingStatus::new(
        lightframes.len(),
        darkframes.len(),
        String::from("processing_state_change"),
        Some(window),
    );

    let paths_light = lightframes.into_iter().map(|x| Path::new(&x).to_path_buf()).collect();
    let paths_dark = darkframes.into_iter().map(|x| Path::new(&x).to_path_buf()).collect();

    let mode = match mode_str.as_str() {
        "falling" => processing::Comets::Falling,
        "raising" => processing::Comets::Raising,
        _ => processing::Comets::Normal,
    };
    info!("Running merge in '{}' mode", mode_str);

    let start = Instant::now();
    let image = processing::run_merge(paths_light, paths_dark, mode, state).anyhow_to_json()?;

    let exif = image.exif.clone();
    let writer = image.get_image_writer().anyhow_to_json()?;
    writer.write_dng(PathBuf::from(out_path)).anyhow_to_json()?;

    // Render a preview to show in the UI
    let preview_bytes = writer.get_preview_bytes().anyhow_to_json()?;

    info!("Processing took {} seconds", start.elapsed().as_secs());

    Ok(json!(RenderedPreview::new(preview_bytes, exif)))
}

fn fetch_exif(path: PathBuf) -> anyhow::Result<ImageCandidate> {
    let metadata = fs::metadata(&path)?;
    anyhow::ensure!(metadata.is_file());

    let candidate = fileinfo::ImageCandidate::load(&path)?;
    Ok(candidate)
}

trait JsonError<T> {
    fn anyhow_to_json(self) -> Result<T, serde_json::Value>;
}

impl<T> JsonError<T> for anyhow::Result<T> {
    fn anyhow_to_json(self) -> Result<T, serde_json::Value> {
        match self {
            Ok(x) => Ok(x),
            Err(err) => {
                error!("Error during processing: {:#?}", err);
                Err(json!({ "message": err.to_string(), "trace": format!("{:#?}", err)}))
            }
        }
    }
}
