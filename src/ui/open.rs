use std::process::Command;

pub fn open_in_browser(url: &str) -> Result<(), String> {
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "linux") {
        Command::new("xdg-open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", url]).spawn()
    } else {
        return Err("Unsupported platform".into());
    };

    result
        .map(|_| ())
        .map_err(|e| format!("Failed to open browser: {}", e))
}
