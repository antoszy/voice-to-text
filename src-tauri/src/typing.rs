use anyhow::Result;
use std::process::Command;

/// Type text into the currently focused input field using clipboard + xdotool.
/// This approach handles Unicode correctly, unlike xdotool type.
pub fn type_text(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }

    // Save current clipboard
    let old_clipboard = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(o.stdout)
            } else {
                None
            }
        });

    // Set new clipboard content
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(text.as_bytes())?;
    }
    child.wait()?;

    // Small delay to ensure clipboard is set
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Paste via Ctrl+V
    Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .status()?;

    // Small delay before restoring clipboard
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Restore original clipboard
    if let Some(old) = old_clipboard {
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()?;
        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(&old)?;
        }
        child.wait()?;
    }

    Ok(())
}
