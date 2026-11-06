//! Windows: the program's icon and version details (Explorer, the taskbar,
//! the file's Properties). A missing resource compiler only loses those.

fn main() {
    println!("cargo:rerun-if-changed=assets/openfo3.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/openfo3.ico")
        .set("ProductName", "OpenFO3")
        .set("FileDescription", "OpenFO3 Launcher")
        .set("CompanyName", "RPGKING The Mad Modder")
        .set(
            "LegalCopyright",
            "Copyright (C) 2026 RPGKING The Mad Modder. GPL-3.0-or-later.",
        );
    if let Err(e) = res.compile() {
        println!("cargo:warning=no icon or version resource: {e}");
    }
}
