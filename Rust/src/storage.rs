// oxycash-rs - storage.rs
// Mapping de core/storage.py
// WebDAV via reqwest (rustls, pas de native-tls)
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::model::{AppData, apply_recurring};

const UA: &str = "Oxycash-rs/0.1";

// --- Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub dav_url: String,
    #[serde(default)]
    pub dav_user: String,
    #[serde(default)]
    pub dav_pass: String,
}

impl Profile {
    pub fn default_profile() -> Self {
        Self {
            name: "Default".into(),
            slug: "default".into(),
            dav_url: String::new(),
            dav_user: String::new(),
            dav_pass: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub profiles: Vec<Profile>,
    #[serde(default = "default_slug")]
    pub active: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub font_scale: i32,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_slug() -> String     { "default".into() }
fn default_lang() -> String     { "en".into() }
fn default_currency() -> String { "CHF".into() }

impl Default for Config {
    fn default() -> Self {
        Self {
            profiles: vec![Profile::default_profile()],
            active: "default".into(),
            lang: "en".into(),
            font_scale: 0,
            currency: "CHF".into(),
        }
    }
}

// --- Paths
fn local_dir() -> PathBuf {
    // On Android, dirs::home_dir() returns the app internal storage
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".oxycash")
}

fn conf_file() -> PathBuf {
    local_dir().join("config.json")
}

fn local_data_file(slug: &str) -> PathBuf {
    local_dir().join(format!("oxycash_{}.json", slug))
}

fn dav_filename(slug: &str) -> String {
    format!("oxycash_{}.json", slug)
}

// --- Config I/O
pub fn load_config() -> Config {
    let path = conf_file();
    if path.exists() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str::<Config>(&text) {
                return cfg;
            }
        }
    }
    Config::default()
}

pub fn save_config(cfg: &Config) {
    let dir = local_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = conf_file();
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(path, json);
    }
}

fn slugify(name: &str) -> String {
    let s: String = name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    s.trim_matches('_').to_string()
}

// --- WebDAV helpers
fn dav_full_url(profile: &Profile) -> Option<String> {
    let url  = profile.dav_url.trim();
    let user = profile.dav_user.trim();
    let pw   = profile.dav_pass.trim();
    if url.is_empty() || user.is_empty() || pw.is_empty() {
        return None;
    }
    let base = if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{}/", url)
    };
    let full = if base.starts_with("http://") || base.starts_with("https://") {
        format!("{}{}", base, dav_filename(&profile.slug))
    } else {
        format!("https://{}{}", base, dav_filename(&profile.slug))
    };
    Some(full)
}

fn auth_header(user: &str, pw: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("{}:{}", user, pw)))
}

fn make_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("reqwest client build failed")
}

pub fn dav_test(profile: &Profile) -> (bool, String) {
    let url = match dav_full_url(profile) {
        Some(u) => u,
        None => return (false, "Connection failed".into()),
    };
    let client = make_client();
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    match client.head(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() || r.status().as_u16() == 404 => {
            (true, "Connected ✓".into())
        }
        _ => (false, "Connection failed".into()),
    }
}

fn dav_load(profile: &Profile) -> Option<AppData> {
    let url = dav_full_url(profile)?;
    let client = make_client();
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let resp = client.get(&url).header("Authorization", &auth).send().ok()?;
    if !resp.status().is_success() { return None; }
    let text = resp.text().ok()?;
    AppData::from_json(&text).ok()
}

fn dav_save(profile: &Profile, data: &AppData) -> bool {
    let url = match dav_full_url(profile) {
        Some(u) => u,
        None => return false,
    };
    let client = make_client();
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let body = data.to_json();
    match client
        .put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body)
        .send()
    {
        Ok(r) => matches!(r.status().as_u16(), 200 | 201 | 204),
        Err(_) => false,
    }
}

// --- Storage manager
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus {
    Dav,
    DavError,
    Local,
}

pub struct Storage {
    pub cfg: Config,
    pub data: AppData,
    pub dav_ok: bool,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            cfg: load_config(),
            data: AppData::default(),
            dav_ok: false,
        }
    }

    // --- Profile helpers
    pub fn active_profile(&self) -> &Profile {
        let slug = &self.cfg.active;
        self.cfg.profiles.iter()
            .find(|p| &p.slug == slug)
            .unwrap_or(&self.cfg.profiles[0])
    }

    pub fn active_profile_mut(&mut self) -> &mut Profile {
        let slug = self.cfg.active.clone();
        if let Some(i) = self.cfg.profiles.iter().position(|p| p.slug == slug) {
            &mut self.cfg.profiles[i]
        } else {
            &mut self.cfg.profiles[0]
        }
    }

    pub fn add_profile(&mut self, name: &str) -> String {
        let base = slugify(name);
        let existing: std::collections::HashSet<_> =
            self.cfg.profiles.iter().map(|p| p.slug.clone()).collect();
        let mut slug = base.clone();
        let mut n = 2;
        while existing.contains(&slug) {
            slug = format!("{}_{}", base, n);
            n += 1;
        }
        self.cfg.profiles.push(Profile {
            name: name.to_string(),
            slug: slug.clone(),
            dav_url: String::new(),
            dav_user: String::new(),
            dav_pass: String::new(),
        });
        save_config(&self.cfg);
        slug
    }

    pub fn delete_profile(&mut self, slug: &str) {
        if self.cfg.profiles.len() <= 1 { return; }
        self.cfg.profiles.retain(|p| p.slug != slug);
        let f = local_data_file(slug);
        let _ = std::fs::remove_file(f);
        if self.cfg.active == slug {
            self.cfg.active = self.cfg.profiles[0].slug.clone();
        }
        save_config(&self.cfg);
    }

    pub fn switch_profile(&mut self, slug: &str) {
        self.cfg.active = slug.to_string();
        save_config(&self.cfg);
        self.load();
    }

    pub fn save_profile_dav(&mut self, slug: &str, url: &str, user: &str, pw: &str) {
        if let Some(p) = self.cfg.profiles.iter_mut().find(|p| p.slug == slug) {
            p.dav_url  = url.to_string();
            p.dav_user = user.to_string();
            p.dav_pass = pw.to_string();
        }
        save_config(&self.cfg);
    }

    // --- Load / Save
    pub fn load(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();

        // Try WebDAV first
        if dav_full_url(&prof).is_some() {
            if let Some(mut app) = dav_load(&prof) {
                apply_recurring(&mut app);
                // Cache locally
                let dir = local_dir();
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::fs::write(local_data_file(&slug), app.to_json());
                self.data   = app;
                self.dav_ok = true;
                return SyncStatus::Dav;
            }
        }
        self.dav_ok = false;

        // Fallback: local cache
        let lf = local_data_file(&slug);
        if lf.exists() {
            if let Ok(text) = std::fs::read_to_string(&lf) {
                if let Ok(mut app) = AppData::from_json(&text) {
                    apply_recurring(&mut app);
                    self.data = app;
                    return SyncStatus::Local;
                }
            }
        }

        // Default: fresh empty data
        self.data = AppData::new_empty();
        apply_recurring(&mut self.data);
        let dir = local_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(local_data_file(&slug), self.data.to_json());
        SyncStatus::Local
    }

    pub fn save(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();

        let dir = local_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(local_data_file(&slug), self.data.to_json());

        if dav_full_url(&prof).is_some() {
            let ok = dav_save(&prof, &self.data);
            self.dav_ok = ok;
            if ok { return SyncStatus::Dav; }
            return SyncStatus::DavError;
        }
        self.dav_ok = false;
        SyncStatus::Local
    }

    pub fn status(&self) -> SyncStatus {
        if self.dav_ok { return SyncStatus::Dav; }
        if dav_full_url(self.active_profile()).is_some() {
            return SyncStatus::DavError;
        }
        SyncStatus::Local
    }

    pub fn test_dav(&self) -> (bool, String) {
        dav_test(self.active_profile())
    }

    // --- Import / Export
    pub fn export_json(&self) -> String {
        self.data.to_json()
    }

    pub fn import_json(&mut self, raw: &str) -> bool {
        match AppData::from_json(raw) {
            Ok(app) if !app.months.is_empty() => {
                self.data = app;
                self.save();
                true
            }
            _ => false,
        }
    }

    pub fn reset(&mut self) {
        self.data = AppData::new_empty();
        self.save();
    }

    // --- Preferences
    pub fn set_currency(&mut self, cur: &str) {
        self.cfg.currency = if cur.trim().is_empty() {
            "CHF".into()
        } else {
            cur.trim().into()
        };
        save_config(&self.cfg);
    }

    pub fn set_lang(&mut self, lang: &str) {
        self.cfg.lang = lang.to_string();
        save_config(&self.cfg);
    }
}
