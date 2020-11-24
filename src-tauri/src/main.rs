#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod cmd;
mod image;

use serde_json::json;
use chrono::prelude::*;
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
                  let time : chrono::DateTime<Local> = metadata.created()?.into();
                  assert!(metadata.is_file());

                  let candidate = image::ImageCandidate::load(p).unwrap();
                  Ok(candidate.json())
                },
                callback,
                error
              )
            }
          }
          Ok(())
        }
      }
    })
    .build()
    .run();
}
