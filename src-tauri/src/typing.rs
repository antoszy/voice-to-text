use anyhow::{Context, Result};
use std::process::Command;

/// Type text into the currently focused input field using clipboard + xdotool.
pub fn type_text(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    // Debug: log which window has focus
    let focus_before = Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output();
    if let Ok(out) = &focus_before {
        let name = String::from_utf8_lossy(&out.stdout);
        log::info!("type_text: active window = {name:?}");
    }

    // Ensure Alt is released before anything (double-Alt might leave state)
    let _ = Command::new("xdotool").args(["keyup", "Alt_L"]).status();
    let _ = Command::new("xdotool").args(["keyup", "Alt_R"]).status();
    let _ = Command::new("xdotool").args(["keyup", "super"]).status();
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Set clipboard using xsel (more reliable in pipes than xclip)
    let mut child = Command::new("xsel")
        .args(["--clipboard", "--input"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .or_else(|_| {
            // Fallback to xclip
            Command::new("xclip")
                .args(["-selection", "clipboard", "-i"])
                .stdin(std::process::Stdio::piped())
                .spawn()
        })
        .context("Failed to spawn xsel/xclip")?;

    {
        use std::io::Write;
        let mut stdin = child.stdin.take().context("No stdin")?;
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;

    // Verify clipboard was set
    let verify = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output();
    if let Ok(out) = &verify {
        let content = String::from_utf8_lossy(&out.stdout);
        log::info!("type_text: clipboard = {content:?}");
    }

    std::thread::sleep(std::time::Duration::from_millis(150));

    // Paste via Ctrl+Shift+V (works in more terminals) then fallback to Ctrl+V
    let active = Command::new("xdotool")
        .args(["getactivewindow"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    log::info!("type_text: pasting to window id {active}");

    // Try xdotool type first (most direct method)
    let status = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "0", text])
        .status();
    log::info!("type_text: xdotool type exit={status:?}");

    Ok(())
}
