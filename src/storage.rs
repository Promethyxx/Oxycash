// oxycash-rs - storage.rs
use std::collections::HashSet;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono;

use crate::model::{AppData, Line, Payment, apply_recurring};

const UA: &str = "Oxycash-rs/0.1";

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub slug: String,
    #[serde(default)] pub dav_url:      String,
    #[serde(default)] pub dav_user:     String,
    #[serde(default)] pub dav_pass:     String,
    #[serde(default)] pub dav2_url:     String,
    #[serde(default)] pub dav2_user:    String,
    #[serde(default)] pub dav2_pass:    String,
    #[serde(default)] pub dav2_enabled: bool,
}

impl Profile {
    pub fn default_profile() -> Self {
        Self {
            name: "Default".into(), slug: "default".into(),
            dav_url: String::new(), dav_user: String::new(), dav_pass: String::new(),
            dav2_url: String::new(), dav2_user: String::new(), dav2_pass: String::new(),
            dav2_enabled: false,
        }
    }

    pub fn as_dav2_profile(&self) -> Profile {
        Profile {
            name: self.name.clone(), slug: self.slug.clone(),
            dav_url: self.dav2_url.clone(), dav_user: self.dav2_user.clone(), dav_pass: self.dav2_pass.clone(),
            dav2_url: String::new(), dav2_user: String::new(), dav2_pass: String::new(),
            dav2_enabled: false,
        }
    }

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub profiles:       Vec<Profile>,
    #[serde(default = "default_slug")]     pub active:         String,
    #[serde(default = "default_lang")]     pub lang:           String,
    #[serde(default)]                      pub font_scale:     i32,
    #[serde(default = "default_currency")] pub currency:       String,
    /// Chemin racine des données. "" = oxycash_config/ à côté de l'exe.
    #[serde(default)]                      pub data_dir:       String,
    /// Backup local activé
    #[serde(default = "default_true")]     pub backup_local:   bool,
    /// Backup WebDAV activé
    #[serde(default = "default_true")]     pub backup_webdav:  bool,
}

fn default_slug()     -> String { "default".into() }
fn default_lang()     -> String { "en".into() }
fn default_currency() -> String { "CHF".into() }
fn default_true()     -> bool   { true }

impl Default for Config {
    fn default() -> Self {
        Self {
            profiles:      vec![Profile::default_profile()],
            active:        "default".into(),
            lang:          "en".into(),
            font_scale:    0,
            currency:      "CHF".into(),
            data_dir:      String::new(),
            backup_local:  true,
            backup_webdav: true,
        }
    }
}

// ── Paths ─────────────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
static ANDROID_DATA_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_data_dir(path: PathBuf) {
    let _ = ANDROID_DATA_DIR.set(path);
}

/// Dossier racine des données. Sur Android : interne. Sur desktop : oxycash_config/ à côté de l'exe
/// sauf si config.data_dir est renseigné.
fn base_dir(cfg: &Config) -> PathBuf {
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

fn backup_dir(cfg: &Config) -> PathBuf {
    base_dir(cfg).join("backup")
}

/// Chemin du fichier config (toujours à côté de l'exe ou dans base_dir).
fn conf_file(cfg: &Config) -> PathBuf {
    base_dir(cfg).join("config.json")
}

/// Chemin du fichier de données actif pour un profil.
fn local_data_file(cfg: &Config, slug: &str) -> PathBuf {
    base_dir(cfg).join(format!("oxycash_{}.json", slug))
}

fn dav_filename(slug: &str) -> String {
    format!("oxycash_{}.json", slug)
}

fn dav_marker_filename(slug: &str) -> String {
    format!("oxycash_{}.sync.json", slug)
}

// ── Config I/O ────────────────────────────────────────────────────────────────

/// Charge la config depuis l'emplacement par défaut (à côté de l'exe).
/// On ne peut pas utiliser base_dir() ici car on n'a pas encore la config.
fn default_conf_file() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Some(p) = ANDROID_DATA_DIR.get() {
            return p.join("config.json");
        }
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

pub fn load_config() -> Config {
    let path = default_conf_file();
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
    let path = conf_file(cfg);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
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

// ── Backup ────────────────────────────────────────────────────────────────────

/// Timestamp lisible pour les noms de fichiers backup : YYYY-MM-DD_HHMMSS
fn ts_filename() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string()
}

/// Crée un backup local du profil actif et purge les anciens (garde 30 max).
pub fn backup_local(cfg: &Config, slug: &str, json: &str) {
    if !cfg.backup_local { return; }
    let dir = backup_dir(cfg);
    let _ = std::fs::create_dir_all(&dir);
    let fname = format!("oxycash_{}_{}.json", slug, ts_filename());
    let _ = std::fs::write(dir.join(&fname), json);
    // Purge : garde les 30 plus récents
    if let Ok(mut entries) = std::fs::read_dir(&dir) {
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| {
                e.file_name().to_string_lossy()
                    .starts_with(&format!("oxycash_{}_", slug))
            })
            .map(|e| e.path())
            .collect();
        files.sort();
        if files.len() > 30 {
            for old in &files[..files.len() - 30] {
                let _ = std::fs::remove_file(old);
            }
        }
    }
}

/// Backup WebDAV : écrit le fichier dans un sous-dossier backup/ sur le WebDAV.
fn backup_dav_upload(profile: &Profile, slug: &str, json: &str) {
    let base_url = profile.dav_url.trim();
    if base_url.is_empty() { return; }
    let base = if base_url.ends_with('/') { base_url.to_string() } else { format!("{}/", base_url) };
    let base = if base.starts_with("http") { base } else { format!("https://{}", base) };
    let fname = format!("backup/oxycash_{}_{}.json", slug, ts_filename());
    let url = format!("{}{}", base, fname);
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    if let Ok(client) = make_client() {
        let _ = client.put(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(json.to_string())
            .send();
    }
}

// ── WebDAV helpers ────────────────────────────────────────────────────────────

fn dav_full_url(profile: &Profile) -> Option<String> {
    let url  = profile.dav_url.trim();
    let user = profile.dav_user.trim();
    let pw   = profile.dav_pass.trim();
    if url.is_empty() || user.is_empty() || pw.is_empty() { return None; }
    let base = if url.ends_with('/') { url.to_string() } else { format!("{}/", url) };
    let full = if base.starts_with("http://") || base.starts_with("https://") {
        format!("{}{}", base, dav_filename(&profile.slug))
    } else {
        format!("https://{}{}", base, dav_filename(&profile.slug))
    };
    Some(full)
}

fn dav_marker_url(profile: &Profile) -> Option<String> {
    let url  = profile.dav_url.trim();
    let user = profile.dav_user.trim();
    let pw   = profile.dav_pass.trim();
    if url.is_empty() || user.is_empty() || pw.is_empty() { return None; }
    let base = if url.ends_with('/') { url.to_string() } else { format!("{}/", url) };
    let full = if base.starts_with("http://") || base.starts_with("https://") {
        format!("{}{}", base, dav_marker_filename(&profile.slug))
    } else {
        format!("https://{}{}", base, dav_marker_filename(&profile.slug))
    };
    Some(full)
}

fn auth_header(user: &str, pw: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("{}:{}", user, pw)))
}

pub fn make_client() -> Result<reqwest::blocking::Client, String> {
    let builder = reqwest::blocking::Client::builder()
        .user_agent(UA)
        .timeout(std::time::Duration::from_secs(10));

    #[cfg(target_os = "android")]
    let builder = {
        let root_store = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
        let tls = rustls::ClientConfig::builder_with_provider(
            std::sync::Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
        )
        .with_safe_default_protocol_versions()
        .map_err(|e| format!("tls: {}", e))?
        .with_root_certificates(root_store)
        .with_no_client_auth();
        builder.use_preconfigured_tls(tls)
    };

    builder.build().map_err(|e| format!("client build: {}", e))
}

pub fn dav_test_http(profile: &Profile, client: reqwest::blocking::Client) -> (bool, String) {
    let url = match dav_full_url(profile) {
        Some(u) => u,
        None => return (false, "url/user/pass manquant".into()),
    };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    match client.head(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() || r.status().as_u16() == 404 => {
            (true, format!("Connecté ✓ (HTTP {})", r.status().as_u16()))
        }
        Ok(r) => (false, format!("HTTP {} — user/pass ou chemin?", r.status().as_u16())),
        Err(e) => {
            let mut msg = format!("ERR: {}", e);
            let mut src: &dyn std::error::Error = &e;
            while let Some(s) = src.source() { msg.push_str(&format!(" | {}", s)); src = s; }
            (false, msg)
        }
    }
}

fn dav_load(profile: &Profile) -> Option<AppData> {
    let url = dav_full_url(profile)?;
    let client = make_client().ok()?;
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let resp = client.get(&url).header("Authorization", &auth).send().ok()?;
    if !resp.status().is_success() { return None; }
    let text = resp.text().ok()?;
    AppData::from_json(&text).ok()
}

fn dav_save(profile: &Profile, data: &AppData) -> bool {
    let url = match dav_full_url(profile) {
        Some(u) => u, None => return false,
    };
    let client = match make_client() { Ok(c) => c, Err(_) => return false };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let body = data.to_json();
    match client.put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body).send()
    {
        Ok(r) => matches!(r.status().as_u16(), 200 | 201 | 204),
        Err(_) => false,
    }
}

// ── Marqueur de sync ──────────────────────────────────────────────────────────

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn dav_read_marker(profile: &Profile) -> u64 {
    let url = match dav_marker_url(profile) { Some(u) => u, None => return 0 };
    let client = match make_client() { Ok(c) => c, Err(_) => return 0 };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let resp = match client.get(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() => r, _ => return 0,
    };
    let text = match resp.text() { Ok(t) => t, Err(_) => return 0 };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v["ts"].as_u64())
        .unwrap_or(0)
}

fn dav_write_marker(profile: &Profile, ts: u64) -> bool {
    let url = match dav_marker_url(profile) { Some(u) => u, None => return false };
    let client = match make_client() { Ok(c) => c, Err(_) => return false };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let body = format!(
        "{{\"ts\":{},\"app\":\"Oxycash\",\"profile\":\"{}\"}}",
        ts, profile.slug
    );
    match client.put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body).send()
    {
        Ok(r) => matches!(r.status().as_u16(), 200 | 201 | 204),
        Err(_) => false,
    }
}

// ── Storage manager ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus { Dav, DavError, Local }

pub struct Storage {
    pub cfg:    Config,
    pub data:   AppData,
    pub dav_ok: bool,
}

impl Storage {
    pub fn new() -> Self {
        Self { cfg: load_config(), data: AppData::default(), dav_ok: false }
    }

    // ── Profile helpers ───────────────────────────────────────────────────

    pub fn active_profile(&self) -> &Profile {
        let slug = &self.cfg.active;
        self.cfg.profiles.iter().find(|p| &p.slug == slug)
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
        let existing: HashSet<_> = self.cfg.profiles.iter().map(|p| p.slug.clone()).collect();
        let mut slug = base.clone();
        let mut n = 2;
        while existing.contains(&slug) { slug = format!("{}_{}", base, n); n += 1; }
        self.cfg.profiles.push(Profile {
            name: name.to_string(), slug: slug.clone(),
            dav_url: String::new(), dav_user: String::new(), dav_pass: String::new(),
            dav2_url: String::new(), dav2_user: String::new(), dav2_pass: String::new(),
            dav2_enabled: false,
        });
        save_config(&self.cfg);
        slug
    }

    pub fn delete_profile(&mut self, slug: &str) {
        if self.cfg.profiles.len() <= 1 { return; }
        self.cfg.profiles.retain(|p| p.slug != slug);
        let _ = std::fs::remove_file(local_data_file(&self.cfg, slug));
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
            p.dav_url = url.to_string(); p.dav_user = user.to_string(); p.dav_pass = pw.to_string();
        }
        save_config(&self.cfg);
    }

    pub fn save_profile_dav2(&mut self, slug: &str, url: &str, user: &str, pw: &str, enabled: bool) {
        if let Some(p) = self.cfg.profiles.iter_mut().find(|p| p.slug == slug) {
            p.dav2_url = url.to_string(); p.dav2_user = user.to_string();
            p.dav2_pass = pw.to_string(); p.dav2_enabled = enabled;
        }
        save_config(&self.cfg);
    }

    pub fn test_dav2(&self) -> (bool, String) {
        let prof = self.active_profile();
        if !prof.has_dav2() { return (false, "WebDAV secondaire non configuré".into()); }
        match make_client() {
            Ok(c) => dav_test_http(&prof.as_dav2_profile(), c),
            Err(e) => (false, e),
        }
    }

    // ── Load / Save ───────────────────────────────────────────────────────

    pub fn load(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();

        let dav1_ok = prof.has_dav();
        let dav2_ok = prof.has_dav2();

        if dav1_ok || dav2_ok {
            let ts1 = if dav1_ok { dav_read_marker(&prof) } else { 0 };
            let ts2 = if dav2_ok { dav_read_marker(&prof.as_dav2_profile()) } else { 0 };
            let p2 = prof.as_dav2_profile();
            let ordered: Vec<&Profile> = if ts2 > ts1 && dav2_ok {
                vec![&p2, &prof]
            } else if dav1_ok {
                vec![&prof, &p2]
            } else {
                vec![&p2]
            };
            let sources: Vec<&Profile> = ordered.into_iter()
                .filter(|p| dav_full_url(p).is_some()).collect();

            for source in sources {
                if let Some(mut app) = dav_load(source) {
                    apply_recurring(&mut app);
                    let dir = base_dir(&self.cfg);
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(local_data_file(&self.cfg, &slug), app.to_json());
                    self.data   = app;
                    self.dav_ok = true;
                    return SyncStatus::Dav;
                }
            }
        }

        self.dav_ok = false;
        let lf = local_data_file(&self.cfg, &slug);
        if lf.exists() {
            if let Ok(text) = std::fs::read_to_string(&lf) {
                if let Ok(mut app) = AppData::from_json(&text) {
                    apply_recurring(&mut app);
                    self.data = app;
                    return SyncStatus::Local;
                }
            }
        }

        self.data = AppData::new_empty();
        apply_recurring(&mut self.data);
        let dir = base_dir(&self.cfg);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(local_data_file(&self.cfg, &slug), self.data.to_json());
        SyncStatus::Local
    }

    pub fn save(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();
        let json = self.data.to_json();

        // Backup local
        if self.cfg.backup_local {
            let dir = base_dir(&self.cfg);
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(local_data_file(&self.cfg, &slug), &json);
        }

        let dav1_ok = prof.has_dav();
        let dav2_ok = prof.has_dav2();

        if dav1_ok || dav2_ok {
            let ts = now_ts();
            let json_clone  = json.clone();
            let prof_clone  = prof.clone();
            let backup_dav  = self.cfg.backup_webdav;
            let slug_clone  = slug.clone();

            std::thread::spawn(move || {
                if dav1_ok {
                    if dav_save(&prof_clone, &AppData::from_json(&json_clone).unwrap_or_default()) {
                        dav_write_marker(&prof_clone, ts);
                        if backup_dav {
                            backup_dav_upload(&prof_clone, &slug_clone, &json_clone);
                        }
                    }
                }
                if dav2_ok {
                    let p2 = prof_clone.as_dav2_profile();
                    if dav_save(&p2, &AppData::from_json(&json_clone).unwrap_or_default()) {
                        dav_write_marker(&p2, ts);
                        if backup_dav {
                            backup_dav_upload(&p2, &slug_clone, &json_clone);
                        }
                    }
                }
            });

            self.dav_ok = true;
            return SyncStatus::Dav;
        }

        self.dav_ok = false;
        SyncStatus::Local
    }

    /// Backup à la fermeture : sauvegarde locale + WebDAV sans purge de session.
    pub fn backup_on_exit(&self) {
        let slug = self.active_profile().slug.clone();
        let json = self.data.to_json();
        backup_local(&self.cfg, &slug, &json);
        if self.cfg.backup_webdav {
            let prof = self.active_profile().clone();
            let slug2 = slug.clone();
            let json2 = json.clone();
            std::thread::spawn(move || {
                if prof.has_dav()  { backup_dav_upload(&prof, &slug2, &json2); }
                if prof.has_dav2() { backup_dav_upload(&prof.as_dav2_profile(), &slug2, &json2); }
            });
        }
    }

    pub fn status(&self) -> SyncStatus {
        if self.dav_ok { return SyncStatus::Dav; }
        let prof = self.active_profile();
        if prof.has_dav() || prof.has_dav2() { return SyncStatus::DavError; }
        SyncStatus::Local
    }

    pub fn test_dav(&self) -> (bool, String) {
        match make_client() {
            Ok(c) => dav_test_http(self.active_profile(), c),
            Err(e) => (false, e),
        }
    }

    // ── Import / Export JSON ──────────────────────────────────────────────

    pub fn export_json(&self) -> String { self.data.to_json() }

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

    // ── Préférences ───────────────────────────────────────────────────────

    pub fn set_currency(&mut self, cur: &str) {
        self.cfg.currency = if cur.trim().is_empty() { "CHF".into() } else { cur.trim().into() };
        save_config(&self.cfg);
    }

    pub fn set_lang(&mut self, lang: &str) {
        self.cfg.lang = lang.to_string();
        save_config(&self.cfg);
    }

    pub fn set_data_dir(&mut self, path: &str) {
        self.cfg.data_dir = path.trim().to_string();
        save_config(&self.cfg);
    }

    pub fn set_backup_local(&mut self, val: bool) {
        self.cfg.backup_local = val;
        save_config(&self.cfg);
    }

    pub fn set_backup_webdav(&mut self, val: bool) {
        self.cfg.backup_webdav = val;
        save_config(&self.cfg);
    }

    pub fn data_dir_display(&self) -> String {
        if self.cfg.data_dir.trim().is_empty() {
            base_dir(&self.cfg).to_string_lossy().to_string()
        } else {
            self.cfg.data_dir.clone()
        }
    }

    pub fn backup_dir_display(&self) -> String {
        backup_dir(&self.cfg).to_string_lossy().to_string()
    }
}
