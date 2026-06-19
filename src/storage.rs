// oxycash-rs - storage.rs
// Mapping de core/storage.py
// WebDAV via reqwest (rustls, pas de native-tls)
use std::collections::HashSet;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::model::{AppData, Line, Payment, apply_recurring};

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
    // --- WebDAV secondaire (optionnel)
    #[serde(default)]
    pub dav2_url: String,
    #[serde(default)]
    pub dav2_user: String,
    #[serde(default)]
    pub dav2_pass: String,
    #[serde(default)]
    pub dav2_enabled: bool,
}

impl Profile {
    pub fn default_profile() -> Self {
        Self {
            name: "Default".into(),
            slug: "default".into(),
            dav_url: String::new(),
            dav_user: String::new(),
            dav_pass: String::new(),
            dav2_url: String::new(),
            dav2_user: String::new(),
            dav2_pass: String::new(),
            dav2_enabled: false,
        }
    }

    /// Retourne un Profile représentant le WebDAV secondaire comme s'il était primaire,
    /// pour pouvoir réutiliser toutes les fonctions dav_* existantes.
    pub fn as_dav2_profile(&self) -> Profile {
        Profile {
            name: self.name.clone(),
            slug: self.slug.clone(),
            dav_url: self.dav2_url.clone(),
            dav_user: self.dav2_user.clone(),
            dav_pass: self.dav2_pass.clone(),
            dav2_url: String::new(),
            dav2_user: String::new(),
            dav2_pass: String::new(),
            dav2_enabled: false,
        }
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
    pub profiles: Vec<Profile>,
    #[serde(default = "default_slug")]
    pub active: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub font_scale: i32,
    #[serde(default = "default_currency")]
    pub currency: String,
    /// "data" = dans ~/.oxycash/, "exe" = à côté de l'exécutable
    #[serde(default = "default_hash_storage")]
    pub hash_storage: String,
}

fn default_slug() -> String         { "default".into() }
fn default_lang() -> String         { "en".into() }
fn default_currency() -> String     { "CHF".into() }
fn default_hash_storage() -> String { "data".into() }

impl Default for Config {
    fn default() -> Self {
        Self {
            profiles: vec![Profile::default_profile()],
            active: "default".into(),
            lang: "en".into(),
            font_scale: 0,
            currency: "CHF".into(),
            hash_storage: "data".into(),
        }
    }
}

// --- Paths
#[cfg(target_os = "android")]
static ANDROID_DATA_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_android_data_dir(path: PathBuf) {
    let _ = ANDROID_DATA_DIR.set(path);
}

fn local_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    if let Some(p) = ANDROID_DATA_DIR.get() {
        return p.clone();
    }
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

fn dav_marker_filename(slug: &str) -> String {
<<<<<<< HEAD
    format!("oxycash_{}.sync.json", slug)
=======
    format!("oxycash_{}.oxysync", slug)
>>>>>>> 2d6172fe777cfcd39ca7e4ee67ffae6751ab6613
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

// --- Hash storage
fn hash_file_path(hash_storage: &str) -> PathBuf {
    if hash_storage == "exe" {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        exe.parent().unwrap_or(std::path::Path::new("."))
            .join("import_hashes.json")
    } else {
        local_dir().join("import_hashes.json")
    }
}

fn load_hashes(hash_storage: &str) -> HashSet<u64> {
    let path = hash_file_path(hash_storage);
    if path.exists() {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<Vec<u64>>(&text) {
                return v.into_iter().collect();
            }
        }
    }
    HashSet::new()
}

fn save_hashes(hash_storage: &str, hashes: &HashSet<u64>) {
    let path = hash_file_path(hash_storage);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let v: Vec<u64> = hashes.iter().copied().collect();
    if let Ok(json) = serde_json::to_string(&v) {
        let _ = std::fs::write(path, json);
    }
}

/// Hash déterministe d'une transaction : date + libellé + montant (arrondi au centime)
fn transaction_hash(date: &str, label: &str, amount: f64) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    date.hash(&mut h);
    label.to_lowercase().hash(&mut h);
    // montant en centimes pour éviter les flottants
    ((amount * 100.0).round() as i64).hash(&mut h);
    h.finish()
}

/// Détermine la section cible d'une transaction importée
fn route_transaction(label: &str, amount: f64) -> &'static str {
    if amount > 0.0 {
        return "revenus";
    }
    let up = label.to_uppercase();
    if up.contains("ATM") || up.contains("RETRAIT") || up.contains("CASH")
        || up.contains("WITHDRAWAL") || up.contains("DISTRIBUTEUR")
    {
        return "retraits";
    }
    "variables"
}

// --- Parsing CSV bancaire
// Formats supportés :
//   date;libellé;montant
//   date;libellé;débit;crédit
//   date,description,amount
//   date,description,debit,credit
pub fn parse_csv(raw: &str) -> Vec<(String, String, f64)> {
    let sep = if raw.contains(';') { ';' } else { ',' };
    let mut results = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }

        let cols: Vec<&str> = line.splitn(5, sep).map(|c| c.trim().trim_matches('"')).collect();
        if cols.len() < 3 { continue; }

        // Colonne date : on accepte YYYY-MM-DD, DD.MM.YYYY, DD/MM/YYYY
        let date = normalize_date(cols[0]);
        if date.is_none() { continue; } // ligne d'en-tête ou non parsable
        let date = date.unwrap();

        let label = cols[1].to_string();

        let amount: f64 = if cols.len() >= 4 {
            // format débit/crédit séparés
            let debit  = parse_amount(cols[2]);
            let credit = parse_amount(cols[3]);
            credit - debit  // positif = crédit, négatif = débit
        } else {
            match parse_amount_signed(cols[2]) {
                Some(v) => v,
                None => continue,
            }
        };

        results.push((date, label, amount));
    }
    results
}

/// Normalise une date vers YYYY-MM-DD, retourne None si non reconnue
fn normalize_date(s: &str) -> Option<String> {
    let s = s.trim().trim_matches('"');
    // YYYY-MM-DD
    if s.len() == 10 && s.chars().nth(4) == Some('-') {
        return Some(s.to_string());
    }
    // DD.MM.YYYY ou DD/MM/YYYY
    let sep = if s.contains('.') { '.' } else if s.contains('/') { '/' } else { return None; };
    let parts: Vec<&str> = s.splitn(3, sep).collect();
    if parts.len() == 3 && parts[2].len() == 4 {
        return Some(format!("{}-{}-{}", parts[2], parts[1], parts[0]));
    }
    None
}

fn parse_amount(s: &str) -> f64 {
    let clean: String = s.chars().filter(|&c| c.is_ascii_digit() || c == '.' || c == ',').collect();
    let clean = clean.replace(',', ".");
    clean.parse::<f64>().unwrap_or(0.0)
}

fn parse_amount_signed(s: &str) -> Option<f64> {
    let s = s.trim().trim_matches('"');
    if s.is_empty() { return None; }
    let negative = s.starts_with('-');
    let clean: String = s.chars().filter(|&c| c.is_ascii_digit() || c == '.' || c == ',').collect();
    if clean.is_empty() { return None; }
    let clean = clean.replace(',', ".");
    let v = clean.parse::<f64>().ok()?;
    Some(if negative { -v } else { v })
}

// --- Parsing OFX/QFX
// Format SGML-like : balises <TAG>valeur sans fermeture pour les champs primitifs
pub fn parse_ofx(raw: &str) -> Vec<(String, String, f64)> {
    let mut results = Vec::new();
    let mut in_stmttrn = false;
    let mut date   = String::new();
    let mut label  = String::new();
    let mut amount = 0.0_f64;
    let mut amount_set = false;

    for line in raw.lines() {
        let line = line.trim();
        if line.eq_ignore_ascii_case("<stmttrn>") {
            in_stmttrn = true;
            date.clear(); label.clear(); amount = 0.0; amount_set = false;
            continue;
        }
        if line.eq_ignore_ascii_case("</stmttrn>") {
            if in_stmttrn && !date.is_empty() && amount_set {
                results.push((date.clone(), label.clone(), amount));
            }
            in_stmttrn = false;
            continue;
        }
        if !in_stmttrn { continue; }

        if let Some(v) = tag_value(line, "DTPOSTED") {
            // Format OFX : YYYYMMDD ou YYYYMMDDHHMMSS
            date = ofx_date(&v);
        } else if let Some(v) = tag_value(line, "TRNAMT") {
            if let Ok(f) = v.parse::<f64>() {
                amount = f;
                amount_set = true;
            }
        } else if let Some(v) = tag_value(line, "NAME") {
            label = v;
        } else if let Some(v) = tag_value(line, "MEMO") {
            if label.is_empty() { label = v; }
        }
    }
    results
}

fn tag_value(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    if line.to_uppercase().starts_with(&open.to_uppercase()) {
        let val = &line[open.len()..];
        // strip closing tag if present
        let val = if let Some(close) = val.to_uppercase().find(&format!("</{}>", tag.to_uppercase())) {
            &val[..close]
        } else {
            val
        };
        return Some(val.trim().to_string());
    }
    None
}

fn ofx_date(s: &str) -> String {
    // YYYYMMDD[HHMMSS...]
    if s.len() >= 8 {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s.to_string()
    }
}

// --- Export CSV (registre du mois courant)
// Colonnes : date,section,libellé,montant
pub fn build_csv_export(data: &AppData, month_key: &str, currency: &str) -> String {
    let mut out = format!("date,section,libellé,montant ({})\n", currency);
    let month = match data.months.get(month_key) {
        Some(m) => m,
        None => return out,
    };
    let sections = [
        ("Revenus",   &month.revenus),
        ("Retraits",  &month.retraits),
        ("Fixes",     &month.fixes),
        ("Variables", &month.variables),
    ];
    for (sec_name, lines) in &sections {
        for line in *lines {
            for pay in &line.payments {
                out.push_str(&format!(
                    "{},{},{},{:.2}\n",
                    pay.date, sec_name,
                    line.name.replace(',', " "),
                    pay.amount
                ));
            }
        }
    }
    out
}

// --- Export OFX (registre du mois courant)
pub fn build_ofx_export(data: &AppData, month_key: &str) -> String {
    let month = match data.months.get(month_key) {
        Some(m) => m,
        None => return String::new(),
    };

    let mut transactions = String::new();
    let sections = [
        ("Revenus",   &month.revenus,   1_i32),   // CREDIT
        ("Retraits",  &month.retraits,  -1_i32),  // DEBIT
        ("Fixes",     &month.fixes,     -1_i32),
        ("Variables", &month.variables, -1_i32),
    ];
    let mut fitid = 1u32;
    for (_sec_name, lines, sign) in &sections {
        for line in *lines {
            for pay in &line.payments {
                let trntype = if *sign > 0 { "CREDIT" } else { "DEBIT" };
                let amount  = pay.amount * (*sign as f64);
                let dt = pay.date.replace('-', "") + "120000"; // YYYYMMDDHHMMSS
                transactions.push_str(&format!(
                    "<STMTTRN>\n\
                     <TRNTYPE>{}\n\
                     <DTPOSTED>{}\n\
                     <TRNAMT>{:.2}\n\
                     <FITID>{}\n\
                     <NAME>{}\n\
                     </STMTTRN>\n",
                    trntype, dt, amount, fitid,
                    line.name.replace('<', "").replace('>', "")
                ));
                fitid += 1;
            }
        }
    }

    let now = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    format!(
        "OFXHEADER:100\nDATA:OFXSGML\nVERSION:102\nSECURITY:NONE\n\
         ENCODING:UTF-8\nCHARSET:1252\nCOMPRESSION:NONE\n\
         OLDFILEUID:NONE\nNEWFILEUID:NONE\n\n\
         <OFX>\n\
         <BANKMSGSRSV1>\n\
         <STMTTRNRS>\n\
         <TRNUID>1\n\
         <STATUS><CODE>0<SEVERITY>INFO</STATUS>\n\
         <STMTRS>\n\
         <CURDEF>CHF\n\
         <BANKTRANLIST>\n\
         <DTSTART>{now}\n\
         <DTEND>{now}\n\
         {transactions}\
         </BANKTRANLIST>\n\
         </STMTRS>\n\
         </STMTTRNRS>\n\
         </BANKMSGSRSV1>\n\
         </OFX>\n"
    )
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

fn dav_marker_url(profile: &Profile) -> Option<String> {
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
        let root_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
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
            while let Some(s) = src.source() {
                msg.push_str(&format!(" | {}", s));
                src = s;
            }
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
        Some(u) => u,
        None => return false,
    };
    let client = match make_client() {
        Ok(c) => c,
        Err(_) => return false,
    };
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

// --- Marqueur de sync
/// Retourne le timestamp Unix en secondes depuis l'epoch.
fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

<<<<<<< HEAD
/// Lit le marqueur .sync.json d'un WebDAV. Retourne 0 si absent ou illisible.
=======
/// Lit le marqueur .oxysync d'un WebDAV. Retourne 0 si absent ou illisible.
>>>>>>> 2d6172fe777cfcd39ca7e4ee67ffae6751ab6613
fn dav_read_marker(profile: &Profile) -> u64 {
    let url = match dav_marker_url(profile) {
        Some(u) => u,
        None => return 0,
    };
    let client = match make_client() {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
    let resp = match client.get(&url).header("Authorization", &auth).send() {
        Ok(r) if r.status().is_success() => r,
        _ => return 0,
    };
<<<<<<< HEAD
    let text = match resp.text() {
        Ok(t) => t,
        Err(_) => return 0,
    };
    // Format JSON : { "ts": 1750123456, "app": "Oxycash", "profile": "..." }
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v["ts"].as_u64())
        .unwrap_or(0)
}

/// Écrit le marqueur .sync.json sur un WebDAV.
=======
    resp.text().ok()
        .and_then(|t| t.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// Écrit le marqueur .oxysync sur un WebDAV.
>>>>>>> 2d6172fe777cfcd39ca7e4ee67ffae6751ab6613
fn dav_write_marker(profile: &Profile, ts: u64) -> bool {
    let url = match dav_marker_url(profile) {
        Some(u) => u,
        None => return false,
    };
    let client = match make_client() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let auth = auth_header(&profile.dav_user, &profile.dav_pass);
<<<<<<< HEAD
    let body = format!(
        "{{\"ts\":{},\"app\":\"Oxycash\",\"profile\":\"{}\"}}",
        ts, profile.slug
    );
    match client
        .put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(body)
=======
    match client
        .put(&url)
        .header("Authorization", &auth)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(ts.to_string())
>>>>>>> 2d6172fe777cfcd39ca7e4ee67ffae6751ab6613
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
        let existing: HashSet<_> =
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
            dav2_url: String::new(),
            dav2_user: String::new(),
            dav2_pass: String::new(),
            dav2_enabled: false,
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

    pub fn save_profile_dav2(&mut self, slug: &str, url: &str, user: &str, pw: &str, enabled: bool) {
        if let Some(p) = self.cfg.profiles.iter_mut().find(|p| p.slug == slug) {
            p.dav2_url     = url.to_string();
            p.dav2_user    = user.to_string();
            p.dav2_pass    = pw.to_string();
            p.dav2_enabled = enabled;
        }
        save_config(&self.cfg);
    }

    pub fn test_dav2(&self) -> (bool, String) {
        let prof = self.active_profile();
        if !prof.has_dav2() {
            return (false, "WebDAV secondaire non configuré".into());
        }
        match make_client() {
            Ok(c) => dav_test_http(&prof.as_dav2_profile(), c),
            Err(e) => (false, e),
        }
    }

    // --- Load / Save
    pub fn load(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();

        let dav1_configured = dav_full_url(&prof).is_some();
        let dav2_configured = prof.has_dav2();

        if dav1_configured || dav2_configured {
            // Lire les marqueurs pour choisir la source la plus récente
            let ts1 = if dav1_configured { dav_read_marker(&prof) } else { 0 };
            let ts2 = if dav2_configured { dav_read_marker(&prof.as_dav2_profile()) } else { 0 };

            // Essayer la source la plus récente en premier, fallback sur l'autre
            let sources: Vec<&Profile>;
            let p2 = prof.as_dav2_profile();
            let ordered: Vec<&Profile> = if ts2 > ts1 && dav2_configured {
                vec![&p2, &prof]
            } else if dav1_configured {
                vec![&prof, &p2]
            } else {
                vec![&p2]
            };

            // Filtrer selon ce qui est réellement configuré
            sources = ordered.into_iter().filter(|p| dav_full_url(p).is_some()).collect();

            for source in sources {
                if let Some(mut app) = dav_load(source) {
                    apply_recurring(&mut app);
                    let dir = local_dir();
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(local_data_file(&slug), app.to_json());
                    self.data   = app;
                    self.dav_ok = true;
                    return SyncStatus::Dav;
                }
            }
        }

        self.dav_ok = false;

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
        let json = self.data.to_json();
        let _ = std::fs::write(local_data_file(&slug), &json);

        let dav1_configured = dav_full_url(&prof).is_some();
        let dav2_configured = prof.has_dav2();

        if dav1_configured || dav2_configured {
            let ts = now_ts();
            let data_clone  = self.data.clone();
            let prof_clone  = prof.clone();

            std::thread::spawn(move || {
                // Push DAV1
                if dav1_configured {
                    if dav_save(&prof_clone, &data_clone) {
                        dav_write_marker(&prof_clone, ts);
                    }
                }
                // Push DAV2
                if dav2_configured {
                    let p2 = prof_clone.as_dav2_profile();
                    if dav_save(&p2, &data_clone) {
                        dav_write_marker(&p2, ts);
                    }
                }
            });

            self.dav_ok = true;
            return SyncStatus::Dav;
        }

        self.dav_ok = false;
        SyncStatus::Local
    }

    pub fn status(&self) -> SyncStatus {
        if self.dav_ok { return SyncStatus::Dav; }
        let prof = self.active_profile();
        if dav_full_url(prof).is_some() || prof.has_dav2() {
            return SyncStatus::DavError;
        }
        SyncStatus::Local
    }

    pub fn test_dav(&self) -> (bool, String) {
        match make_client() {
            Ok(c) => dav_test_http(self.active_profile(), c),
            Err(e) => (false, e),
        }
    }

    // --- Import / Export JSON (profil complet)
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

    // --- Import relevé CSV
    /// Retourne (insérées, doublons_ignorés)
    pub fn import_statement_csv(&mut self, raw: &str, month_key: &str) -> (usize, usize) {
        let transactions = parse_csv(raw);
        self.insert_transactions(transactions, month_key)
    }

    // --- Import relevé OFX
    /// Retourne (insérées, doublons_ignorés)
    pub fn import_statement_ofx(&mut self, raw: &str, month_key: &str) -> (usize, usize) {
        let transactions = parse_ofx(raw);
        self.insert_transactions(transactions, month_key)
    }

    /// Insère une liste de (date, label, amount) dans le mois donné.
    /// Déduplique via hashes. Retourne (insérées, doublons_ignorés).
    fn insert_transactions(
        &mut self,
        transactions: Vec<(String, String, f64)>,
        month_key: &str,
    ) -> (usize, usize) {
        let hash_storage = self.cfg.hash_storage.clone();
        let mut hashes = load_hashes(&hash_storage);
        let mut inserted = 0usize;
        let mut skipped  = 0usize;

        let month = match self.data.months.get_mut(month_key) {
            Some(m) => m,
            None => return (0, 0),
        };

        for (date, label, amount) in transactions {
            let h = transaction_hash(&date, &label, amount);
            if hashes.contains(&h) {
                skipped += 1;
                continue;
            }
            hashes.insert(h);

            let section = route_transaction(&label, amount);
            let abs_amount = amount.abs();

            let lines: &mut Vec<Line> = match section {
                "revenus"  => &mut month.revenus,
                "retraits" => &mut month.retraits,
                _          => &mut month.variables,
            };

            // Cherche une ligne existante avec le même nom, sinon crée
            if let Some(line) = lines.iter_mut().find(|l| l.name == label) {
                line.payments.push(Payment { date, amount: abs_amount });
                // Met à jour le budget si non encore défini
                if line.banque == 0.0 && line.cash == 0.0 {
                    line.banque = abs_amount;
                }
            } else {
                let mut new_line = Line::new(&label);
                new_line.banque = abs_amount;
                new_line.payments.push(Payment { date, amount: abs_amount });
                lines.push(new_line);
            }
            inserted += 1;
        }

        save_hashes(&hash_storage, &hashes);
        self.save();
        (inserted, skipped)
    }

    // --- Export CSV (mois courant)
    pub fn export_statement_csv(&self, month_key: &str) -> String {
        build_csv_export(&self.data, month_key, &self.cfg.currency)
    }

    // --- Export OFX (mois courant)
    pub fn export_statement_ofx(&self, month_key: &str) -> String {
        build_ofx_export(&self.data, month_key)
    }

    // --- Préférences
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

    pub fn set_hash_storage(&mut self, mode: &str) {
        self.cfg.hash_storage = mode.to_string();
        save_config(&self.cfg);
    }

    pub fn hash_file_location(&self) -> String {
        hash_file_path(&self.cfg.hash_storage)
            .to_string_lossy()
            .to_string()
    }
}
