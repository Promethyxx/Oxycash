fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    // Embed icon in Windows exe
    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new("assets/Oxycash_icon.ico").exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/Oxycash_icon.ico");
            res.compile().unwrap();
        }
    }
}
