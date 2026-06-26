// oxycash-rs - ui_helpers.rs
// Slint model builders, color/theme helpers, toast, tab↔month-key conversions, status_str.
use slint::{ComponentHandle, Global, ModelRc, VecModel};

use crate::{AppWindow, LineItem, Palette, PaymentItem, Tab};
use crate::model::{fmt, Line};
use crate::storage::SyncStatus;
use crate::theme::{DARK, LIGHT};

// ── Slint model builders ──────────────────────────────────────────────────────

pub fn make_line_items(lines: &[Line]) -> ModelRc<LineItem> {
    let items: Vec<LineItem> = lines.iter().enumerate().map(|(i, l)| {
        let etat  = l.payments.iter().map(|p| p.amount).sum::<f64>();
        let solde = (l.banque + l.cash) - etat;
        let has_rec  = l.recurring.is_some();
        let rec_freq = l.recurring.as_ref().map(|r| r.freq as i32).unwrap_or(0);
        LineItem {
            name:           l.name.clone().into(),
            banque:         l.banque as f32,
            cash:           l.cash as f32,
            etat:           fmt(etat).into(),
            solde:          fmt(solde).into(),
            solde_val:      solde as f32,
            idx:            i as i32,
            has_recurring:  has_rec,
            recurring_freq: rec_freq,
        }
    }).collect();
    ModelRc::new(VecModel::from(items))
}

pub fn make_payment_items(lines: &[Line]) -> ModelRc<ModelRc<PaymentItem>> {
    let outer: Vec<ModelRc<PaymentItem>> = lines.iter().map(|l| {
        let pays: Vec<PaymentItem> = l.payments.iter().enumerate().map(|(i, p)| {
            PaymentItem {
                date:   p.date.clone().into(),
                amount: p.amount as f32,
                idx:    i as i32,
            }
        }).collect();
        ModelRc::new(VecModel::from(pays))
    }).collect();
    ModelRc::new(VecModel::from(outer))
}

pub fn make_bool_model(vals: &[bool]) -> ModelRc<bool> {
    ModelRc::new(VecModel::from(vals.to_vec()))
}

// ── Color helpers ─────────────────────────────────────────────────────────────

pub fn hex_color(hex: &str) -> slint::Color {
    let h = hex.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0xFF00FF);
    slint::Color::from_argb_encoded(0xFF000000 | n)
}

pub fn color_for(n: f64) -> slint::Color {
    if n > 0.01       { hex_color("#7BC47F") }
    else if n < -0.01 { hex_color("#E05555") }
    else              { hex_color("#E8E4DE") }
}

// ── Theme ─────────────────────────────────────────────────────────────────────

pub fn apply_theme(window: &AppWindow, is_dark: bool) {
    let p = if is_dark { &DARK } else { &LIGHT };
    let pal = Palette::get(window);
    pal.set_bg(hex_color(p.bg));
    pal.set_bg2(hex_color(p.bg2));
    pal.set_card(hex_color(p.card));
    pal.set_card_border(hex_color(p.card_border));
    pal.set_text(hex_color(p.text));
    pal.set_text2(hex_color(p.text2));
    pal.set_text3(hex_color(p.text3));
    pal.set_red(hex_color(p.red));
    pal.set_teal(hex_color(p.teal));
    pal.set_gold(hex_color(p.gold));
    pal.set_green(hex_color(p.green));
    pal.set_danger(hex_color(p.danger));
    pal.set_amber(hex_color(p.amber));
    pal.set_brown(hex_color(p.brown));
    pal.set_blue(hex_color(p.blue));
    pal.set_purple(hex_color(p.purple));
}

// ── Toast ─────────────────────────────────────────────────────────────────────

pub fn show_toast(window: &AppWindow, msg: &str) {
    window.set_toast_message(msg.into());
    window.set_toast_show(true);
    let w2 = window.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(2500), move || {
        if let Some(w) = w2.upgrade() { w.set_toast_show(false); }
    });
}

// ── Tab ↔ month-key conversions ───────────────────────────────────────────────

pub fn month_key_to_tab(key: &str) -> Tab {
    match key {
        "JAN" => Tab::JAN, "FEB" => Tab::FEB, "MAR" => Tab::MAR,
        "APR" => Tab::APR, "MAI" => Tab::MAI, "JUN" => Tab::JUN,
        "JUL" => Tab::JUL, "AUG" => Tab::AUG, "SEP" => Tab::SEP,
        "OCT" => Tab::OCT, "NOV" => Tab::NOV, "DEC" => Tab::DEC,
        _ => Tab::JAN,
    }
}

pub fn tab_to_month_key(tab: Tab) -> Option<&'static str> {
    match tab {
        Tab::JAN => Some("JAN"), Tab::FEB => Some("FEB"), Tab::MAR => Some("MAR"),
        Tab::APR => Some("APR"), Tab::MAI => Some("MAI"), Tab::JUN => Some("JUN"),
        Tab::JUL => Some("JUL"), Tab::AUG => Some("AUG"), Tab::SEP => Some("SEP"),
        Tab::OCT => Some("OCT"), Tab::NOV => Some("NOV"), Tab::DEC => Some("DEC"),
        _ => None,
    }
}

// ── Sync status string ────────────────────────────────────────────────────────

pub fn status_str(s: SyncStatus) -> &'static str {
    match s {
        SyncStatus::Dav      => "dav",
        SyncStatus::DavError => "dav_err",
        SyncStatus::Local    => "local",
    }
}
