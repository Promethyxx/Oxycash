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
    dav_backup_upload, dav_connect, dav_read_marker, dav_save, dav_test_http,
    dav_write_marker, log_dav, make_client, now_ts, ConnectResult,
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
            .or_else(|| self.cfg.profiles.first())
            .unwrap_or_else(|| {
                log_dav(&self.cfg, "active_profile: profiles list is empty — this should never happen");
                // Safety: load_config always ensures at least one profile exists.
                // If we get here the config is corrupted beyond recovery.
                panic!("no profiles in config");
            })
    }

    pub fn active_profile_mut(&mut self) -> &mut Profile {
        let slug = self.cfg.active.clone();
        if let Some(i) = self.cfg.profiles.iter().position(|p| p.slug == slug) {
            &mut self.cfg.profiles[i]
        } else if !self.cfg.profiles.is_empty() {
            &mut self.cfg.profiles[0]
        } else {
            panic!("no profiles in config");
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
        let lf = local_data_file(&self.cfg, slug);
        if let Err(e) = std::fs::remove_file(&lf) {
            log_dav(&self.cfg, &format!("delete_profile: remove {:?} failed: {}", lf, e));
        }
        if self.cfg.active == slug {
            if let Some(first) = self.cfg.profiles.first() {
                self.cfg.active = first.slug.clone();
            }
        }
        save_config(&self.cfg);
    }

    /// Rename the display name of a profile (slug and data file are unchanged).
    pub fn rename_profile(&mut self, slug: &str, new_name: &str) {
        let new_name = new_name.trim();
        if new_name.is_empty() { return; }
        if let Some(p) = self.cfg.profiles.iter_mut().find(|p| p.slug == slug) {
            p.name = new_name.to_string();
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

    // ── DAV test ──────────────────────────────────────────────────────────────

    pub fn test_dav(&self) -> (bool, String) {
        match make_client() {
            Ok(c)  => dav_test_http(self.active_profile(), c),
            Err(e) => (false, e),
        }
    }

    pub fn test_dav2(&self) -> (bool, String) {
        let prof = self.active_profile();
        if !prof.has_dav2() { return (false, "WebDAV secondaire non configuré".into()); }
        match make_client() {
            Ok(c)  => dav_test_http(&prof.as_dav2_profile(), c),
            Err(e) => (false, e),
        }
    }

    // ── Load ──────────────────────────────────────────────────────────────────

    pub fn load(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();
        let dav1 = prof.has_dav();
        let dav2 = prof.has_dav2();

        if dav1 || dav2 {
            let ts1 = if dav1 { dav_read_marker(&prof) } else { 0 };
            let ts2 = if dav2 { dav_read_marker(&prof.as_dav2_profile()) } else { 0 };
            let p2  = prof.as_dav2_profile();

            let sources: Vec<&Profile> = if ts2 > ts1 && dav2 { vec![&p2, &prof] }
                                         else if dav1          { vec![&prof, &p2] }
                                         else                  { vec![&p2] };

            let lf         = local_data_file(&self.cfg, &slug);
            let local_json = std::fs::read_to_string(&lf).unwrap_or_else(|_| {
                AppData::new_empty().to_json()
            });

            for source in sources.into_iter().filter(|p| p.has_dav()) {
                match dav_connect(source, &local_json, &self.cfg) {
                    ConnectResult::Loaded(mut app) => {
                        apply_recurring(&mut app);
                        let dir = base_dir(&self.cfg);
                        if let Err(e) = std::fs::create_dir_all(&dir) {
                            log_dav(&self.cfg, &format!("load: create_dir {:?} failed: {}", dir, e));
                        }
                        if let Err(e) = std::fs::write(&lf, app.to_json()) {
                            log_dav(&self.cfg, &format!("load: write local mirror {:?} failed: {}", lf, e));
                        }
                        self.data   = app;
                        self.dav_ok = true;
                        return SyncStatus::Dav;
                    }
                    ConnectResult::Pushed => {
                        match std::fs::read_to_string(&lf) {
                            Ok(text) => match AppData::from_json(&text) {
                                Ok(mut app) => { apply_recurring(&mut app); self.data = app; }
                                Err(e) => log_dav(&self.cfg, &format!("load/pushed: parse local failed: {}", e)),
                            },
                            Err(e) => log_dav(&self.cfg, &format!("load/pushed: read local failed: {}", e)),
                        }
                        self.dav_ok = true;
                        return SyncStatus::Dav;
                    }
                    ConnectResult::Failed(e) => {
                        log_dav(&self.cfg, &format!("load: dav_connect failed: {}", e));
                        continue;
                    }
                }
            }
        }

        // DAV unavailable — fall back to local file
        self.dav_ok = false;
        let lf = local_data_file(&self.cfg, &slug);
        if lf.exists() {
            match std::fs::read_to_string(&lf) {
                Ok(text) => match AppData::from_json(&text) {
                    Ok(mut app) => { apply_recurring(&mut app); self.data = app; return SyncStatus::Local; }
                    Err(e) => log_dav(&self.cfg, &format!("load: parse local {:?} failed: {}", lf, e)),
                },
                Err(e) => log_dav(&self.cfg, &format!("load: read local {:?} failed: {}", lf, e)),
            }
        }

        // No usable file — start fresh
        log_dav(&self.cfg, &format!("load: no data found for '{}', starting empty", slug));
        self.data = AppData::new_empty();
        apply_recurring(&mut self.data);
        let dir = base_dir(&self.cfg);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log_dav(&self.cfg, &format!("load: create_dir {:?} failed: {}", dir, e));
        }
        if let Err(e) = std::fs::write(local_data_file(&self.cfg, &slug), self.data.to_json()) {
            log_dav(&self.cfg, &format!("load: write initial file failed: {}", e));
        }
        SyncStatus::Local
    }

    // ── Save ──────────────────────────────────────────────────────────────────

    pub fn save(&mut self) -> SyncStatus {
        let prof = self.active_profile().clone();
        let slug = prof.slug.clone();
        let json = self.data.to_json();

        // Always write local copy first
        let dir = base_dir(&self.cfg);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log_dav(&self.cfg, &format!("save: create_dir {:?} failed: {}", dir, e));
        }
        if let Err(e) = std::fs::write(local_data_file(&self.cfg, &slug), &json) {
            log_dav(&self.cfg, &format!("save: write local failed: {}", e));
        }

        let dav1          = prof.has_dav();
        let dav2          = prof.has_dav2();
        let backup_webdav = self.cfg.backup_webdav;

        if dav1 || dav2 {
            let ts         = now_ts();
            let json_clone = json.clone();
            let prof_clone = prof.clone();
            let slug_clone = slug.clone();
            let cfg_clone  = self.cfg.clone();

            std::thread::spawn(move || {
                if dav1 {
                    match AppData::from_json(&json_clone) {
                        Ok(data) => {
                            if dav_save(&prof_clone, &data, &cfg_clone) {
                                dav_write_marker(&prof_clone, ts);
                                if backup_webdav {
                                    dav_backup_upload(&prof_clone, &slug_clone, &json_clone, &cfg_clone);
                                }
                            }
                        }
                        Err(e) => log_dav(&cfg_clone, &format!("save thread: parse json failed: {}", e)),
                    }
                }
                if dav2 {
                    let p2 = prof_clone.as_dav2_profile();
                    match AppData::from_json(&json_clone) {
                        Ok(data) => {
                            if dav_save(&p2, &data, &cfg_clone) {
                                dav_write_marker(&p2, ts);
                                if backup_webdav {
                                    dav_backup_upload(&p2, &slug_clone, &json_clone, &cfg_clone);
                                }
                            }
                        }
                        Err(e) => log_dav(&cfg_clone, &format!("save thread dav2: parse json failed: {}", e)),
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

    /// Synchronous — runs to completion before the window closes.
    pub fn backup_on_exit(&self) {
        let slug = self.active_profile().slug.clone();
        let json = self.data.to_json();
        backup_local(&self.cfg, &slug, &json);
        if self.cfg.backup_webdav {
            let prof = self.active_profile();
            if prof.has_dav()  { dav_backup_upload(&prof,                   &slug, &json, &self.cfg); }
            if prof.has_dav2() { dav_backup_upload(&prof.as_dav2_profile(), &slug, &json, &self.cfg); }
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
            Ok(_)  => { log_dav(&self.cfg, "import_json: empty months, rejected"); false }
            Err(e) => { log_dav(&self.cfg, &format!("import_json: parse failed: {}", e)); false }
        }
    }

    pub fn reset(&mut self) {
        log_dav(&self.cfg, &format!("reset: clearing data for '{}'", self.active_profile().slug));
        self.data = AppData::new_empty();
        self.save();
    }

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
