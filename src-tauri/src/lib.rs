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

pub struct AppState {
    status: Mutex<AppStatus>,
    recorder: Mutex<Option<audio::AudioRecorder>>,
    transcriber: Mutex<Option<transcribe::Transcriber>>,
    settings: Mutex<Settings>,
}

#[tauri::command]
fn get_status(state: tauri::State<'_, AppState>) -> AppStatus {
    *state.status.lock()
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
fn update_settings(state: tauri::State<'_, AppState>, settings: Settings) -> Result<(), String> {
    // Reload model if path changed
    let old_path = state.settings.lock().model_path.clone();
    if old_path != settings.model_path {
        let path = PathBuf::from(&settings.model_path);
        let t = transcribe::Transcriber::new(&path).map_err(|e| e.to_string())?;
        *state.transcriber.lock() = Some(t);
    }
    *state.settings.lock() = settings;
    Ok(())
}

#[tauri::command]
fn check_model(state: tauri::State<'_, AppState>) -> bool {
    let path = PathBuf::from(&state.settings.lock().model_path);
    path.exists()
}

#[tauri::command]
fn toggle_recording(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    do_toggle(&app, &state)
}

fn do_toggle(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let current = *state.status.lock();
    match current {
        AppStatus::Idle => start_recording(app, state),
        AppStatus::Recording => stop_and_transcribe(app, state),
        AppStatus::Transcribing => Ok(()), // ignore during transcription
    }
}

fn start_recording(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let mut recorder = audio::AudioRecorder::new().map_err(|e| e.to_string())?;
    recorder.start().map_err(|e| e.to_string())?;
    *state.recorder.lock() = Some(recorder);
    *state.status.lock() = AppStatus::Recording;
    let _ = app.emit("status-changed", AppStatus::Recording);
    Ok(())
}

fn stop_and_transcribe(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let audio = {
        let mut rec_lock = state.recorder.lock();
        match rec_lock.as_mut() {
            Some(rec) => rec.stop(),
            None => return Err("No active recording".into()),
        }
    };
    *state.recorder.lock() = None;
    *state.status.lock() = AppStatus::Transcribing;
    let _ = app.emit("status-changed", AppStatus::Transcribing);

    let language = state.settings.lock().language.clone();
    let app_handle = app.clone();

    // Transcribe in background thread
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let result = {
            let lock = state.transcriber.lock();
            match lock.as_ref() {
                Some(t) => t.transcribe(&audio, &language),
                None => Err(anyhow::anyhow!("Model not loaded")),
            }
        };

        match result {
            Ok(text) => {
                log::info!("Transcribed: {text}");
                if let Err(e) = typing::type_text(&text) {
                    log::error!("Failed to type text: {e}");
                    let _ = app_handle.emit("error", format!("Typing failed: {e}"));
                }
            }
            Err(e) => {
                log::error!("Transcription failed: {e}");
                let _ = app_handle.emit("error", format!("Transcription failed: {e}"));
            }
        }

        *state.status.lock() = AppStatus::Idle;
        let _ = app_handle.emit("status-changed", AppStatus::Idle);
    });

    Ok(())
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItemBuilder::with_id("show", "Show / Hide").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

    let icon_bytes = include_bytes!("../icons/icon.png");
    let icon = Image::from_bytes(icon_bytes)?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Voice to Text - Double-press Alt to record")
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
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let settings = Settings::default();
    let model_path = PathBuf::from(&settings.model_path);

    let transcriber = if model_path.exists() {
        match transcribe::Transcriber::new(&model_path) {
            Ok(t) => {
                log::info!("Model loaded successfully");
                Some(t)
            }
            Err(e) => {
                log::error!("Failed to load model: {e}");
                None
            }
        }
    } else {
        log::warn!("Model not found at {}", model_path.display());
        None
    };

    let app_state = AppState {
        status: Mutex::new(AppStatus::Idle),
        recorder: Mutex::new(None),
        transcriber: Mutex::new(transcriber),
        settings: Mutex::new(settings),
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
        .setup(|app| {
            setup_tray(app.handle())?;

            // Start hotkey listener
            let (tx, rx) = mpsc::channel();
            hotkey::start_listener(tx);

            let handle = app.handle().clone();
            std::thread::spawn(move || {
                while let Ok(event) = rx.recv() {
                    match event {
                        hotkey::HotkeyEvent::DoubleAlt => {
                            let state = handle.state::<AppState>();
                            if let Err(e) = do_toggle(&handle, &state) {
                                log::error!("Toggle failed: {e}");
                            }
                        }
                    }
                }
            });

            // Hide window on start (tray-only mode)
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Failed to run application");
}
