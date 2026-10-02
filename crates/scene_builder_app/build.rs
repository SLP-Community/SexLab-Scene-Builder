fn main() {
    // Embed the .exe icon for Explorer / taskbar on Windows targets.
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // Cross-compiles without a Windows RC tool still get the egui window icon.
            println!("cargo:warning=winresource icon embed skipped: {e}");
        }
    }
}
