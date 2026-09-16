//! Optional Windows console for log output.
//! The GUI binary uses `windows_subsystem = "windows"` so no console appears by default.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static CONSOLE_VISIBLE: AtomicBool = AtomicBool::new(false);
static CONSOLE_OUT: Mutex<Option<std::fs::File>> = Mutex::new(None);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn AllocConsole() -> i32;
    fn FreeConsole() -> i32;
    fn GetConsoleWindow() -> *mut core::ffi::c_void;
    fn SetConsoleTitleW(lp_console_title: *const u16) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Show a console window. Returns a writer handle for logging, if available.
pub fn show() -> bool {
    if CONSOLE_VISIBLE.load(Ordering::SeqCst) {
        return true;
    }
    unsafe {
        if GetConsoleWindow().is_null() && AllocConsole() == 0 {
            return false;
        }
        let title = to_wide("SexLab Scene Builder — Console");
        SetConsoleTitleW(title.as_ptr());
    }
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .open("CONOUT$")
        .ok();
    if let Ok(mut slot) = CONSOLE_OUT.lock() {
        *slot = file;
    }
    CONSOLE_VISIBLE.store(true, Ordering::SeqCst);
    let _ = writeln!(console_writer(), "Console logging enabled.");
    true
}

/// Hide / detach the console window.
pub fn hide() {
    CONSOLE_VISIBLE.store(false, Ordering::SeqCst);
    if let Ok(mut slot) = CONSOLE_OUT.lock() {
        *slot = None;
    }
    unsafe {
        FreeConsole();
    }
}

/// Writer that mirrors log lines into the allocated console (if any).
pub struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut slot) = CONSOLE_OUT.lock() {
            if let Some(file) = slot.as_mut() {
                return file.write(buf);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Ok(mut slot) = CONSOLE_OUT.lock() {
            if let Some(file) = slot.as_mut() {
                return file.flush();
            }
        }
        Ok(())
    }
}

pub fn console_writer() -> ConsoleWriter {
    ConsoleWriter
}
