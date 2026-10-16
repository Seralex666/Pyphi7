#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod codegen;
mod editing;
mod libs;
mod model;
mod pyhl;
mod settings;
mod state;
mod theme;

use std::path::{Path, PathBuf};

/// Unsichtbarer Kommandozeilenmodus (fuer Tests):  pyphi7 --generate projekt.pyphi tk,qt
fn cli_generate(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("Aufruf: pyphi7 --generate <projekt.pyphi> [tk,qt]");
        return 2;
    }
    let path = Path::new(&args[0]);
    let proj = match model::load(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Fehler: {}", e);
            return 1;
        }
    };
    let tks: Vec<codegen::Toolkit> = args[1]
        .split(',')
        .filter_map(|s| match s {
            "tk" => Some(codegen::Toolkit::Tk),
            "qt" => Some(codegen::Toolkit::Qt),
            _ => None,
        })
        .collect();
    let folder = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let folder = if folder.as_os_str().is_empty() { Path::new(".").to_path_buf() } else { folder };
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("projekt");
    match codegen::generate(&proj, &folder, stem, &tks) {
        Ok((files, added)) => {
            println!("erzeugt: {:?} neue Funktionen: {:?}", files, added);
            0
        }
        Err(e) => {
            eprintln!("Fehler: {}", e);
            1
        }
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("--generate") {
        std::process::exit(cli_generate(&args[1..]));
    }
    let open: Option<PathBuf> = args.first().map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Pyphi7")
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([900.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Pyphi7",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, open.clone())))),
    )
}
