fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        if std::path::Path::new("assets/Oxycash_icon.ico").exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/Oxycash_icon.ico");
            res.compile().unwrap();
        }
    }
}