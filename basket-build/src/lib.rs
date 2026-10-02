//! Build-script helpers. Use from a `build.rs` (as a `[build-dependencies]` entry):
//!
//! ```no_run
//! basket_build::windows_icon("../docs/branding/icons/app.ico", "App", "App, an emulator");
//! ```

/// On a Windows target, embed `ico` (a path relative to the package) and a version block naming
/// `product` and `description` into the exe, so Explorer and the taskbar show the mark. Elsewhere
/// it does nothing. A missing resource compiler (`rc.exe` / `windres`) only warns, so the build
/// never depends on it.
pub fn windows_icon(ico: &str, product: &str, description: &str) {
    println!("cargo:rerun-if-changed={ico}");
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
            return;
        }
        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico);
        res.set("ProductName", product);
        res.set("FileDescription", description);
        if let Err(e) = res.compile() {
            println!("cargo:warning=exe icon not embedded (no rc.exe/windres?): {e}");
        }
    }
    #[cfg(not(windows))]
    let _ = (product, description);
}
