// oxycash-rs - callbacks_misc.rs
// Callbacks: manual save, go-to-config shortcut, on_close backup.
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

use crate::{AppWindow, Tab};
use crate::state::AppState;
use crate::ui_helpers::{show_toast, status_str};

pub fn register(window: &AppWindow, state: &Arc<Mutex<AppState>>) {

    // Manual save
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_save_requested(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let status = st.storage.save();
            w.set_sync_status(status_str(status).into());
            show_toast(&w, "Saved");
        });
    }

    // Navigate to config tab
    {
        let ww = window.as_weak();
        window.on_go_config(move || { ww.unwrap().set_current_tab(Tab::Config); });
    }

    // Backup on window close
    {
        let state_ref = state.clone();
        window.window().on_close_requested(move || {
            let st = state_ref.lock().unwrap();
            st.storage.backup_on_exit();
            slint::CloseRequestResponse::HideWindow
        });
    }
}
