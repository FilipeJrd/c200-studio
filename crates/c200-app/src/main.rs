mod controller;
mod interaction;
mod ui;
mod widgets;

use c200_core::{CameraBackend, Error, Result, profiles};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Args {
    demo: bool,
    diagnose: bool,
    probe_preview: bool,
    smoke_test: bool,
    screenshot: Option<PathBuf>,
    config: Option<PathBuf>,
}

fn args() -> Result<Args> {
    let mut args = Args::default();
    let mut input = std::env::args().skip(1);
    while let Some(arg) = input.next() {
        match arg.as_str() {
            "--demo" => args.demo = true,
            "--diagnose" => args.diagnose = true,
            "--probe-preview" => args.probe_preview = true,
            "--smoke-test" => args.smoke_test = true,
            "--config" | "--screenshot" => {
                let value = input
                    .next()
                    .ok_or_else(|| Error::Message(format!("Missing path after {arg}")))?;
                if arg == "--config" {
                    args.config = Some(value.into());
                } else {
                    args.screenshot = Some(value.into());
                }
            }
            "--help" | "-h" => {
                println!(
                    "C200 Studio\n\nUsage: c200-studio [options]\n  --demo                 Simulated camera; no hardware access\n  --diagnose             JSON diagnostics; read-only camera controls\n  --probe-preview        Capture 10 frames to verify preview (no images saved)\n  --config PATH          Use an alternate configuration file\n  --smoke-test           Close automatically after 4 seconds\n  --screenshot PATH      Save a window screenshot and exit\n  --help                 Show this help"
                );
                std::process::exit(0);
            }
            _ => {
                return Err(Error::Message(format!(
                    "Unknown option: {arg}. Use --help."
                )));
            }
        }
    }
    Ok(args)
}

fn diagnose(demo: bool) -> Result<()> {
    let mut backend: Box<dyn CameraBackend> = if demo {
        Box::<c200_core::mock::MockCamera>::default()
    } else {
        Box::<c200_linux::LinuxCamera>::default()
    };
    let mut output = Vec::new();
    for device in backend.devices()? {
        let item = match backend.connect(&device) {
            Ok(()) => match backend.controls() {
                Ok(controls) => {
                    serde_json::json!({"device": device, "controls": controls.iter().map(|c| serde_json::json!({"key":c.key,"value":c.value,"min":c.min,"max":c.max,"step":c.step,"available":c.available,"writable":c.writable,"inactive":c.inactive,"reason":c.reason})).collect::<Vec<_>>(), "modes": backend.modes().unwrap_or_default()})
                }
                Err(error) => serde_json::json!({"device": device,"error": error.to_string()}),
            },
            Err(error) => serde_json::json!({"device": device, "error": error.to_string()}),
        };
        backend.disconnect();
        output.push(item);
    }
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn probe(demo: bool) -> Result<()> {
    let mut backend: Box<dyn CameraBackend> = if demo {
        Box::<c200_core::mock::MockCamera>::default()
    } else {
        Box::<c200_linux::LinuxCamera>::default()
    };
    let devices = backend.devices()?;
    if devices.len() != 1 {
        return Err(Error::Message(
            "Connect exactly one C200 to run the preview probe".into(),
        ));
    }
    backend.connect(&devices[0])?;
    let mode = c200_core::preferred_mode(&backend.modes()?)
        .ok_or_else(|| Error::Message("No compatible capture mode".into()))?;
    let mut preview = c200_linux::preview::Preview::default();
    let (tx, rx) = std::sync::mpsc::channel();
    preview.start(0, devices[0].clone(), mode, tx);
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut count = 0;
    while Instant::now() < deadline && count < 10 {
        if let Ok(event) = rx.try_recv() {
            match event {
                c200_linux::preview::PreviewEvent::Failed(_, error) => {
                    return Err(Error::Message(error));
                }
                c200_linux::preview::PreviewEvent::Started(_, mode) => {
                    println!("Mode: {}", mode.label())
                }
            }
        }
        if let Some(frame) = preview
            .frames
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            count += 1;
            if count == 1 {
                println!(
                    "RGB frame: {} × {} · {} bytes",
                    frame.width,
                    frame.height,
                    frame.rgb.len()
                );
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    preview.stop();
    if count < 10 {
        return Err(Error::Message(format!("Received only {count}/10 frames")));
    }
    println!("Preview verified: {count} frames; capture stopped.");
    Ok(())
}

fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = args()?;
    if args.diagnose {
        return Ok(diagnose(args.demo)?);
    }
    if args.probe_preview {
        return Ok(probe(args.demo)?);
    }
    let config = args.config.clone().unwrap_or(if args.demo {
        std::env::temp_dir().join(format!(
            "c200-studio-demo-{}/settings.json",
            std::process::id()
        ))
    } else {
        profiles::config_path()?
    });
    if let Some(path) = &args.screenshot
        && let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/c200-studio.png"))?;
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([950.0, 620.0])
            .with_app_id("io.github.c200studio.C200Studio")
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "C200 Studio",
        options,
        Box::new(move |cc| Ok(Box::new(ui::Studio::new(cc, args, config)))),
    )?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("C200 Studio: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
