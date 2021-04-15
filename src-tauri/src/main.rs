#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod cmd;
mod fileinfo;
mod processing;

use crate::processing::status::Status;
use std::path::Path;
use std::fs;


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

            LoadImage { path, callback, error} => {
              tauri::execute_promise(
                _webview,
                move || {
                  let p = Path::new(&path);
                  let metadata = fs::metadata(p)?;
                  assert!(metadata.is_file());

                  let candidate = fileinfo::ImageCandidate::load(p).unwrap();
                  Ok(candidate.json())
                }, callback, error)
            },

            RunMerge { lightframes, mode_str, out_path, callback, error} => {
              let state = Status::new(lightframes.len(), _webview.as_mut());
              let paths = lightframes.into_iter().map(|x| Path::new(&x).to_path_buf()).collect();
              let output = Path::new(&out_path).to_path_buf();
              let mode = match mode_str.as_str() {
                "falling" => processing::CometMode::Falling,
                "raising" => processing::CometMode::Raising,
                _ => processing::CometMode::Normal
              };
              println!("Selected '{}' mode", mode_str);

              tauri::execute_promise(
                _webview,
                move || {
                  processing::run_merge(paths, output, mode, state);
                  Ok(())
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
