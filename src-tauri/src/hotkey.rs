use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum HotkeyEvent {
    DoubleAlt,
}

const DOUBLE_PRESS_WINDOW: Duration = Duration::from_millis(400);

pub fn start_listener(tx: mpsc::Sender<HotkeyEvent>) {
    thread::spawn(move || {
        let mut last_release = Instant::now() - Duration::from_secs(60);
        let mut armed = false;

        if let Err(e) = rdev::listen(move |event: rdev::Event| {
            match event.event_type {
                rdev::EventType::KeyRelease(rdev::Key::Alt) => {
                    let now = Instant::now();
                    if armed && now.duration_since(last_release) < DOUBLE_PRESS_WINDOW {
                        let _ = tx.send(HotkeyEvent::DoubleAlt);
                        armed = false;
                    } else {
                        armed = true;
                    }
                    last_release = now;
                }
                // Any non-Alt keypress disarms
                rdev::EventType::KeyPress(key)
                    if !matches!(key, rdev::Key::Alt | rdev::Key::AltGr) =>
                {
                    armed = false;
                }
                _ => {}
            }
        }) {
            log::error!("Hotkey listener failed: {e:?}");
        }
    });
}
