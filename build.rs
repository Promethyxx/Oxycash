fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let icon = std::path::Path::new("assets/Oxycash_icon.ico");
        if icon.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon.to_str().unwrap());
            res.compile().expect("Failed to compile Windows resources");
        }
    }
}
