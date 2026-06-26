// oxycash-rs - callbacks_settings.rs
// Callbacks: WebDAV1/2 (save/test/clear), currency, language, theme, font scale,
// profile management, backup toggles, data dir, export/import/reset.
use slint::{ComponentHandle, Global};
use std::sync::{Arc, Mutex};

use crate::{AppWindow, Palette};
use crate::compute::sync_frais_from_months;
use crate::config::save_config;
use crate::push::{push_charts, push_debts, push_expenses, push_month, push_savings, push_settings};
use crate::state::AppState;
use crate::storage::SyncStatus;
use crate::ui_helpers::{apply_theme, show_toast, status_str};
use crate::webdav::{dav_test_http, make_client};
use crate::push::push_i18n;

pub fn register(window: &AppWindow, state: &Arc<Mutex<AppState>>) {

    // ── WebDAV1 ───────────────────────────────────────────────────────────────

    // Save + reconnect DAV1
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_save_dav(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url:  String = w.get_settings_dav_url().into();
            let user: String = w.get_settings_dav_user().into();
            let pass: String = w.get_settings_dav_pass().into();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav(&slug, &url, &user, &pass);
            w.set_settings_dav_status("Connexion en cours…".into());
            let status = st.storage.load();
            if status == SyncStatus::Dav {
                w.set_settings_dav_status("Connecté ✓".into());
                w.set_sync_status(status_str(SyncStatus::Dav).into());
                sync_frais_from_months(&mut st);
                push_month(&w, &st);
                push_charts(&w, &st);
                push_debts(&w, &st);
                push_savings(&w, &st);
                push_expenses(&w, &st);
                show_toast(&w, "WebDAV connecté ✓");
            } else {
                w.set_settings_dav_status("Connexion échouée — vérifier URL/credentials".into());
                w.set_sync_status(status_str(st.storage.status()).into());
                show_toast(&w, "Connexion échouée");
            }
        });
    }

    // Progressive 4-step DAV1 test (runs in a thread)
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_test_dav(move || {
            let ww2 = ww.clone();
            let w   = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url:  String = w.get_settings_dav_url().into();
            let user: String = w.get_settings_dav_user().into();
            let pass: String = w.get_settings_dav_pass().into();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav(&slug, &url, &user, &pass);
            let profile = st.storage.active_profile().clone();
            drop(st);
            w.set_settings_dav_status("1/4 init client…".into());

            let state_ref2 = state_ref.clone();
            let ww3 = ww2.clone();

            macro_rules! step {
                ($ww:expr, $msg:expr) => {{
                    let ww_ = $ww.clone(); let msg_ = $msg.to_string();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = ww_.upgrade() { w.set_settings_dav_status(msg_.into()); }
                    });
                }};
            }

            std::thread::spawn(move || {
                let client = match make_client() {
                    Ok(c) => c,
                    Err(e) => { step!(ww3, format!("1/4 FAIL client: {}", e)); return; }
                };
                step!(ww3, "2/4 TCP 1.1.1.1…");
                if let Err(e) = std::net::TcpStream::connect_timeout(
                    &"1.1.1.1:443".parse().unwrap(), std::time::Duration::from_secs(4))
                { step!(ww3, format!("2/4 FAIL réseau: {}", e)); return; }

                step!(ww3, "3/4 DNS…");
                let host = extract_host(&profile.dav_url);
                let ip   = match resolve_dns(&host) {
                    Ok(ip) => ip,
                    Err(e) => { step!(ww3, e); return; }
                };

                step!(ww3, format!("4/4 HTTPS {} → {}…", host, ip));
                let (ok, msg) = dav_test_http(&profile, client);
                let msg2 = msg.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = ww3.upgrade() { w.set_settings_dav_status(format!("4/4 {}", msg2).into()); }
                });
                let _ = slint::invoke_from_event_loop(move || {
                    let mut st = state_ref2.lock().unwrap();
                    if ok { st.storage.dav_ok = true; }
                    let status = status_str(st.storage.status()).to_string();
                    drop(st);
                    if let Some(w) = ww2.upgrade() {
                        w.set_sync_status(status.into());
                        show_toast(&w, &msg);
                    }
                });
            });
        });
    }

    // Clear DAV1
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_clear_dav(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav(&slug, "", "", "");
            st.storage.dav_ok = false;
            w.set_settings_dav_url("".into());
            w.set_settings_dav_user("".into());
            w.set_settings_dav_pass("".into());
            w.set_settings_dav_status("Config cleared".into());
            w.set_sync_status(status_str(st.storage.status()).into());
            show_toast(&w, "Config cleared");
        });
    }

    // ── WebDAV2 ───────────────────────────────────────────────────────────────

    // Save + reconnect DAV2
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_save_dav2(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url:     String = w.get_settings_dav2_url().into();
            let user:    String = w.get_settings_dav2_user().into();
            let pass:    String = w.get_settings_dav2_pass().into();
            let enabled: bool   = w.get_settings_dav2_enabled();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav2(&slug, &url, &user, &pass, enabled);
            w.set_settings_dav2_status("Connexion en cours…".into());
            let status = st.storage.load();
            if status == SyncStatus::Dav {
                w.set_settings_dav2_status("Connecté ✓".into());
                w.set_sync_status(status_str(SyncStatus::Dav).into());
                sync_frais_from_months(&mut st);
                push_month(&w, &st);
                push_charts(&w, &st);
                push_debts(&w, &st);
                push_savings(&w, &st);
                push_expenses(&w, &st);
                show_toast(&w, "WebDAV2 connecté ✓");
            } else {
                w.set_settings_dav2_status("Connexion échouée — vérifier URL/credentials".into());
                w.set_sync_status(status_str(st.storage.status()).into());
                show_toast(&w, "WebDAV2 connexion échouée");
            }
        });
    }

    // Progressive 4-step DAV2 test (runs in a thread)
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_test_dav2(move || {
            let ww2 = ww.clone();
            let w   = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url:     String = w.get_settings_dav2_url().into();
            let user:    String = w.get_settings_dav2_user().into();
            let pass:    String = w.get_settings_dav2_pass().into();
            let enabled: bool   = w.get_settings_dav2_enabled();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav2(&slug, &url, &user, &pass, enabled);
            let profile = st.storage.active_profile().as_dav2_profile();
            drop(st);
            w.set_settings_dav2_status("1/4 init client…".into());

            let state_ref2 = state_ref.clone();
            let ww3 = ww2.clone();

            macro_rules! step {
                ($ww:expr, $msg:expr) => {{
                    let ww_ = $ww.clone(); let msg_ = $msg.to_string();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = ww_.upgrade() { w.set_settings_dav2_status(msg_.into()); }
                    });
                }};
            }

            std::thread::spawn(move || {
                let client = match make_client() {
                    Ok(c) => c,
                    Err(e) => { step!(ww3, format!("1/4 FAIL client: {}", e)); return; }
                };
                step!(ww3, "2/4 TCP 1.1.1.1…");
                if let Err(e) = std::net::TcpStream::connect_timeout(
                    &"1.1.1.1:443".parse().unwrap(), std::time::Duration::from_secs(4))
                { step!(ww3, format!("2/4 FAIL réseau: {}", e)); return; }

                step!(ww3, "3/4 DNS…");
                let host = extract_host(&profile.dav_url);
                let ip   = match resolve_dns(&host) {
                    Ok(ip) => ip,
                    Err(e) => { step!(ww3, e); return; }
                };

                step!(ww3, format!("4/4 HTTPS {} → {}…", host, ip));
                let (_ok, msg) = dav_test_http(&profile, client);
                let msg2 = msg.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = ww3.upgrade() { w.set_settings_dav2_status(format!("4/4 {}", msg2).into()); }
                });
                let _ = slint::invoke_from_event_loop(move || {
                    let st = state_ref2.lock().unwrap();
                    let status = status_str(st.storage.status()).to_string();
                    drop(st);
                    if let Some(w) = ww2.upgrade() {
                        w.set_sync_status(status.into());
                        show_toast(&w, &msg);
                    }
                });
            });
        });
    }

    // Clear DAV2
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_clear_dav2(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav2(&slug, "", "", "", false);
            w.set_settings_dav2_url("".into());
            w.set_settings_dav2_user("".into());
            w.set_settings_dav2_pass("".into());
            w.set_settings_dav2_enabled(false);
            w.set_settings_dav2_status("Config cleared".into());
            show_toast(&w, "WebDAV2 cleared");
        });
    }

    // ── Currency / Lang / Theme / Font ────────────────────────────────────────

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_save_currency(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let cur: String = w.get_settings_currency_edit().into();
            st.storage.set_currency(&cur);
            w.set_currency(st.storage.cfg.currency.clone().into());
            show_toast(&w, "Currency saved");
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_toggle_lang(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let new_lang = if st.storage.cfg.lang == "en" { "fr" } else { "en" };
            st.storage.set_lang(new_lang);
            w.set_lang_label(if new_lang == "en" { "FR" } else { "EN" }.into());
            push_i18n(&w, new_lang);
            show_toast(&w, &format!("🌐 {}", new_lang.to_uppercase()));
        });
    }

    {
        let ww = window.as_weak();
        window.on_toggle_theme(move || {
            let w = ww.unwrap();
            let new_dark = !w.get_is_dark();
            w.set_is_dark(new_dark);
            apply_theme(&w, new_dark);
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_font_scale_down(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.cfg.font_scale = (st.storage.cfg.font_scale - 2).max(-6);
            save_config(&st.storage.cfg);
            Palette::get(&w).set_font_offset(st.storage.cfg.font_scale as f32);
            show_toast(&w, &format!("Font {}", st.storage.cfg.font_scale));
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_font_scale_up(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.cfg.font_scale = (st.storage.cfg.font_scale + 2).min(8);
            save_config(&st.storage.cfg);
            Palette::get(&w).set_font_offset(st.storage.cfg.font_scale as f32);
            show_toast(&w, &format!("Font +{}", st.storage.cfg.font_scale));
        });
    }

    // ── Profiles ──────────────────────────────────────────────────────────────

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_switch_profile(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.cfg.profiles.len() {
                let slug = st.storage.cfg.profiles[i].slug.clone();
                st.storage.switch_profile(&slug);
                push_month(&w, &st);
                push_charts(&w, &st);
                push_debts(&w, &st);
                push_savings(&w, &st);
                push_expenses(&w, &st);
                push_settings(&w, &st);
                w.set_sync_status(status_str(st.storage.status()).into());
            }
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_profile(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.cfg.profiles.len() && st.storage.cfg.profiles.len() > 1 {
                let slug = st.storage.cfg.profiles[i].slug.clone();
                st.storage.delete_profile(&slug);
                st.storage.load();
                push_month(&w, &st);
                push_settings(&w, &st);
                w.set_sync_status(status_str(st.storage.status()).into());
                show_toast(&w, "Profile deleted");
            }
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_profile(move |name| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let n: String = name.into();
            let n = n.trim();
            if n.is_empty() { return; }
            let slug = st.storage.add_profile(n);
            st.storage.switch_profile(&slug);
            push_month(&w, &st);
            push_settings(&w, &st);
            w.set_sync_status(status_str(st.storage.status()).into());
            show_toast(&w, &format!("Profile '{}' created", n));
        });
    }

    // Rename profile
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_rename_profile(move |new_name| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let n: String = new_name.into();
            let slug = st.storage.cfg.active.clone();
            st.storage.rename_profile(&slug, &n);
            w.set_profile_name(st.storage.active_profile().name.clone().into());
            push_settings(&w, &st);
            show_toast(&w, "Profile renamed");
        });
    }

    // ── Backup / Data dir ─────────────────────────────────────────────────────

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_set_backup_local(move |val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.set_backup_local(val);
            w.set_settings_backup_local(val);
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_set_backup_webdav(move |val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.set_backup_webdav(val);
            w.set_settings_backup_webdav(val);
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_set_data_dir(move |path| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let p: String = path.into();
            st.storage.set_data_dir(&p);
            w.set_settings_data_dir_display(st.storage.data_dir_display().into());
            w.set_settings_backup_dir_display(st.storage.backup_dir_display().into());
            show_toast(&w, "Dossier sauvegardé ✓");
        });
    }

    // ── Export / Import / Reset ───────────────────────────────────────────────

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_do_export(move || {
            let w = ww.unwrap();
            let st = state_ref.lock().unwrap();
            let json  = st.storage.export_json();
            let fname = format!("oxycash_{}.json", st.storage.cfg.active);
            drop(st);
            #[cfg(not(target_os = "android"))]
            {
                let path = rfd::FileDialog::new()
                    .set_file_name(&fname)
                    .add_filter("JSON", &["json"])
                    .save_file();
                match path {
                    Some(p) => match std::fs::write(&p, &json) {
                        Ok(_)  => show_toast(&w, "Exported ✓"),
                        Err(e) => show_toast(&w, &format!("Error: {}", e)),
                    },
                    None => {}
                }
            }
            #[cfg(target_os = "android")]
            {
                let dir  = dirs::data_local_dir().unwrap_or_default();
                let path = dir.join(&fname);
                match std::fs::write(&path, &json) {
                    Ok(_)  => show_toast(&w, "Exported ✓"),
                    Err(e) => show_toast(&w, &format!("Error: {}", e)),
                }
            }
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_do_import(move || {
            let w = ww.unwrap();
            #[cfg(not(target_os = "android"))]
            {
                let path = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file();
                match path {
                    Some(p) => match std::fs::read_to_string(&p) {
                        Ok(raw) => {
                            let mut st = state_ref.lock().unwrap();
                            if st.storage.import_json(&raw) {
                                push_month(&w, &st);
                                push_charts(&w, &st);
                                push_debts(&w, &st);
                                push_savings(&w, &st);
                                push_expenses(&w, &st);
                                show_toast(&w, "Import OK ✓");
                            } else {
                                show_toast(&w, "Invalid JSON format");
                            }
                        }
                        Err(e) => show_toast(&w, &format!("Error: {}", e)),
                    },
                    None => {}
                }
            }
            #[cfg(target_os = "android")]
            show_toast(&w, "Import not available on Android");
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_do_reset(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.reset();
            push_month(&w, &st);
            push_charts(&w, &st);
            push_debts(&w, &st);
            push_savings(&w, &st);
            push_expenses(&w, &st);
            show_toast(&w, "Reset ✓");
        });
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

pub fn extract_host(url: &str) -> String {
    let s = url.trim()
        .strip_prefix("https://").or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    s[..s.find('/').unwrap_or(s.len())].to_string()
}

/// Spawn a thread to resolve `host`:443, wait up to 6s, return the IP string or None.
/// On failure, the error string is returned so the caller can feed it to their step! macro.
pub fn resolve_dns(host: &str) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let host2 = host.to_string();
    std::thread::spawn(move || {
        let r = std::net::ToSocketAddrs::to_socket_addrs(&(host2.as_str(), 443u16))
            .map(|mut it| it.next().map(|a| a.ip().to_string()));
        let _ = tx.send(r);
    });
    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok(Ok(Some(ip))) => Ok(ip),
        Ok(Ok(None))     => Err(format!("3/4 FAIL DNS vide: {}", host)),
        Ok(Err(e))       => Err(format!("3/4 FAIL DNS: {}", e)),
        Err(_)           => Err("3/4 FAIL DNS timeout 6s".to_string()),
    }
}
