mod audio;
mod hotkey;
mod transcribe;
mod typing;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::MacosLauncher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppStatus {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub model_path: String,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model_path: transcribe::default_model_path()
                .to_string_lossy()
                .to_string(),
            language: "pl".to_string(),
        }
    }
}

// Commands sent to the worker thread (owns non-Send types)
enum WorkerCmd {
    Toggle,
    UpdateSettings(Settings),
}

// Tauri-managed state — all fields are Send + Sync
pub struct AppState {
    status: Mutex<AppStatus>,
    settings: Mutex<Settings>,
    cmd_tx: Mutex<mpsc::Sender<WorkerCmd>>,
}

// --- Tauri commands ---

#[tauri::command]
fn get_status(state: tauri::State<'_, AppState>) -> AppStatus {
    *state.status.lock()
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
fn update_settings(state: tauri::State<'_, AppState>, settings: Settings) {
    *state.settings.lock() = settings.clone();
    let _ = state.cmd_tx.lock().send(WorkerCmd::UpdateSettings(settings));
}

#[tauri::command]
fn check_model(state: tauri::State<'_, AppState>) -> bool {
    let path = PathBuf::from(&state.settings.lock().model_path);
    path.exists()
}

#[tauri::command]
fn toggle_recording(state: tauri::State<'_, AppState>) {
    let _ = state.cmd_tx.lock().send(WorkerCmd::Toggle);
}

// --- Worker thread: owns AudioRecorder + Transcriber (non-Send) ---

fn run_worker(rx: mpsc::Receiver<WorkerCmd>, app: AppHandle) {
    let model_path = {
        let st = app.state::<AppState>();
        let path = st.settings.lock().model_path.clone();
        PathBuf::from(path)
    };

    let mut transcriber = if model_path.exists() {
        match transcribe::Transcriber::new(&model_path) {
            Ok(t) => {
                log::info!("Whisper model loaded");
                Some(t)
            }
            Err(e) => {
                log::error!("Failed to load model: {e}");
                None
            }
        }
    } else {
        log::warn!("Model not found: {}", model_path.display());
        None
    };

    let mut recorder: Option<audio::AudioRecorder> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            WorkerCmd::Toggle => {
                let status = *app.state::<AppState>().status.lock();

                match status {
                    AppStatus::Idle => {
                        match audio::AudioRecorder::new() {
                            Ok(mut rec) => {
                                if let Err(e) = rec.start() {
                                    log::error!("Recording start failed: {e}");
                                    let _ = app.emit("error", e.to_string());
                                    continue;
                                }
                                recorder = Some(rec);
                                *app.state::<AppState>().status.lock() = AppStatus::Recording;
                                let _ = app.emit("status-changed", AppStatus::Recording);
                            }
                            Err(e) => {
                                log::error!("Recorder init failed: {e}");
                                let _ = app.emit("error", e.to_string());
                            }
                        }
                    }
                    AppStatus::Recording => {
                        let samples = match recorder.as_mut() {
                            Some(rec) => rec.stop(),
                            None => continue,
                        };
                        recorder = None;

                        *app.state::<AppState>().status.lock() = AppStatus::Transcribing;
                        let _ = app.emit("status-changed", AppStatus::Transcribing);

                        let language = app.state::<AppState>().settings.lock().language.clone();

                        if let Some(ref t) = transcriber {
                            match t.transcribe(&samples, &language) {
                                Ok(text) => {
                                    log::info!("Transcribed: {text}");
                                    if let Err(e) = typing::type_text(&text) {
                                        log::error!("Typing failed: {e}");
                                        let _ = app.emit("error", e.to_string());
                                    }
                                }
                                Err(e) => {
                                    log::error!("Transcription failed: {e}");
                                    let _ = app.emit("error", e.to_string());
                                }
                            }
                        } else {
                            let _ = app.emit("error", "Model not loaded".to_string());
                        }

                        *app.state::<AppState>().status.lock() = AppStatus::Idle;
                        let _ = app.emit("status-changed", AppStatus::Idle);
                    }
                    AppStatus::Transcribing => {} // ignore while transcribing
                }
            }
            WorkerCmd::UpdateSettings(settings) => {
                let new_path = PathBuf::from(&settings.model_path);
                if new_path.exists() {
                    match transcribe::Transcriber::new(&new_path) {
                        Ok(t) => {
                            transcriber = Some(t);
                            log::info!("Model reloaded from {}", new_path.display());
                        }
                        Err(e) => log::error!("Model reload failed: {e}"),
                    }
                }
            }
        }
    }
}

// --- System tray ---

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItemBuilder::with_id("show", "Show / Hide").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

    let icon = Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Voice to Text — Double-press Alt")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

// --- App entry ---

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (cmd_tx, cmd_rx) = mpsc::channel();

    let app_state = AppState {
        status: Mutex::new(AppStatus::Idle),
        settings: Mutex::new(Settings::default()),
        cmd_tx: Mutex::new(cmd_tx.clone()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_settings,
            update_settings,
            check_model,
            toggle_recording,
        ])
        .setup(move |app| {
            setup_tray(app.handle())?;

            // Worker thread — owns AudioRecorder + Transcriber
            let worker_handle = app.handle().clone();
            std::thread::spawn(move || run_worker(cmd_rx, worker_handle));

            // Hotkey listener → forwards double-Alt to worker
            let hotkey_tx = cmd_tx.clone();
            let (htx, hrx) = mpsc::channel();
            hotkey::start_listener(htx);
            std::thread::spawn(move || {
                while let Ok(hotkey::HotkeyEvent::DoubleAlt) = hrx.recv() {
                    let _ = hotkey_tx.send(WorkerCmd::Toggle);
                }
            });

            // Start hidden (tray-only)
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Failed to run application");
}
