// oxycash-rs - storage.rs
// Storage orchestrator: load (DAV-first with marker arbitration), save (local + async DAV),
// profile management, import/export/reset, config setters.
use std::collections::HashSet;

use crate::config::{
    Config, Profile,
    backup_dir, base_dir, local_data_file,
    load_config, save_config, slugify, backup_local,
};
use crate::webdav::{
    backup_dav_upload, dav_load, dav_read_marker, dav_save, dav_test_http,
    dav_write_marker, make_client, now_ts,
};
use crate::model::{AppData, apply_recurring};

// ── SyncStatus ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStatus { Dav, DavError, Local }

// ── Storage ───────────────────────────────────────────────────────────────────

pub struct Storage {
    pub cfg:    Config,
    pub data:   AppData,
    pub dav_ok: bool,
}

impl Storage {
    pub fn new() -> Self {
        Self { cfg: load_config(), data: AppData::default(), dav_ok: false }
    }

    // ── Profile accessors ─────────────────────────────────────────────────────

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
        if self.cfg.active == slug { self.cfg.active = self.cfg.profiles[0].slug.clone(); }
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

    // ── DAV test ──────────────────────────────────────────────────────────────

    pub fn test_dav(&self) -> (bool, String) {
        match make_client() {
            Ok(c) => dav_test_http(self.active_profile(), c),
            Err(e) => (false, e),
        }
    }

    pub fn test_dav2(&self) -> (bool, String) {
        let prof = self.active_profile();
        if !prof.has_dav2() { return (false, "WebDAV secondaire non configuré".into()); }
        match make_client() {
            Ok(c) => dav_test_http(&prof.as_dav2_profile(), c),
            Err(e) => (false, e),
        }
    }

    // ── Load ──────────────────────────────────────────────────────────────────

    /// Load data for the active profile.
    /// Strategy: if any DAV is configured, pick the source with the most-recent sync marker,
    /// fall back to local file, fall back to empty data.
    pub fn load(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();
        let dav1 = prof.has_dav();
        let dav2 = prof.has_dav2();

        if dav1 || dav2 {
            let ts1 = if dav1 { dav_read_marker(&prof) } else { 0 };
            let ts2 = if dav2 { dav_read_marker(&prof.as_dav2_profile()) } else { 0 };
            let p2 = prof.as_dav2_profile();

            // Prefer the source with the newer marker; fall back to DAV1
            let ordered: Vec<&Profile> = if ts2 > ts1 && dav2 { vec![&p2, &prof] }
                                         else if dav1          { vec![&prof, &p2] }
                                         else                  { vec![&p2] };

            for source in ordered.into_iter().filter(|p| crate::webdav::dav_full_url(p).is_some()) {
                if let Some(mut app) = dav_load(source, &self.cfg) {
                    apply_recurring(&mut app);
                    let dir = base_dir(&self.cfg);
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(local_data_file(&self.cfg, &slug), app.to_json());
                    self.data = app;
                    self.dav_ok = true;
                    return SyncStatus::Dav;
                } else {
                    // 404 — remote directory may be missing; ensure it and push local data
                    let client = match make_client() { Ok(c) => c, Err(_) => continue };
                    if crate::webdav::dav_ensure_dir(source, &client, &self.cfg).is_ok() {
                        let lf = local_data_file(&self.cfg, &slug);
                        let data_to_push = if lf.exists() {
                            std::fs::read_to_string(&lf).ok()
                                .and_then(|t| AppData::from_json(&t).ok())
                                .unwrap_or_else(AppData::new_empty)
                        } else {
                            AppData::new_empty()
                        };
                        if dav_save(source, &data_to_push, &self.cfg) {
                            crate::webdav::log_dav(&self.cfg, "first push OK — connected");
                            apply_recurring(&mut self.data);
                            self.dav_ok = true;
                            return SyncStatus::Dav;
                        }
                    }
                }
            }
        }

        // DAV unavailable — load from local file
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

        // No local file either — start fresh
        self.data = AppData::new_empty();
        apply_recurring(&mut self.data);
        let dir = base_dir(&self.cfg);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(local_data_file(&self.cfg, &slug), self.data.to_json());
        SyncStatus::Local
    }

    // ── Save ──────────────────────────────────────────────────────────────────

    /// Save data for the active profile.
    /// Always writes locally first; DAV upload happens asynchronously if configured.
    pub fn save(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();
        let json = self.data.to_json();

        let dir = base_dir(&self.cfg);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(local_data_file(&self.cfg, &slug), &json);

        let dav1 = prof.has_dav();
        let dav2 = prof.has_dav2();

        if dav1 || dav2 {
            let ts          = now_ts();
            let json_clone  = json.clone();
            let prof_clone  = prof.clone();
            let backup_dav  = self.cfg.backup_webdav;
            let slug_clone  = slug.clone();
            let cfg_clone   = self.cfg.clone();

            std::thread::spawn(move || {
                if dav1 {
                    if dav_save(&prof_clone, &AppData::from_json(&json_clone).unwrap_or_default(), &cfg_clone) {
                        dav_write_marker(&prof_clone, ts);
                        if backup_dav { backup_dav_upload(&prof_clone, &slug_clone, &json_clone); }
                    }
                }
                if dav2 {
                    let p2 = prof_clone.as_dav2_profile();
                    if dav_save(&p2, &AppData::from_json(&json_clone).unwrap_or_default(), &cfg_clone) {
                        dav_write_marker(&p2, ts);
                        if backup_dav { backup_dav_upload(&p2, &slug_clone, &json_clone); }
                    }
                }
            });

            self.dav_ok = true;
            return SyncStatus::Dav;
        }

        self.dav_ok = false;
        SyncStatus::Local
    }

    // ── Exit backup ───────────────────────────────────────────────────────────

    /// Write a local backup and optionally a DAV backup on application close.
    pub fn backup_on_exit(&self) {
        let slug = self.active_profile().slug.clone();
        let json = self.data.to_json();
        backup_local(&self.cfg, &slug, &json);
        if self.cfg.backup_webdav {
            let prof  = self.active_profile().clone();
            let json2 = json.clone();
            let slug2 = slug.clone();
            std::thread::spawn(move || {
                if prof.has_dav()  { backup_dav_upload(&prof, &slug2, &json2); }
                if prof.has_dav2() { backup_dav_upload(&prof.as_dav2_profile(), &slug2, &json2); }
            });
        }
    }

    // ── Status ────────────────────────────────────────────────────────────────

    pub fn status(&self) -> SyncStatus {
        if self.dav_ok { return SyncStatus::Dav; }
        let prof = self.active_profile();
        if prof.has_dav() || prof.has_dav2() { return SyncStatus::DavError; }
        SyncStatus::Local
    }

    // ── Import / Export / Reset ───────────────────────────────────────────────

    pub fn export_json(&self) -> String { self.data.to_json() }

    pub fn import_json(&mut self, raw: &str) -> bool {
        match AppData::from_json(raw) {
            Ok(app) if !app.months.is_empty() => { self.data = app; self.save(); true }
            _ => false,
        }
    }

    pub fn reset(&mut self) { self.data = AppData::new_empty(); self.save(); }

    // ── Config setters ────────────────────────────────────────────────────────

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

    pub fn set_backup_local(&mut self, val: bool)  { self.cfg.backup_local  = val; save_config(&self.cfg); }
    pub fn set_backup_webdav(&mut self, val: bool) { self.cfg.backup_webdav = val; save_config(&self.cfg); }

    // ── Display helpers ───────────────────────────────────────────────────────

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
