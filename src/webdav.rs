// oxycash-rs - webdav.rs
// WebDAV HTTP operations.
//
// Connection sequence (dav_connect):
//   1. PROPFIND oxycash_config/          — vérifier si le dossier existe
//      1a. 200/207 → rien
//      1b. 404     → MKCOL oxycash_config/
//   2. PROPFIND oxycash_config/backup/   — vérifier si backup/ existe
//      2a. 200/207 → rien
//      2b. 404     → MKCOL oxycash_config/backup/
//   3. GET oxycash_config/oxycash_xxx.json
//      3a. 200 → charger les données
//      3b. 404 → PUT le fichier local (premier push)
//   4. PUT oxycash_config/backup/oxycash_xxx_YYYY-MM-DD_HHMMSS.json
//      (pas de limite de rétention — les utilisateurs gèrent eux-mêmes)

use base64::{engine::general_purpose::STANDARD, Engine};

use crate::config::{Config, Profile, base_dir, dav_filename, dav_marker_filename};
use crate::model::AppData;

const UA: &str = "Oxycash-rs/0.1";

// ── HTTP client ───────────────────────────────────────────────────────────────

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
        .with_safe_default_protocol_versions().map_err(|e| format!("tls: {}", e))?
        .with_root_certificates(root_store).with_no_client_auth();
        builder.use_preconfigured_tls(tls)
    };
    builder.build().map_err(|e| format!("client build: {}", e))
}

// ── URL helpers ───────────────────────────────────────────────────────────────

/// Normalise a DAV base URL: ensure trailing slash and https:// prefix.
fn normalise_base(url: &str) -> String {
    let base = if url.ends_with('/') { url.to_string() } else { format!("{}/", url) };
    if base.starts_with("http") { base } else { format!("https://{}", base) }
}

/// Root DAV URL (user-configured base), normalised with trailing slash.
fn root_url(profile: &Profile) -> Option<String> {
    let url = profile.dav_url.trim();
    if url.is_empty() || profile.dav_user.trim().is_empty() || profile.dav_pass.trim().is_empty() {
        return None;
    }
    Some(normalise_base(url))
}

/// URL of the oxycash_config/ directory: <root>/oxycash_config/
fn dir_url(profile: &Profile) -> Option<String> {
    root_url(profile).map(|r| format!("{}oxycash_config/", r))
}

/// URL of the oxycash_config/backup/ subdirectory.
fn backup_dir_url(profile: &Profile) -> Option<String> {
    dir_url(profile).map(|d| format!("{}backup/", d))
}

/// URL of the data file: <root>/oxycash_config/oxycash_xxx.json
pub fn dav_full_url(profile: &Profile) -> Option<String> {
    dir_url(profile).map(|d| format!("{}{}", d, dav_filename(&profile.slug)))
}

/// URL of the sync-marker file: <root>/oxycash_config/oxycash_xxx.sync.json
fn dav_marker_url(profile: &Profile) -> Option<String> {
    dir_url(profile).map(|d| format!("{}{}", d, dav_marker_filename(&profile.slug)))
}

pub fn auth_header(user: &str, pw: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("{}:{}", user, pw)))
}

// ── Logging ───────────────────────────────────────────────────────────────────

pub fn log_dav(cfg: &Config, msg: &str) {
    let path = base_dir(cfg).join("dav.log");
    let ts   = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("[{}] {}\n", ts, msg);
    let _ = std::fs::create_dir_all(base_dir(cfg));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

// ── Connection test ───────────────────────────────────────────────────────────

/// HEAD the data file to verify credentials and reachability.
/// 404 is treated as success (credentials OK, file just doesn't exist yet).
pub fn dav_test_http(profile: &Profile, client: reqwest::blocking::Client) -> (bool, String) {
    let url = match dav_full_url(profile) {
        Some(u) => u,
        None    => return (false, "url/user/pass manquant".into()),
    };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    match client.head(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() || r.status().as_u16() == 404 =>
            (true, format!("Connecté ✓ (HTTP {})", r.status().as_u16())),
        Ok(r) => (false, format!("HTTP {} — user/pass ou chemin?", r.status().as_u16())),
        Err(e) => {
            let mut msg = format!("ERR: {}", e);
            let mut src: &dyn std::error::Error = &e;
            while let Some(s) = src.source() { msg.push_str(&format!(" | {}", s)); src = s; }
            (false, msg)
        }
    }
}

// ── Low-level PROPFIND / MKCOL ────────────────────────────────────────────────

/// PROPFIND a URL with Depth:0. Returns the HTTP status code, or Err on network failure.
fn propfind(client: &reqwest::blocking::Client, url: &str, auth: &str, cfg: &Config)
    -> Result<u16, String>
{
    log_dav(cfg, &format!("PROPFIND {}", url));
    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), url)
        .header("Authorization", auth)
        .header("Depth", "0")
        .send()
        .map_err(|e| { let m = format!("PROPFIND err: {}", e); log_dav(cfg, &m); m })?;
    let s = resp.status().as_u16();
    log_dav(cfg, &format!("PROPFIND status={}", s));
    Ok(s)
}

/// MKCOL a URL. Returns Ok(()) on 200/201/204, Err otherwise.
fn mkcol(client: &reqwest::blocking::Client, url: &str, auth: &str, cfg: &Config)
    -> Result<(), String>
{
    log_dav(cfg, &format!("MKCOL {}", url));
    let resp = client
        .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), url)
        .header("Authorization", auth)
        .send()
        .map_err(|e| { let m = format!("MKCOL err: {}", e); log_dav(cfg, &m); m })?;
    let s = resp.status().as_u16();
    log_dav(cfg, &format!("MKCOL status={}", s));
    match s {
        200 | 201 | 204 => Ok(()),
        s => Err(format!("MKCOL HTTP {}", s)),
    }
}

/// Ensure a directory exists: PROPFIND it, MKCOL if 404.
fn ensure_dir(client: &reqwest::blocking::Client, url: &str, auth: &str, cfg: &Config)
    -> Result<(), String>
{
    match propfind(client, url, auth, cfg)? {
        200 | 207 => { log_dav(cfg, &format!("dir exists: {}", url)); Ok(()) }
        404       => mkcol(client, url, auth, cfg),
        s         => Err(format!("PROPFIND HTTP {} for {}", s, url)),
    }
}

// ── Full connection sequence ──────────────────────────────────────────────────

/// Result of dav_connect.
pub enum ConnectResult {
    /// Data loaded from DAV.
    Loaded(AppData),
    /// No remote file found; local data was pushed.
    Pushed,
    /// Fatal error at one of the steps.
    Failed(String),
}

/// Full DAV connection sequence:
///   1. Ensure oxycash_config/ exists (PROPFIND → MKCOL if 404)
///   2. Ensure oxycash_config/backup/ exists (PROPFIND → MKCOL if 404)
///   3. GET oxycash_config/oxycash_xxx.json
///      - 200 → return Loaded(data)
///      - 404 → PUT local data, return Pushed
///   4. PUT backup (called separately via dav_backup_upload after a successful load/save)
pub fn dav_connect(profile: &Profile, local_json: &str, cfg: &Config) -> ConnectResult {
    let client = match make_client() {
        Ok(c)  => c,
        Err(e) => return ConnectResult::Failed(format!("client: {}", e)),
    };

    let data_url   = match dav_full_url(profile)    { Some(u) => u, None => return ConnectResult::Failed("url/user/pass manquant".into()) };
    let dir        = match dir_url(profile)          { Some(u) => u, None => return ConnectResult::Failed("url invalide".into()) };
    let backup_dir = match backup_dir_url(profile)   { Some(u) => u, None => return ConnectResult::Failed("url invalide".into()) };
    let auth       = auth_header(&profile.dav_user, &profile.dav_pass);

    // Step 1: oxycash_config/
    if let Err(e) = ensure_dir(&client, &dir, &auth, cfg) {
        return ConnectResult::Failed(format!("étape 1 (oxycash_config/): {}", e));
    }

    // Step 2: oxycash_config/backup/
    if let Err(e) = ensure_dir(&client, &backup_dir, &auth, cfg) {
        return ConnectResult::Failed(format!("étape 2 (backup/): {}", e));
    }

    // Step 3: GET data file
    log_dav(cfg, &format!("GET {}", data_url));
    let resp = match client.get(&data_url).header("Authorization", &auth).send() {
        Ok(r)  => r,
        Err(e) => return ConnectResult::Failed(format!("GET err: {}", e)),
    };
    let status = resp.status().as_u16();
    log_dav(cfg, &format!("GET status={}", status));

    match status {
        200 => {
            let text = match resp.text() {
                Ok(t)  => t,
                Err(e) => return ConnectResult::Failed(format!("GET body err: {}", e)),
            };
            log_dav(cfg, &format!("GET body len={}", text.len()));
            match AppData::from_json(&text) {
                Ok(data) => ConnectResult::Loaded(data),
                Err(e)   => ConnectResult::Failed(format!("parse err: {}", e)),
            }
        }
        404 => {
            // No remote file yet — push local data
            log_dav(cfg, "data file absent, first push");
            match dav_put_raw(&client, &data_url, local_json, &auth, cfg) {
                true  => ConnectResult::Pushed,
                false => ConnectResult::Failed("first push failed".into()),
            }
        }
        s => ConnectResult::Failed(format!("GET HTTP {}", s)),
    }
}

// ── Save ──────────────────────────────────────────────────────────────────────

/// PUT the data file. Ensures both directories exist first.
pub fn dav_save(profile: &Profile, data: &AppData, cfg: &Config) -> bool {
    let client = match make_client() {
        Ok(c)  => c,
        Err(e) => { log_dav(cfg, &format!("client err: {}", e)); return false; }
    };
    let data_url   = match dav_full_url(profile)  { Some(u) => u, None => return false };
    let dir        = match dir_url(profile)        { Some(u) => u, None => return false };
    let backup_dir = match backup_dir_url(profile) { Some(u) => u, None => return false };
    let auth       = auth_header(&profile.dav_user, &profile.dav_pass);

    if let Err(e) = ensure_dir(&client, &dir, &auth, cfg) {
        log_dav(cfg, &format!("ensure oxycash_config/ failed: {}", e)); return false;
    }
    if let Err(e) = ensure_dir(&client, &backup_dir, &auth, cfg) {
        log_dav(cfg, &format!("ensure backup/ failed: {}", e)); return false;
    }

    dav_put_raw(&client, &data_url, &data.to_json(), &auth, cfg)
}

/// Raw PUT of a string body to a URL.
fn dav_put_raw(
    client: &reqwest::blocking::Client,
    url: &str, body: &str, auth: &str, cfg: &Config,
) -> bool {
    log_dav(cfg, &format!("PUT {}", url));
    match client.put(url)
        .header("Authorization", auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body.to_string()).send()
    {
        Ok(r)  => { let s = r.status().as_u16(); log_dav(cfg, &format!("PUT status={}", s)); matches!(s, 200 | 201 | 204) }
        Err(e) => { log_dav(cfg, &format!("PUT err: {}", e)); false }
    }
}

// ── Backup upload ─────────────────────────────────────────────────────────────

/// PUT a timestamped backup into oxycash_config/backup/.
/// No retention limit — users manage their own backups.
pub fn dav_backup_upload(profile: &Profile, slug: &str, json: &str, cfg: &Config) {
    let base = match dir_url(profile) { Some(u) => u, None => return };
    let ts   = chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string();
    let url  = format!("{}backup/oxycash_{}_{}.json", base, slug, ts);
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    if let Ok(client) = make_client() {
        log_dav(cfg, &format!("BACKUP PUT {}", url));
        let _ = client.put(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(json.to_string()).send();
    }
}

// ── Sync marker ───────────────────────────────────────────────────────────────

pub fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_secs()
}

/// Read the `ts` field from the remote sync-marker file; returns 0 on any error.
pub fn dav_read_marker(profile: &Profile) -> u64 {
    let url    = match dav_marker_url(profile) { Some(u) => u, None => return 0 };
    let client = match make_client()           { Ok(c) => c,  Err(_) => return 0 };
    let auth   = auth_header(&profile.dav_user, &profile.dav_pass);
    let resp   = match client.get(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() => r,
        _ => return 0,
    };
    let text = match resp.text() { Ok(t) => t, Err(_) => return 0 };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok().and_then(|v| v["ts"].as_u64()).unwrap_or(0)
}

/// PUT a sync-marker file with the given timestamp.
pub fn dav_write_marker(profile: &Profile, ts: u64) -> bool {
    let url    = match dav_marker_url(profile) { Some(u) => u, None => return false };
    let client = match make_client()           { Ok(c) => c,  Err(_) => return false };
    let auth   = auth_header(&profile.dav_user, &profile.dav_pass);
    let body   = format!("{{\"ts\":{},\"app\":\"Oxycash\",\"profile\":\"{}\"}}", ts, profile.slug);
    match client.put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body).send()
    {
        Ok(r) => matches!(r.status().as_u16(), 200 | 201 | 204),
        Err(_) => false,
    }
}
