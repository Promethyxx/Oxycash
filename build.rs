fn main() {
    slint_build::compile("ui/main.slint").unwrap();
    windows_resources();
}

fn windows_resources() {
    // On vérifie si la cible finale (target) est bien Windows
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    
    if target_os == "windows" {
        let icon = std::path::Path::new("assets/Oxycash_icon.ico");
        if icon.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon.to_str().unwrap());
            res.compile().expect("Failed to compile Windows resources");
        }
    }
}