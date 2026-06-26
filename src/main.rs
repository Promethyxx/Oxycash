slint::include_modules!();

mod model;
mod config;
mod webdav;
mod storage;
mod state;
mod compute;
mod ui_helpers;
mod push;
mod callbacks_budget;
mod callbacks_data;
mod callbacks_settings;
mod callbacks_misc;
mod theme;
mod i18n;

use std::sync::{Arc, Mutex};

use compute::sync_frais_from_months;
use push::{push_charts, push_debts, push_expenses, push_i18n, push_month, push_savings, push_settings, push_viability};
use state::AppState;
use ui_helpers::{apply_theme, month_key_to_tab, status_str};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    if let Some(path) = app.internal_data_path() {
        config::set_android_data_dir(path);
    }
    slint::android::init(app).unwrap();
    run();
}

#[cfg(not(target_os = "android"))]
fn main() { run(); }

fn run() {
    let state  = Arc::new(Mutex::new(AppState::new()));
    let window = AppWindow::new().unwrap();

    // Initial data push
    {
        let mut st = state.lock().unwrap();
        sync_frais_from_months(&mut st);
        window.set_current_tab(month_key_to_tab(&st.current_month));
        window.set_profile_name(st.storage.active_profile().name.clone().into());
        window.set_currency(st.storage.cfg.currency.clone().into());
        window.set_sync_status(status_str(st.storage.status()).into());
        window.set_lang_label(if st.storage.cfg.lang == "en" { "FR" } else { "EN" }.into());
        push_month(&window, &st);
        push_charts(&window, &st);
        push_debts(&window, &st);
        push_savings(&window, &st);
        push_expenses(&window, &st);
        push_viability(&window, &st);
        push_settings(&window, &st);
        apply_theme(&window, true);
        #[cfg(target_os = "android")]
        window.set_status_bar_height(48.0);
        push_i18n(&window, &st.storage.cfg.lang);
        Palette::get(&window).set_font_offset(st.storage.cfg.font_scale as f32);
    }

    // Register all callbacks
    callbacks_budget::register(&window, &state);
    callbacks_data::register(&window, &state);
    callbacks_settings::register(&window, &state);
    callbacks_misc::register(&window, &state);

    window.run().unwrap();
}
