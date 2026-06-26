// oxycash-rs - config.rs
// Profile & Config structs, file-system paths, config I/O, local backup, slugify.
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono;

// ── Profile ───────────────────────────────────────────────────────────────────
// A profile is just a named data slot. DAV credentials are global (in Config).

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub slug: String,
}

impl Profile {
    pub fn default_profile() -> Self {
        Self { name: "Default".into(), slug: "default".into() }
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub profiles:      Vec<Profile>,
    #[serde(default = "default_slug")]     pub active:        String,
    #[serde(default = "default_lang")]     pub lang:          String,
    #[serde(default)]                      pub font_scale:    i32,
    #[serde(default = "default_currency")] pub currency:      String,
    #[serde(default)]                      pub data_dir:      String,
    #[serde(default = "default_true")]     pub backup_local:  bool,
    #[serde(default = "default_true")]     pub backup_webdav: bool,
    // WebDAV1 — global, shared by all profiles
    #[serde(default)] pub dav_url:      String,
    #[serde(default)] pub dav_user:     String,
    #[serde(default)] pub dav_pass:     String,
    // WebDAV2 — optional secondary, global
    #[serde(default)] pub dav2_url:     String,
    #[serde(default)] pub dav2_user:    String,
    #[serde(default)] pub dav2_pass:    String,
    #[serde(default)] pub dav2_enabled: bool,
}

fn default_slug()     -> String { "default".into() }
fn default_lang()     -> String { "en".into() }
fn default_currency() -> String { "CHF".into() }
fn default_true()     -> bool   { true }

impl Default for Config {
    fn default() -> Self {
        Self {
            profiles:     vec![Profile::default_profile()],
            active:       "default".into(),
            lang:         "en".into(),
            font_scale:   0,
            currency:     "CHF".into(),
            data_dir:     String::new(),
            backup_local: true, backup_webdav: true,
            dav_url:  String::new(), dav_user:  String::new(), dav_pass:  String::new(),
            dav2_url: String::new(), dav2_user: String::new(), dav2_pass: String::new(),
            dav2_enabled: false,
        }
    }
}

impl Config {
    pub fn has_dav(&self) -> bool {
        !self.dav_url.trim().is_empty()
            && !self.dav_user.trim().is_empty()
            && !self.dav_pass.trim().is_empty()
    }

    pub fn has_dav2(&self) -> bool {
        self.dav2_enabled
            && !self.dav2_url.trim().is_empty()
            && !self.dav2_user.trim().is_empty()
            && !self.dav2_pass.trim().is_empty()
    }

    /// Build a Profile-shaped struct for dav functions that expect credentials + slug.
    pub fn dav_profile(&self, slug: &str) -> DavProfile {
        DavProfile {
            slug:     slug.to_string(),
            dav_url:  self.dav_url.clone(),
            dav_user: self.dav_user.clone(),
            dav_pass: self.dav_pass.clone(),
        }
    }

    pub fn dav2_profile(&self, slug: &str) -> DavProfile {
        DavProfile {
            slug:     slug.to_string(),
            dav_url:  self.dav2_url.clone(),
            dav_user: self.dav2_user.clone(),
            dav_pass: self.dav2_pass.clone(),
        }
    }
}

/// Lightweight credential+slug bundle used by webdav functions.
/// Replaces the old per-profile DAV fields.
#[derive(Debug, Clone)]
pub struct DavProfile {
    pub slug:     String,
    pub dav_url:  String,
    pub dav_user: String,
    pub dav_pass: String,
}

impl DavProfile {
    pub fn has_dav(&self) -> bool {
        !self.dav_url.trim().is_empty()
            && !self.dav_user.trim().is_empty()
            && !self.dav_pass.trim().is_empty()
    }
}

// ── Android data-dir override ─────────────────────────────────────────────────

#[cfg(target_os = "android")]
static ANDROID_DATA_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_data_dir(path: PathBuf) {
    let _ = ANDROID_DATA_DIR.set(path);
}

// ── Paths ─────────────────────────────────────────────────────────────────────

pub fn base_dir(cfg: &Config) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Some(p) = ANDROID_DATA_DIR.get() { return p.clone(); }
        return PathBuf::from(".");
    }
    #[cfg(not(target_os = "android"))]
    {
        if !cfg.data_dir.trim().is_empty() {
            return PathBuf::from(cfg.data_dir.trim());
        }
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        exe.parent().unwrap_or(std::path::Path::new("."))
            .join("oxycash_config")
    }
}

pub fn backup_dir(cfg: &Config) -> PathBuf {
    base_dir(cfg).join("backup")
}

pub fn default_conf_file() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Some(p) = ANDROID_DATA_DIR.get() { return p.join("config.json"); }
        return PathBuf::from("config.json");
    }
    #[cfg(not(target_os = "android"))]
    {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        exe.parent().unwrap_or(std::path::Path::new("."))
            .join("oxycash_config")
            .join("config.json")
    }
}

pub fn local_data_file(cfg: &Config, slug: &str) -> PathBuf {
    base_dir(cfg).join(format!("oxycash_{}.json", slug))
}

pub fn dav_filename(slug: &str)        -> String { format!("oxycash_{}.json", slug) }
pub fn dav_marker_filename(slug: &str) -> String { format!("oxycash_{}.sync.json", slug) }

// ── Config I/O ────────────────────────────────────────────────────────────────

pub fn load_config() -> Config {
    let path = default_conf_file();
    if path.exists() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(mut cfg) = serde_json::from_str::<Config>(&text) {
                if cfg.profiles.is_empty() {
                    cfg.profiles.push(Profile::default_profile());
                    cfg.active = "default".into();
                }
                // Migration: if old per-profile DAV fields exist, hoist them to Config level
                // (handled transparently by serde #[serde(default)] on the new Config fields)
                return cfg;
            }
        }
    }
    Config::default()
}

pub fn save_config(cfg: &Config) {
    let path = base_dir(cfg).join("config.json");
    if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(path, json);
    }
}

pub fn slugify(name: &str) -> String {
    let s: String = name.to_lowercase()
        .chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    s.trim_matches('_').to_string()
}

// ── Local backup ──────────────────────────────────────────────────────────────

fn ts_filename() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string()
}

pub fn backup_local(cfg: &Config, slug: &str, json: &str) {
    if !cfg.backup_local { return; }
    let dir = backup_dir(cfg);
    let _ = std::fs::create_dir_all(&dir);
    let fname = format!("oxycash_{}_{}.json", slug, ts_filename());
    let _ = std::fs::write(dir.join(&fname), json);
}
