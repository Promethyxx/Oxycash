slint::include_modules!();

mod model;
mod storage;
mod theme;

use model::{detect_budget_month, fmt, fmt_sign, today, Month, Payment, Line, MONTHS};
use storage::{Storage, SyncStatus};

use std::sync::{Arc, Mutex};
use slint::{SharedString, VecModel, ModelRc};

// --- App state shared across callbacks
struct AppState {
    storage:         Storage,
    current_month:   String,
    sections_open:   [bool; 4],
    lines_expanded:  [Vec<bool>; 4],
}

impl AppState {
    fn new() -> Self {
        let mut storage = Storage::new();
        storage.load();
        let current_month = detect_budget_month().to_string();
        Self {
            storage,
            current_month,
            sections_open:  [false; 4],
            lines_expanded: [vec![], vec![], vec![], vec![]],
        }
    }

    fn month(&self) -> Option<&Month> {
        self.storage.data.months.get(&self.current_month)
    }

    fn month_mut(&mut self) -> Option<&mut Month> {
        self.storage.data.months.get_mut(&self.current_month)
    }

    fn sec_lines(&self, si: usize) -> &Vec<Line> {
        match self.month() {
            Some(m) => match si {
                0 => &m.revenus,
                1 => &m.retraits,
                2 => &m.fixes,
                _ => &m.variables,
            },
            None => panic!("no month"),
        }
    }

    fn sec_lines_mut(&mut self, si: usize) -> &mut Vec<Line> {
        let mk = self.current_month.clone();
        let m = self.storage.data.months.get_mut(&mk).unwrap();
        match si {
            0 => &mut m.revenus,
            1 => &mut m.retraits,
            2 => &mut m.fixes,
            _ => &mut m.variables,
        }
    }

    fn ensure_expanded(&mut self) {
        for si in 0..4 {
            let n = self.sec_lines(si).len();
            let exp = &mut self.lines_expanded[si];
            exp.resize(n, false);
        }
    }
}

// --- Slint model helpers
fn make_line_items(lines: &[Line]) -> ModelRc<LineItem> {
    let items: Vec<LineItem> = lines.iter().enumerate().map(|(i, l)| {
        let etat  = l.payments.iter().map(|p| p.amount).sum::<f64>();
        let solde = (l.banque + l.cash) - etat;
        LineItem {
            name:   l.name.clone().into(),
            banque: l.banque as f32,
            cash:   l.cash as f32,
            etat:   etat as f32,
            solde:  solde as f32,
            idx:    i as i32,
        }
    }).collect();
    ModelRc::new(VecModel::from(items))
}

fn make_payment_items(lines: &[Line]) -> ModelRc<ModelRc<PaymentItem>> {
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

fn make_bool_model(vals: &[bool]) -> ModelRc<bool> {
    ModelRc::new(VecModel::from(vals.to_vec()))
}

// --- Summary calculations (exact port of month_view.py summary())
fn compute_summary(m: &Month) -> MonthSummary {
    let rev_banque = m.revenus.iter().map(|l| l.banque).sum::<f64>();
    let rev_cash   = m.revenus.iter().map(|l| l.cash).sum::<f64>();
    let rev_recu_b = m.revenus.iter().filter(|l| l.banque > 0.01).map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let rev_recu_c = m.revenus.iter().filter(|l| l.cash > 0.01).map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();

    let ret_a_retirer = m.retraits.iter().map(|l| l.banque).sum::<f64>();
    let ret_retire    = m.retraits.iter().map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();

    let all_dep: Vec<&Line> = m.fixes.iter().chain(m.variables.iter()).collect();
    let dep_banque = all_dep.iter().map(|l| l.banque).sum::<f64>();
    let dep_cash   = all_dep.iter().map(|l| l.cash).sum::<f64>();

    let paye_banque = all_dep.iter().filter(|l| l.banque > 0.01).map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>() + ret_retire;
    let paye_cash   = all_dep.iter().filter(|l| l.cash > 0.01).map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let paye_total  = paye_banque + paye_cash;

    let a_payer_b = dep_banque - all_dep.iter().filter(|l| l.banque > 0.01).map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let a_payer_c = dep_cash   - all_dep.iter().filter(|l| l.cash > 0.01).map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();

    let prev_banque = rev_banque - dep_banque - ret_a_retirer;
    let prev_cash   = rev_cash + ret_a_retirer - dep_cash;
    let prev_total  = prev_banque + prev_cash;

    let solde_banque = rev_recu_b - paye_banque;
    let solde_cash   = ret_retire + rev_recu_c - paye_cash;
    let solde_total  = solde_banque + solde_cash;

    MonthSummary {
        income: rev_banque + rev_cash, income_b: rev_banque, income_c: rev_cash,
        paid: paye_total, paid_b: paye_banque, paid_c: paye_cash,
        topay: a_payer_b + a_payer_c, topay_b: a_payer_b, topay_c: a_payer_c,
        forecast: prev_total, forecast_b: prev_banque, forecast_c: prev_cash,
        balance: solde_total, balance_b: solde_banque, balance_c: solde_cash,
    }
}

struct MonthSummary {
    income: f64, income_b: f64, income_c: f64,
    paid: f64, paid_b: f64, paid_c: f64,
    topay: f64, topay_b: f64, topay_c: f64,
    forecast: f64, forecast_b: f64, forecast_c: f64,
    balance: f64, balance_b: f64, balance_c: f64,
}

fn color_for(n: f64) -> slint::Color {
    if n > 0.01      { hex_color("#7BC47F") }
    else if n < -0.01 { hex_color("#E05555") }
    else              { hex_color("#E8E4DE") }
}

fn hex_color(hex: &str) -> slint::Color {
    let h = hex.trim_start_matches('#');
    let n = u32::from_str_radix(h, 16).unwrap_or(0xFF00FF);
    slint::Color::from_argb_encoded(0xFF000000 | n)
}

fn status_str(s: SyncStatus) -> &'static str {
    match s { SyncStatus::Dav => "dav", SyncStatus::DavError => "dav_err", SyncStatus::Local => "local" }
}

fn month_key_to_tab(key: &str) -> Tab {
    match key {
        "JAN" => Tab::JAN, "FEB" => Tab::FEB, "MAR" => Tab::MAR,
        "APR" => Tab::APR, "MAI" => Tab::MAI, "JUN" => Tab::JUN,
        "JUL" => Tab::JUL, "AUG" => Tab::AUG, "SEP" => Tab::SEP,
        "OCT" => Tab::OCT, "NOV" => Tab::NOV, "DEC" => Tab::DEC,
        _ => Tab::JAN,
    }
}

fn tab_to_month_key(tab: Tab) -> Option<&'static str> {
    match tab {
        Tab::JAN => Some("JAN"), Tab::FEB => Some("FEB"), Tab::MAR => Some("MAR"),
        Tab::APR => Some("APR"), Tab::MAI => Some("MAI"), Tab::JUN => Some("JUN"),
        Tab::JUL => Some("JUL"), Tab::AUG => Some("AUG"), Tab::SEP => Some("SEP"),
        Tab::OCT => Some("OCT"), Tab::NOV => Some("NOV"), Tab::DEC => Some("DEC"),
        _ => None,
    }
}

// --- Push all month data to the window
fn push_month(window: &AppWindow, state: &AppState) {
    let Some(m) = state.month() else { return };

    // Summary
    let s = compute_summary(m);
    window.set_sum_income(fmt_sign(s.income).into());
    window.set_sum_income_b(fmt(s.income_b).into());
    window.set_sum_income_c(fmt(s.income_c).into());
    window.set_sum_income_col(color_for(s.income));
    window.set_sum_paid(fmt(s.paid).into());
    window.set_sum_paid_b(fmt(s.paid_b).into());
    window.set_sum_paid_c(fmt(s.paid_c).into());
    window.set_sum_topay(fmt(s.topay).into());
    window.set_sum_topay_b(fmt(s.topay_b).into());
    window.set_sum_topay_c(fmt(s.topay_c).into());
    window.set_sum_forecast(fmt_sign(s.forecast).into());
    window.set_sum_forecast_b(fmt(s.forecast_b).into());
    window.set_sum_forecast_c(fmt(s.forecast_c).into());
    window.set_sum_forecast_col(color_for(s.forecast));
    window.set_sum_balance(fmt_sign(s.balance).into());
    window.set_sum_balance_b(fmt(s.balance_b).into());
    window.set_sum_balance_c(fmt(s.balance_c).into());
    window.set_sum_balance_col(color_for(s.balance));

    // Withdrawals
    let ret_a_retirer = m.retraits.iter().map(|l| l.banque).sum::<f64>();
    window.set_sum_withdrawals(fmt(ret_a_retirer).into());

    // Bar chart data
    let fix_budget = m.fixes.iter().map(|l| l.banque + l.cash).sum::<f64>();
    let var_budget = m.variables.iter().map(|l| l.banque + l.cash).sum::<f64>();
    let fix_paid   = m.fixes.iter().map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let var_paid   = m.variables.iter().map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let ret_paid   = m.retraits.iter().map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let chart_max  = [ret_a_retirer, fix_budget, var_budget].iter().cloned().fold(1.0_f64, f64::max);
    window.set_chart_ret_budget(ret_a_retirer as f32);
    window.set_chart_ret_paid(ret_paid as f32);
    window.set_chart_fix_budget(fix_budget as f32);
    window.set_chart_fix_paid(fix_paid as f32);
    window.set_chart_var_budget(var_budget as f32);
    window.set_chart_var_paid(var_paid as f32);
    window.set_chart_max(chart_max as f32);

    window.set_month_name(m.name.clone().into());

    // Sections open state
    window.set_sections_open(make_bool_model(&state.sections_open));

    // Lines + payments per section
    window.set_sec0_lines(make_line_items(&m.revenus));
    window.set_sec1_lines(make_line_items(&m.retraits));
    window.set_sec2_lines(make_line_items(&m.fixes));
    window.set_sec3_lines(make_line_items(&m.variables));

    window.set_sec0_payments(make_payment_items(&m.revenus));
    window.set_sec1_payments(make_payment_items(&m.retraits));
    window.set_sec2_payments(make_payment_items(&m.fixes));
    window.set_sec3_payments(make_payment_items(&m.variables));

    let empty = vec![];
    window.set_sec0_expanded(make_bool_model(state.lines_expanded.get(0).unwrap_or(&empty)));
    window.set_sec1_expanded(make_bool_model(state.lines_expanded.get(1).unwrap_or(&empty)));
    window.set_sec2_expanded(make_bool_model(state.lines_expanded.get(2).unwrap_or(&empty)));
    window.set_sec3_expanded(make_bool_model(state.lines_expanded.get(3).unwrap_or(&empty)));
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    run();
}

#[cfg(not(target_os = "android"))]
fn main() { run(); }

fn run() {
    let state = Arc::new(Mutex::new(AppState::new()));
    let window = AppWindow::new().unwrap();

    // Initial push
    {
        let st = state.lock().unwrap();
        window.set_current_tab(month_key_to_tab(&st.current_month));
        window.set_profile_name(st.storage.active_profile().name.clone().into());
        window.set_currency(st.storage.cfg.currency.clone().into());
        window.set_sync_status(status_str(st.storage.status()).into());
        push_month(&window, &st);
    }

    // Tab change
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_tab_changed(move |tab| {
            let w = ww.unwrap();
            w.set_current_tab(tab);
            if let Some(key) = tab_to_month_key(tab) {
                let mut st = state_ref.lock().unwrap();
                st.current_month = key.to_string();
                st.sections_open = [false; 4];
                st.lines_expanded = [vec![], vec![], vec![], vec![]];
                st.ensure_expanded();
                push_month(&w, &st);
            }
        });
    }

    // Toggle section
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_toggle_section(move |si| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = si as usize;
            st.sections_open[i] = !st.sections_open[i];
            st.ensure_expanded();
            push_month(&w, &st);
        });
    }

    // Toggle line
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_toggle_line(move |si, li| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.ensure_expanded();
            let exp = &mut st.lines_expanded[si as usize];
            if let Some(v) = exp.get_mut(li as usize) { *v = !*v; }
            push_month(&w, &st);
        });
    }

    // Update banque
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_banque(move |si, li, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            if let Ok(v) = val.parse::<f64>() {
                st.sec_lines_mut(si as usize)[li as usize].banque = v;
                st.storage.save();
                push_month(&w, &st);
            }
        });
    }

    // Update cash
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_cash(move |si, li, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            if let Ok(v) = val.parse::<f64>() {
                st.sec_lines_mut(si as usize)[li as usize].cash = v;
                st.storage.save();
                push_month(&w, &st);
            }
        });
    }

    // Update name
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_name(move |si, li, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.sec_lines_mut(si as usize)[li as usize].name = val.to_string();
            st.storage.save();
            push_month(&w, &st);
        });
    }

    // Delete line
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_line(move |si, li| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let sec = st.sec_lines_mut(si as usize);
            if (li as usize) < sec.len() { sec.remove(li as usize); }
            st.lines_expanded[si as usize] = vec![];
            st.ensure_expanded();
            st.storage.save();
            push_month(&w, &st);
        });
    }

    // Add line
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_line(move |si| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.sec_lines_mut(si as usize).push(Line::new("New entry"));
            st.sections_open[si as usize] = true;
            st.lines_expanded[si as usize] = vec![];
            st.ensure_expanded();
            st.storage.save();
            push_month(&w, &st);
        });
    }

    // Delete payment
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_pay(move |si, li, pi| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let line = &mut st.sec_lines_mut(si as usize)[li as usize];
            if (pi as usize) < line.payments.len() { line.payments.remove(pi as usize); }
            st.storage.save();
            push_month(&w, &st);
        });
    }

    // Add payment
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_pay(move |si, li, date, amt| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            if let Ok(amount) = amt.parse::<f64>() {
                let d = if date.is_empty() { today() } else { date.to_string() };
                st.sec_lines_mut(si as usize)[li as usize].payments.push(Payment { date: d, amount });
                st.storage.save();
                push_month(&w, &st);
            }
        });
    }

    // Save
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_save_requested(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let status = st.storage.save();
            w.set_sync_status(status_str(status).into());
            w.set_toast_message("Saved".into());
            w.set_toast_show(true);
            let w2 = w.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(2500), move || {
                if let Some(w) = w2.upgrade() { w.set_toast_show(false); }
            });
        });
    }

    // Toggle theme
    {
        let ww = window.as_weak();
        window.on_toggle_theme(move || {
            let w = ww.unwrap();
            w.set_is_dark(!w.get_is_dark());
        });
    }

    // Go config
    {
        let ww = window.as_weak();
        window.on_go_config(move || { ww.unwrap().set_current_tab(Tab::Config); });
    }

    window.run().unwrap();
}
