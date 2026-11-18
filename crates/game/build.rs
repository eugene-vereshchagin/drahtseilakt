// The build script: on Windows it puts the game's icon (scripts/packaging/icon.ico, our wooden ball) and the version
// into the program as a resource, so Explorer and the taskbar show it. Other targets need nothing here.
fn main() -> Result<(), String> {
    let ico = "../../scripts/packaging/icon.ico";
    println!("cargo:rerun-if-changed={ico}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico);
    res.set("ProductName", "Drahtseilakt");
    res.set("FileDescription", "Drahtseilakt");
    // a missing resource compiler must not pass silently: the release would ship without its icon. An error from main
    // fails the build with this message.
    res.compile()
        .map_err(|e| format!("the Windows resource (icon) did not compile: {e}"))
}
