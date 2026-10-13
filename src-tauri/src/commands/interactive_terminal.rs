//! Interactive terminal (PTY)


/// Open a PTY shell at `root` and stream its output via `terminal-output`
/// (base64). Fires `terminal-exit` when the shell exits.
///
/// Every chunk goes two places: to the UI, and into `terminal::feed`, which is how
/// an agent `run_shell` running in this same shell learns what it printed.
#[tauri::command]
pub fn terminal_open(app: tauri::AppHandle, root: String, cols: u16, rows: u16) -> Result<(), String> {
    use base64::Engine;
    use std::io::Read;
    use tauri::Emitter;

    let mut reader = crate::core::terminal::open(&root, cols, rows)?;
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => { let _ = app.emit("terminal-exit", ()); break; }
                Ok(n) => {
                    crate::core::terminal::feed(&String::from_utf8_lossy(&buf[..n]));
                    let data = base64::engine::general_purpose::STANDARD.encode(&buf[..n]);
                    let _ = app.emit("terminal-output", data);
                }
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn terminal_write(data: String) {
    crate::core::terminal::write(&data);
}

#[tauri::command]
pub fn terminal_resize(cols: u16, rows: u16) {
    crate::core::terminal::resize(cols, rows);
}

#[tauri::command]
pub fn terminal_close() {
    crate::core::terminal::close();
}
