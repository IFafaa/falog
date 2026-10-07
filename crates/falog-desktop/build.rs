//! Embeds the app icon in `falog.exe`, so Explorer, the Start menu and taskbar pins show it, and names
//! the executable "Falog" in Task Manager instead of the crate description.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/falog.ico");
    // The resource compiler only exists on Windows hosts; cross builds go without an icon.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/icon/falog.ico")
            .set("FileDescription", "Falog")
            .set("ProductName", "Falog");
        if let Err(err) = resource.compile() {
            println!("cargo:warning=could not embed the app icon: {err}");
        }
    }
}
