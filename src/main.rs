#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
#![allow(non_upper_case_globals)]

extern crate pretty_env_logger;

#[macro_use]
extern crate version;
extern crate anyhow;
extern crate log;

mod fileinfo;
mod frontend;
mod processing;

use crate::processing::status::ProcessingStatus;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use log::info;
use processing::Comets;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs the merge on the CLI
    Merge(Merge),
}

#[derive(Args)]
struct Merge {
    /// RAW input files to merge
    files: Vec<PathBuf>,

    /// Save the resulting DNG file in this path
    #[arg(short, long)]
    out: PathBuf,

    /// Save the preview JPEG of the result in this path
    #[arg(short, long)]
    preview: Option<PathBuf>,

    /// The mode for merging
    #[arg(short, long)]
    mode: Comets,
}

fn program_description() -> String {
    format!("{} v{}", env!("CARGO_PKG_NAME"), version!())
}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    let cli = Cli::parse();

    info!("{}", program_description());

    match &cli.command {
        Some(Commands::Merge(cmd)) => {
            let state = ProcessingStatus::new(cmd.files.len(), 0, String::from("processing_state_change"), None);
            let image = processing::run_merge(cmd.files.clone(), vec![], cmd.mode, state)?;

            let writer = image.get_image_writer()?;
            writer.write_dng(cmd.out.clone())?;

            if let Some(x) = &cmd.preview {
                writer.write_preview_jpg(x.to_path_buf())?;
            }
        }
        None => {
            info!("Called without parameters. Starting GUI");
            tauri::Builder::new()
                .invoke_handler(tauri::generate_handler![
                    frontend::get_app_version,
                    frontend::load_image_infos,
                    frontend::run_merge
                ])
                .run(tauri::generate_context!())?;
        }
    }

    Ok(())
}
