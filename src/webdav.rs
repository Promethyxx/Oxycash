// oxycash-rs - webdav.rs
// WebDAV HTTP operations: load, save, directory creation, sync marker, backup upload, test.
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

fn normalise_base(url: &str) -> String {
    let base = if url.ends_with('/') { url.to_string() } else { format!("{}/", url) };
    if base.starts_with("http") { base } else { format!("https://{}", base) }
}

pub fn dav_full_url(profile: &Profile) -> Option<String> {
    let url = profile.dav_url.trim();
    if url.is_empty() || profile.dav_user.trim().is_empty() || profile.dav_pass.trim().is_empty() {
        return None;
    }
    Some(format!("{}{}", normalise_base(url), dav_filename(&profile.slug)))
}

fn dav_marker_url(profile: &Profile) -> Option<String> {
    let url = profile.dav_url.trim();
    if url.is_empty() || profile.dav_user.trim().is_empty() || profile.dav_pass.trim().is_empty() {
        return None;
    }
    Some(format!("{}{}", normalise_base(url), dav_marker_filename(&profile.slug)))
}

pub fn auth_header(user: &str, pw: &str) -> String {
    format!("Basic {}", STANDARD.encode(format!("{}:{}", user, pw)))
}

// ── Logging ───────────────────────────────────────────────────────────────────

pub fn log_dav(cfg: &Config, msg: &str) {
    let path = base_dir(cfg).join("dav.log");
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let line = format!("[{}] {}\n", ts, msg);
    let _ = std::fs::create_dir_all(base_dir(cfg));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
}

// ── Connection test ───────────────────────────────────────────────────────────

/// HEAD the target file to verify credentials and reachability.
/// 404 is treated as success (credentials OK, file just doesn't exist yet).
pub fn dav_test_http(profile: &Profile, client: reqwest::blocking::Client) -> (bool, String) {
    let url = match dav_full_url(profile) {
        Some(u) => u,
        None => return (false, "url/user/pass manquant".into()),
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

// ── Directory ensure (PROPFIND / MKCOL) ──────────────────────────────────────

/// Verify the remote directory exists via PROPFIND; create it with MKCOL if absent.
pub fn dav_ensure_dir(profile: &Profile, client: &reqwest::blocking::Client, cfg: &Config) -> Result<(), String> {
    let url = profile.dav_url.trim();
    if url.is_empty() { return Err("url vide".into()); }
    let dir_url = normalise_base(url);
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);

    log_dav(cfg, &format!("PROPFIND {}", dir_url));
    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &dir_url)
        .header("Authorization", &auth)
        .header("Depth", "0")
        .send()
        .map_err(|e| { let m = format!("PROPFIND err: {}", e); log_dav(cfg, &m); m })?;

    let status = resp.status().as_u16();
    log_dav(cfg, &format!("PROPFIND status={}", status));

    match status {
        200 | 207 => { log_dav(cfg, "dir exists"); Ok(()) }
        404 => {
            log_dav(cfg, &format!("MKCOL {}", dir_url));
            let resp2 = client
                .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &dir_url)
                .header("Authorization", &auth)
                .send()
                .map_err(|e| { let m = format!("MKCOL err: {}", e); log_dav(cfg, &m); m })?;
            let s2 = resp2.status().as_u16();
            log_dav(cfg, &format!("MKCOL status={}", s2));
            match s2 {
                200 | 201 | 204 => Ok(()),
                s => Err(format!("MKCOL HTTP {}", s)),
            }
        }
        s => Err(format!("PROPFIND HTTP {}", s)),
    }
}

// ── Load / Save ───────────────────────────────────────────────────────────────

/// GET the data file from WebDAV; returns None on network error or 4xx/5xx.
pub fn dav_load(profile: &Profile, cfg: &Config) -> Option<AppData> {
    let url = dav_full_url(profile)?;
    log_dav(cfg, &format!("GET {}", url));
    let client = make_client().map_err(|e| log_dav(cfg, &format!("client err: {}", e))).ok()?;
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let resp = client.get(&url).header("Authorization", &auth).send()
        .map_err(|e| log_dav(cfg, &format!("GET err: {}", e))).ok()?;
    let status = resp.status().as_u16();
    log_dav(cfg, &format!("GET status={}", status));
    if !resp.status().is_success() { return None; }
    let text = resp.text().map_err(|e| log_dav(cfg, &format!("text err: {}", e))).ok()?;
    log_dav(cfg, &format!("GET body len={}", text.len()));
    AppData::from_json(&text).map_err(|e| log_dav(cfg, &format!("parse err: {}", e))).ok()
}

/// PUT the data file to WebDAV; ensures the remote directory exists first.
pub fn dav_save(profile: &Profile, data: &AppData, cfg: &Config) -> bool {
    let url = match dav_full_url(profile) { Some(u) => u, None => return false };
    let client = match make_client() {
        Ok(c) => c,
        Err(e) => { log_dav(cfg, &format!("client err: {}", e)); return false; }
    };
    if let Err(e) = dav_ensure_dir(profile, &client, cfg) {
        log_dav(cfg, &format!("ensure_dir failed: {}", e));
        return false;
    }
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    log_dav(cfg, &format!("PUT {}", url));
    match client.put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(data.to_json()).send()
    {
        Ok(r) => { let s = r.status().as_u16(); log_dav(cfg, &format!("PUT status={}", s)); matches!(s, 200 | 201 | 204) }
        Err(e) => { log_dav(cfg, &format!("PUT err: {}", e)); false }
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
    let url = match dav_marker_url(profile) { Some(u) => u, None => return 0 };
    let client = match make_client() { Ok(c) => c, Err(_) => return 0 };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let resp = match client.get(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() => r,
        _ => return 0,
    };
    let text = match resp.text() { Ok(t) => t, Err(_) => return 0 };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok().and_then(|v| v["ts"].as_u64()).unwrap_or(0)
}

/// PUT a sync-marker file with the given timestamp.
pub fn dav_write_marker(profile: &Profile, ts: u64) -> bool {
    let url = match dav_marker_url(profile) { Some(u) => u, None => return false };
    let client = match make_client() { Ok(c) => c, Err(_) => return false };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let body = format!("{{\"ts\":{},\"app\":\"Oxycash\",\"profile\":\"{}\"}}", ts, profile.slug);
    match client.put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body).send()
    {
        Ok(r) => matches!(r.status().as_u16(), 200 | 201 | 204),
        Err(_) => false,
    }
}

// ── Backup upload ─────────────────────────────────────────────────────────────

/// PUT a timestamped backup copy into the remote `backup/` subdirectory.
pub fn backup_dav_upload(profile: &Profile, slug: &str, json: &str) {
    let base_url = profile.dav_url.trim();
    if base_url.is_empty() { return; }
    let ts = chrono::Local::now().format("%Y-%m-%d_%H%M%S").to_string();
    let url = format!("{}backup/oxycash_{}_{}.json", normalise_base(base_url), slug, ts);
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    if let Ok(client) = make_client() {
        let _ = client.put(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(json.to_string()).send();
    }
}
