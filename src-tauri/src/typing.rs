use anyhow::Result;
use std::process::Command;
use std::time::Duration;

/// Type text into the currently focused window using xdotool.
///
/// Uses a per-character delay to prevent X11 from reordering keystrokes.
/// Does not touch the clipboard.
pub fn type_text(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    // Ensure modifier keys are released (double-Alt hotkey might leave state)
    let _ = Command::new("xdotool").args(["keyup", "Alt_L"]).status();
    let _ = Command::new("xdotool").args(["keyup", "Alt_R"]).status();
    let _ = Command::new("xdotool").args(["keyup", "super"]).status();
    std::thread::sleep(Duration::from_millis(50));

    let status = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "12", text])
        .status();
    log::info!("type_text: typed {} chars, exit={status:?}", text.len());

    Ok(())
}
