//! Windows: OpenFO3's icon and version details on the game's program
//! (Explorer, the taskbar, the file's Properties). A missing resource
//! compiler only loses those.

fn main() {
    println!("cargo:rerun-if-changed=../launcher/assets/openfo3.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../launcher/assets/openfo3.ico")
        .set("ProductName", "OpenFO3")
        .set("FileDescription", "OpenFO3")
        .set("CompanyName", "RPGKING The Mad Modder")
        .set(
            "LegalCopyright",
            "Copyright (C) 2026 RPGKING The Mad Modder and the nv-rs contributors. GPL-3.0-or-later.",
        );
    if let Err(e) = res.compile() {
        println!("cargo:warning=no icon or version resource: {e}");
    }
}
