fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    #[cfg(windows)]
    if std::path::Path::new("assets/Oxycash_icon.ico").exists() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/Oxycash_icon.ico");
        let _ = res.compile();
    }
}