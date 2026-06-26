// oxycash-rs - webdav.rs
// WebDAV HTTP operations. Uses DavProfile (slug + credentials) instead of the
// old per-profile struct so credentials stay global in Config.
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::config::{Config, DavProfile, base_dir, dav_filename, dav_marker_filename};
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

fn normalise_base(url: &str) -> String {
    let base = if url.ends_with('/') { url.to_string() } else { format!("{}/", url) };
    if base.starts_with("http") { base } else { format!("https://{}", base) }
}

fn root_url(dp: &DavProfile) -> Option<String> {
    let url = dp.dav_url.trim();
    if url.is_empty() || dp.dav_user.trim().is_empty() || dp.dav_pass.trim().is_empty() {
        return None;
    }
    Some(normalise_base(url))
}

fn dir_url(dp: &DavProfile) -> Option<String> {
    root_url(dp).map(|r| format!("{}oxycash_config/", r))
}

fn backup_dir_url(dp: &DavProfile) -> Option<String> {
    dir_url(dp).map(|d| format!("{}backup/", d))
}

pub fn dav_full_url(dp: &DavProfile) -> Option<String> {
    dir_url(dp).map(|d| format!("{}{}", d, dav_filename(&dp.slug)))
}

fn dav_marker_url(dp: &DavProfile) -> Option<String> {
    dir_url(dp).map(|d| format!("{}{}", d, dav_marker_filename(&dp.slug)))
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

pub fn dav_test_http(dp: &DavProfile, client: reqwest::blocking::Client) -> (bool, String) {
    let url = match dav_full_url(dp) {
        Some(u) => u,
        None    => return (false, "url/user/pass manquant".into()),
    };
    let auth = auth_header(&dp.dav_user, &dp.dav_pass);
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

pub enum ConnectResult {
    Loaded(AppData),
    Pushed,
    Failed(String),
}

pub fn dav_connect(dp: &DavProfile, local_json: &str, cfg: &Config) -> ConnectResult {
    let client = match make_client() {
        Ok(c)  => c,
        Err(e) => return ConnectResult::Failed(format!("client: {}", e)),
    };

    let data_url   = match dav_full_url(dp)    { Some(u) => u, None => return ConnectResult::Failed("url/user/pass manquant".into()) };
    let dir        = match dir_url(dp)          { Some(u) => u, None => return ConnectResult::Failed("url invalide".into()) };
    let backup_dir = match backup_dir_url(dp)   { Some(u) => u, None => return ConnectResult::Failed("url invalide".into()) };
    let auth       = auth_header(&dp.dav_user, &dp.dav_pass);

    if let Err(e) = ensure_dir(&client, &dir, &auth, cfg) {
        return ConnectResult::Failed(format!("étape 1 (oxycash_config/): {}", e));
    }
    if let Err(e) = ensure_dir(&client, &backup_dir, &auth, cfg) {
        return ConnectResult::Failed(format!("étape 2 (backup/): {}", e));
    }

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

pub fn dav_save(dp: &DavProfile, data: &AppData, cfg: &Config) -> bool {
    let client = match make_client() {
        Ok(c)  => c,
        Err(e) => { log_dav(cfg, &format!("client err: {}", e)); return false; }
    };
    let data_url   = match dav_full_url(dp)    { Some(u) => u, None => return false };
    let dir        = match dir_url(dp)          { Some(u) => u, None => return false };
    let backup_dir = match backup_dir_url(dp)   { Some(u) => u, None => return false };
    let auth       = auth_header(&dp.dav_user, &dp.dav_pass);

    if let Err(e) = ensure_dir(&client, &dir, &auth, cfg) {
        log_dav(cfg, &format!("ensure oxycash_config/ failed: {}", e)); return false;
    }
    if let Err(e) = ensure_dir(&client, &backup_dir, &auth, cfg) {
        log_dav(cfg, &format!("ensure backup/ failed: {}", e)); return false;
    }
    dav_put_raw(&client, &data_url, &data.to_json(), &auth, cfg)
}

fn dav_put_raw(client: &reqwest::blocking::Client, url: &str, body: &str, auth: &str, cfg: &Config) -> bool {
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

// ── Delete ────────────────────────────────────────────────────────────────────

pub fn dav_delete(dp: &DavProfile, url: &str, cfg: &Config) -> bool {
    let client = match make_client() {
        Ok(c)  => c,
        Err(e) => { log_dav(cfg, &format!("dav_delete: client err: {}", e)); return false; }
    };
    let auth = auth_header(&dp.dav_user, &dp.dav_pass);
    log_dav(cfg, &format!("DELETE {}", url));
    match client.delete(url).header("Authorization", &auth).send() {
        Ok(r)  => { let s = r.status().as_u16(); log_dav(cfg, &format!("DELETE status={}", s)); matches!(s, 200 | 204) }
        Err(e) => { log_dav(cfg, &format!("DELETE err: {}", e)); false }
    }
}

// ── Backup upload ─────────────────────────────────────────────────────────────

pub fn dav_backup_upload(dp: &DavProfile, json: &str, cfg: &Config) {
    let base = match dir_url(dp) { Some(u) => u, None => return };
    let ts   = chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string();
    let url  = format!("{}backup/oxycash_{}_{}.json", base, dp.slug, ts);
    let auth = auth_header(&dp.dav_user, &dp.dav_pass);
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

pub fn dav_read_marker(dp: &DavProfile) -> u64 {
    let url    = match dav_marker_url(dp) { Some(u) => u, None => return 0 };
    let client = match make_client()      { Ok(c) => c,  Err(_) => return 0 };
    let auth   = auth_header(&dp.dav_user, &dp.dav_pass);
    let resp   = match client.get(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() => r,
        _ => return 0,
    };
    let text = match resp.text() { Ok(t) => t, Err(_) => return 0 };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("ts").and_then(|t| t.as_u64()))
        .unwrap_or(0)
}

pub fn dav_write_marker(dp: &DavProfile, ts: u64) -> bool {
    let url    = match dav_marker_url(dp) { Some(u) => u, None => return false };
    let client = match make_client()      { Ok(c) => c,  Err(_) => return false };
    let auth   = auth_header(&dp.dav_user, &dp.dav_pass);
    let body   = format!("{{\"ts\":{},\"app\":\"Oxycash\",\"profile\":\"{}\"}}", ts, dp.slug);
    match client.put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body).send()
    {
        Ok(r) => matches!(r.status().as_u16(), 200 | 201 | 204),
        Err(_) => false,
    }
}

// ── Rename (data file + marker) ───────────────────────────────────────────────

/// GET old data+marker files, PUT to new slug URLs, DELETE old files.
pub fn dav_rename(old_dp: &DavProfile, new_slug: &str, cfg: &Config) {
    let client = match make_client() { Ok(c) => c, Err(_) => return };

    let mut new_dp = old_dp.clone();
    new_dp.slug = new_slug.to_string();

    let old_data_url   = match dav_full_url(old_dp)   { Some(u) => u, None => return };
    let new_data_url   = match dav_full_url(&new_dp)   { Some(u) => u, None => return };
    let old_marker_url = match dav_marker_url(old_dp)  { Some(u) => u, None => return };
    let new_marker_url = match dav_marker_url(&new_dp) { Some(u) => u, None => return };
    let auth = auth_header(&old_dp.dav_user, &old_dp.dav_pass);

    // Rename data file
    dav_move_file(&client, &old_data_url, &new_data_url, &auth, cfg);
    // Rename marker file (best-effort, may not exist)
    dav_move_file(&client, &old_marker_url, &new_marker_url, &auth, cfg);
}

fn dav_move_file(
    client: &reqwest::blocking::Client,
    old_url: &str, new_url: &str, auth: &str, cfg: &Config,
) {
    log_dav(cfg, &format!("MOVE {} → {}", old_url, new_url));
    // GET old
    let resp = match client.get(old_url).header("Authorization", auth).send() {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => { log_dav(cfg, &format!("MOVE GET status={} (skipping)", r.status())); return; }
        Err(e) => { log_dav(cfg, &format!("MOVE GET err: {}", e)); return; }
    };
    let body = match resp.text() {
        Ok(t)  => t,
        Err(e) => { log_dav(cfg, &format!("MOVE body err: {}", e)); return; }
    };
    // PUT new
    match client.put(new_url)
        .header("Authorization", auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body).send()
    {
        Ok(r) if matches!(r.status().as_u16(), 200 | 201 | 204) => {}
        Ok(r) => { log_dav(cfg, &format!("MOVE PUT status={} (not deleting old)", r.status())); return; }
        Err(e) => { log_dav(cfg, &format!("MOVE PUT err: {}", e)); return; }
    }
    // DELETE old
    if let Ok(r) = client.delete(old_url).header("Authorization", auth).send() {
        log_dav(cfg, &format!("MOVE DELETE status={}", r.status()));
    }
}
