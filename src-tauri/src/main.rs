#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#![allow(non_upper_case_globals)]

mod cmd;
mod fileinfo;
mod processing;
mod errors;

use crate::processing::status::Status;
use std::path::Path;
use std::fs;
use std::time::Instant;

use rayon::prelude::*;


fn load_images(paths: Vec<String>) -> Result<serde_json::Value, String> {
  let res: Vec<serde_json::Value> = paths.par_iter().map(|x| {
    let p = Path::new(x);
    let metadata = fs::metadata(p).unwrap();
    assert!(metadata.is_file());

    let candidate = fileinfo::ImageCandidate::load(p).unwrap();
    candidate.json()
  }).collect();

  Ok(serde_json::json!(res))
}


fn main() {
  tauri::AppBuilder::new()
    .invoke_handler(|_webview, arg| {
      use cmd::Cmd::*;
      match serde_json::from_str(arg) {
        Err(e) => {
          Err(e.to_string())
        }
        Ok(command) => {
          match command {

            LoadImages { paths, callback, error} => {
              tauri::execute_promise(_webview, move || {
                let res = load_images(paths).unwrap();
                Ok(res) }, callback, error)
            },

            RunMerge { lightframes, darkframes, mode_str, out_path, callback, error} => {
              let state = Status::new(lightframes.len(), darkframes.len(), _webview.as_mut());
              let paths_light = lightframes.into_iter().map(|x| Path::new(&x).to_path_buf()).collect();
              let paths_dark = darkframes.into_iter().map(|x| Path::new(&x).to_path_buf()).collect();
              let output = Path::new(&out_path).to_path_buf();
              let mode = match mode_str.as_str() {
                "falling" => processing::Comets::Falling,
                "raising" => processing::Comets::Raising,
                _ => processing::Comets::Normal
              };
              println!("Selected '{}' mode", mode_str);

              tauri::execute_promise(
                _webview,
                move || {
                  let start = Instant::now();
                  let preview = processing::run_merge(paths_light, paths_dark, output, mode, state);

                  println!("Processing took {} seconds.", start.elapsed().as_secs());
                  preview
                }, callback, error)
            }
          }
          Ok(())
        }
      }
    })
    .build()
    .run();
}
