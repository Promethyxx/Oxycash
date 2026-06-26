// oxycash-rs - storage.rs
use std::collections::HashSet;

use crate::config::{
    Config, Profile,
    backup_dir, base_dir, local_data_file,
    load_config, save_config, slugify, backup_local,
};
use crate::webdav::{
    dav_backup_upload, dav_connect, dav_read_marker, dav_rename, dav_save,
    dav_test_http, dav_write_marker, log_dav, make_client, now_ts, ConnectResult,
};
use crate::model::{AppData, apply_recurring};

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

    // ── Profile accessors ─────────────────────────────────────────────────────

    pub fn active_profile(&self) -> &Profile {
        let slug = &self.cfg.active;
        self.cfg.profiles.iter().find(|p| &p.slug == slug)
            .or_else(|| self.cfg.profiles.first())
            .unwrap_or_else(|| panic!("no profiles in config"))
    }

    pub fn active_profile_mut(&mut self) -> &mut Profile {
        let slug = self.cfg.active.clone();
        if let Some(i) = self.cfg.profiles.iter().position(|p| p.slug == slug) {
            &mut self.cfg.profiles[i]
        } else if !self.cfg.profiles.is_empty() {
            &mut self.cfg.profiles[0]
        } else {
            panic!("no profiles in config")
        }
    }

    pub fn add_profile(&mut self, name: &str) -> String {
        let base = slugify(name);
        let existing: HashSet<_> = self.cfg.profiles.iter().map(|p| p.slug.clone()).collect();
        let mut slug = base.clone();
        let mut n = 2;
        while existing.contains(&slug) { slug = format!("{}_{}", base, n); n += 1; }
        self.cfg.profiles.push(Profile { name: name.to_string(), slug: slug.clone() });
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

    /// Rename a profile: updates name + slug, migrates local file and DAV files.
    /// Returns the new slug.
    pub fn rename_profile(&mut self, old_slug: &str, new_name: &str) -> String {
        let new_name = new_name.trim();
        if new_name.is_empty() { return old_slug.to_string(); }

        let base = slugify(new_name);
        let existing: HashSet<_> = self.cfg.profiles.iter()
            .filter(|p| p.slug != old_slug)
            .map(|p| p.slug.clone()).collect();
        let mut new_slug = base.clone();
        let mut n = 2;
        while existing.contains(&new_slug) { new_slug = format!("{}_{}", base, n); n += 1; }

        // Only display name changed
        if new_slug == old_slug {
            if let Some(p) = self.cfg.profiles.iter_mut().find(|p| p.slug == old_slug) {
                p.name = new_name.to_string();
            }
            save_config(&self.cfg);
            return new_slug;
        }

        // Local file rename
        let old_lf = local_data_file(&self.cfg, old_slug);
        let new_lf = local_data_file(&self.cfg, &new_slug);
        if old_lf.exists() {
            if let Err(e) = std::fs::rename(&old_lf, &new_lf) {
                log_dav(&self.cfg, &format!("rename_profile: rename {:?}→{:?} failed: {}", old_lf, new_lf, e));
                if let Ok(data) = std::fs::read_to_string(&old_lf) {
                    if std::fs::write(&new_lf, &data).is_ok() {
                        let _ = std::fs::remove_file(&old_lf);
                    }
                }
            }
        }

        // DAV rename (data file + marker file)
        if self.cfg.has_dav() {
            let old_dp = self.cfg.dav_profile(old_slug);
            dav_rename(&old_dp, &new_slug, &self.cfg);
        }
        if self.cfg.has_dav2() {
            let old_dp2 = self.cfg.dav2_profile(old_slug);
            dav_rename(&old_dp2, &new_slug, &self.cfg);
        }

        // Update config
        if let Some(p) = self.cfg.profiles.iter_mut().find(|p| p.slug == old_slug) {
            p.name = new_name.to_string();
            p.slug = new_slug.clone();
        }
        if self.cfg.active == old_slug { self.cfg.active = new_slug.clone(); }
        save_config(&self.cfg);
        log_dav(&self.cfg, &format!("rename_profile: '{}' → '{}' ('{}')", old_slug, new_slug, new_name));
        new_slug
    }

    pub fn switch_profile(&mut self, slug: &str) {
        self.cfg.active = slug.to_string();
        save_config(&self.cfg);
        self.load();
    }

    // ── DAV config ────────────────────────────────────────────────────────────

    pub fn save_dav_config(&mut self, url: &str, user: &str, pw: &str) {
        self.cfg.dav_url  = url.to_string();
        self.cfg.dav_user = user.to_string();
        self.cfg.dav_pass = pw.to_string();
        save_config(&self.cfg);
    }

    pub fn save_dav2_config(&mut self, url: &str, user: &str, pw: &str, enabled: bool) {
        self.cfg.dav2_url     = url.to_string();
        self.cfg.dav2_user    = user.to_string();
        self.cfg.dav2_pass    = pw.to_string();
        self.cfg.dav2_enabled = enabled;
        save_config(&self.cfg);
    }

    // ── DAV test ──────────────────────────────────────────────────────────────

    pub fn test_dav(&self) -> (bool, String) {
        let dp = self.cfg.dav_profile(&self.cfg.active);
        match make_client() {
            Ok(c)  => dav_test_http(&dp, c),
            Err(e) => (false, e),
        }
    }

    pub fn test_dav2(&self) -> (bool, String) {
        if !self.cfg.has_dav2() { return (false, "WebDAV secondaire non configuré".into()); }
        let dp = self.cfg.dav2_profile(&self.cfg.active);
        match make_client() {
            Ok(c)  => dav_test_http(&dp, c),
            Err(e) => (false, e),
        }
    }

    // ── Load ──────────────────────────────────────────────────────────────────

    pub fn load(&mut self) -> SyncStatus {
        let slug = self.cfg.active.clone();
        let dav1 = self.cfg.has_dav();
        let dav2 = self.cfg.has_dav2();

        if dav1 || dav2 {
            let dp1 = self.cfg.dav_profile(&slug);
            let dp2 = self.cfg.dav2_profile(&slug);
            let ts1 = if dav1 { dav_read_marker(&dp1) } else { 0 };
            let ts2 = if dav2 { dav_read_marker(&dp2) } else { 0 };

            let lf         = local_data_file(&self.cfg, &slug);
            let local_json = std::fs::read_to_string(&lf)
                .unwrap_or_else(|_| AppData::new_empty().to_json());

            // Try most-recent source first
            let sources: Vec<bool> = if ts2 > ts1 && dav2 { vec![false, true] }
                                     else if dav1          { vec![true, false] }
                                     else                  { vec![false] };

            for use_dav1 in sources {
                if use_dav1 && !dav1 { continue; }
                if !use_dav1 && !dav2 { continue; }
                let dp = if use_dav1 { self.cfg.dav_profile(&slug) } else { self.cfg.dav2_profile(&slug) };

                match dav_connect(&dp, &local_json, &self.cfg) {
                    ConnectResult::Loaded(mut app) => {
                        apply_recurring(&mut app);
                        let dir = base_dir(&self.cfg);
                        if let Err(e) = std::fs::create_dir_all(&dir) {
                            log_dav(&self.cfg, &format!("load: create_dir failed: {}", e));
                        }
                        if let Err(e) = std::fs::write(&lf, app.to_json()) {
                            log_dav(&self.cfg, &format!("load: write mirror failed: {}", e));
                        }
                        self.data   = app;
                        self.dav_ok = true;
                        return SyncStatus::Dav;
                    }
                    ConnectResult::Pushed => {
                        match std::fs::read_to_string(&lf) {
                            Ok(text) => match AppData::from_json(&text) {
                                Ok(mut app) => { apply_recurring(&mut app); self.data = app; }
                                Err(e) => log_dav(&self.cfg, &format!("load/pushed: parse failed: {}", e)),
                            },
                            Err(e) => log_dav(&self.cfg, &format!("load/pushed: read failed: {}", e)),
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

        self.dav_ok = false;
        let lf = local_data_file(&self.cfg, &slug);
        if lf.exists() {
            match std::fs::read_to_string(&lf) {
                Ok(text) => match AppData::from_json(&text) {
                    Ok(mut app) => { apply_recurring(&mut app); self.data = app; return SyncStatus::Local; }
                    Err(e) => log_dav(&self.cfg, &format!("load: parse local failed: {}", e)),
                },
                Err(e) => log_dav(&self.cfg, &format!("load: read local failed: {}", e)),
            }
        }

        log_dav(&self.cfg, &format!("load: no data for '{}', starting empty", slug));
        self.data = AppData::new_empty();
        apply_recurring(&mut self.data);
        let dir = base_dir(&self.cfg);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log_dav(&self.cfg, &format!("load: create_dir failed: {}", e));
        }
        if let Err(e) = std::fs::write(local_data_file(&self.cfg, &slug), self.data.to_json()) {
            log_dav(&self.cfg, &format!("load: write initial failed: {}", e));
        }
        SyncStatus::Local
    }

    // ── Save ──────────────────────────────────────────────────────────────────

    pub fn save(&mut self) -> SyncStatus {
        let slug = self.cfg.active.clone();
        let json = self.data.to_json();

        let dir = base_dir(&self.cfg);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log_dav(&self.cfg, &format!("save: create_dir failed: {}", e));
        }
        if let Err(e) = std::fs::write(local_data_file(&self.cfg, &slug), &json) {
            log_dav(&self.cfg, &format!("save: write local failed: {}", e));
        }

        let dav1          = self.cfg.has_dav();
        let dav2          = self.cfg.has_dav2();
        let backup_webdav = self.cfg.backup_webdav;

        if dav1 || dav2 {
            let ts    = now_ts();
            let dp1   = self.cfg.dav_profile(&slug);
            let dp2   = self.cfg.dav2_profile(&slug);
            let json2 = json.clone();
            let cfg2  = self.cfg.clone();

            std::thread::spawn(move || {
                if dav1 {
                    if let Ok(data) = AppData::from_json(&json2) {
                        if dav_save(&dp1, &data, &cfg2) {
                            dav_write_marker(&dp1, ts);
                            if backup_webdav { dav_backup_upload(&dp1, &json2, &cfg2); }
                        }
                    }
                }
                if dav2 {
                    if let Ok(data) = AppData::from_json(&json2) {
                        if dav_save(&dp2, &data, &cfg2) {
                            dav_write_marker(&dp2, ts);
                            if backup_webdav { dav_backup_upload(&dp2, &json2, &cfg2); }
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

    // ── Exit backup ───────────────────────────────────────────────────────────

    /// Backup all profiles on exit — synchronous.
    /// Active profile uses in-memory data; other profiles read from their local file.
    pub fn backup_on_exit(&self) {
        let active_slug = self.cfg.active.clone();
        let active_json = self.data.to_json();

        for prof in &self.cfg.profiles {
            let slug = &prof.slug;

            // Get the JSON for this profile
            let json: String = if *slug == active_slug {
                active_json.clone()
            } else {
                let lf = local_data_file(&self.cfg, slug);
                match std::fs::read_to_string(&lf) {
                    Ok(s)  => s,
                    Err(e) => {
                        log_dav(&self.cfg, &format!("backup_on_exit: read '{}' failed: {}", slug, e));
                        continue;
                    }
                }
            };

            // Local backup
            backup_local(&self.cfg, slug, &json);

            // DAV backup
            if self.cfg.backup_webdav {
                if self.cfg.has_dav() {
                    dav_backup_upload(&self.cfg.dav_profile(slug), &json, &self.cfg);
                }
                if self.cfg.has_dav2() {
                    dav_backup_upload(&self.cfg.dav2_profile(slug), &json, &self.cfg);
                }
            }
        }
    }

    // ── Status ────────────────────────────────────────────────────────────────

    pub fn status(&self) -> SyncStatus {
        if self.dav_ok { return SyncStatus::Dav; }
        if self.cfg.has_dav() || self.cfg.has_dav2() { return SyncStatus::DavError; }
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
        log_dav(&self.cfg, &format!("reset: clearing data for '{}'", self.cfg.active));
        self.data = AppData::new_empty();
        self.save();
    }

    // ── Config setters ────────────────────────────────────────────────────────

    pub fn set_currency(&mut self, cur: &str) {
        self.cfg.currency = if cur.trim().is_empty() { "CHF".into() } else { cur.trim().into() };
        save_config(&self.cfg);
    }

    pub fn set_lang(&mut self, lang: &str) { self.cfg.lang = lang.to_string(); save_config(&self.cfg); }

    pub fn set_data_dir(&mut self, path: &str) { self.cfg.data_dir = path.trim().to_string(); save_config(&self.cfg); }

    pub fn set_backup_local(&mut self, val: bool)  { self.cfg.backup_local  = val; save_config(&self.cfg); }
    pub fn set_backup_webdav(&mut self, val: bool) { self.cfg.backup_webdav = val; save_config(&self.cfg); }

    pub fn data_dir_display(&self) -> String {
        if self.cfg.data_dir.trim().is_empty() { base_dir(&self.cfg).to_string_lossy().to_string() }
        else { self.cfg.data_dir.clone() }
    }

    pub fn backup_dir_display(&self) -> String {
        backup_dir(&self.cfg).to_string_lossy().to_string()
    }
}
