slint::include_modules!();

mod model;
mod storage;
mod theme;
mod i18n;

use model::{detect_budget_month, fmt, fmt_sign, today, Month, Payment, Line, Recurring, Dette, SavingsProject, SavingsRow, FraisLine, MONTHS};
use storage::{Storage, SyncStatus, save_config};
use theme::{DARK, LIGHT};

use std::sync::{Arc, Mutex};
use slint::{SharedString, VecModel, ModelRc};

// --- App state shared across callbacks
struct AppState {
    storage:         Storage,
    current_month:   String,
    sections_open:   [bool; 4],
    lines_expanded:  [Vec<bool>; 4],
    register_asc:    bool,
}

impl AppState {
    fn new() -> Self {
        let mut storage = Storage::new();
        let status = storage.load();
        // Auto-connect : marquer dav_ok si le load a réussi via DAV
        if status == SyncStatus::Dav {
            storage.dav_ok = true;
        }
        let current_month = detect_budget_month().to_string();
        Self {
            storage,
            current_month,
            sections_open:  [false; 4],
            lines_expanded: [vec![], vec![], vec![], vec![]],
            register_asc:   false,
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
        let has_rec = l.recurring.is_some();
        let rec_freq = l.recurring.as_ref().map(|r| r.freq as i32).unwrap_or(0);
        LineItem {
            name:   l.name.clone().into(),
            banque: l.banque as f32,
            cash:   l.cash as f32,
            etat:   fmt(etat).into(),
            solde:  fmt(solde).into(),
            solde_val: solde as f32,
            idx:    i as i32,
            has_recurring: has_rec,
            recurring_freq: rec_freq,
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

fn show_toast(window: &AppWindow, msg: &str) {
    window.set_toast_message(msg.into());
    window.set_toast_show(true);
    let w2 = window.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(2500), move || {
        if let Some(w) = w2.upgrade() { w.set_toast_show(false); }
    });
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

    // Per-section totals (budget = banque+cash, paid, remaining)
    let income_total = m.revenus.iter().map(|l| l.banque + l.cash).sum::<f64>();
    let ret_total    = m.retraits.iter().map(|l| l.banque).sum::<f64>();
    let fix_budget   = m.fixes.iter().map(|l| l.banque + l.cash).sum::<f64>();
    let var_budget   = m.variables.iter().map(|l| l.banque + l.cash).sum::<f64>();
    let fix_paid     = m.fixes.iter().map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let var_paid     = m.variables.iter().map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();
    let ret_paid     = m.retraits.iter().map(|l| l.payments.iter().map(|p| p.amount).sum::<f64>()).sum::<f64>();

    window.set_sum_sec_income(fmt(income_total).into());
    window.set_sum_sec_withdrawals(fmt(ret_total).into());
    window.set_sum_withdrawals(fmt(ret_total).into());
    window.set_sum_sec_fixed(fmt(fix_budget).into());
    window.set_sum_sec_variable(fmt(var_budget).into());

    // Bar chart data
    let chart_max = [ret_total, fix_budget, var_budget].iter().cloned().fold(1.0_f64, f64::max);
    window.set_chart_ret_budget(ret_total as f32);
    window.set_chart_ret_paid(ret_paid as f32);
    window.set_chart_fix_budget(fix_budget as f32);
    window.set_chart_fix_paid(fix_paid as f32);
    window.set_chart_var_budget(var_budget as f32);
    window.set_chart_var_paid(var_paid as f32);
    window.set_chart_max(chart_max as f32);

    // Translate month name
    let month_keys = ["jan","feb","mar","apr","mai","jun","jul","aug","sep","oct","nov","dec"];
    let mi = model::MONTHS.iter().position(|&x| x == state.current_month.as_str()).unwrap_or(0);
    let tr = i18n::get_translations(&state.storage.cfg.lang);
    let translated_month = tr.get(month_keys[mi]).copied().unwrap_or(&state.current_month);
    window.set_month_name(translated_month.into());

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

    // Register
    push_register(window, state);
}

// --- Push register (all payments sorted by date)
fn push_register(window: &AppWindow, state: &AppState) {
    let Some(m) = state.month() else { return };
    let sec_names = ["Income", "Withdrawal", "Fixed", "Variable"];
    let sections: [&Vec<Line>; 4] = [&m.revenus, &m.retraits, &m.fixes, &m.variables];

    let mut entries: Vec<(String, String, String, f64)> = vec![];
    for (si, lines) in sections.iter().enumerate() {
        for line in lines.iter() {
            for p in &line.payments {
                entries.push((
                    p.date.clone(),
                    line.name.clone(),
                    sec_names[si].to_string(),
                    p.amount,
                ));
            }
        }
    }

    if state.register_asc {
        entries.sort_by(|a, b| a.0.cmp(&b.0));
    } else {
        entries.sort_by(|a, b| b.0.cmp(&a.0));
    }

    let items: Vec<RegisterEntry> = entries.iter().map(|(d, l, s, a)| {
        RegisterEntry {
            date:    d.clone().into(),
            label:   l.clone().into(),
            section: s.clone().into(),
            amount:  *a as f32,
        }
    }).collect();

    window.set_register_entries(ModelRc::new(VecModel::from(items)));
    window.set_register_asc(state.register_asc);
}

// --- Push annual charts data to window (port of charts_view.py)
fn push_charts(window: &AppWindow, state: &AppState) {
    let data = &state.storage.data;
    let currency = &state.storage.cfg.currency;

    let mut tot_rev = 0.0_f64;
    let mut tot_paye = 0.0_f64;
    let mut tot_a_payer = 0.0_f64;
    let mut tot_ret = 0.0_f64;
    let mut tot_prev = 0.0_f64;
    let mut tot_solde = 0.0_f64;

    let month_labels: Vec<&str> = MONTHS.iter().copied().collect();

    struct RowData { rev: f64, dep: f64, prev: f64 }
    let mut rows: Vec<RowData> = Vec::with_capacity(12);

    for &mk in &MONTHS {
        let m = match data.months.get(mk) {
            Some(m) => m,
            None => { rows.push(RowData { rev: 0.0, dep: 0.0, prev: 0.0 }); continue; }
        };

        let rev_banque: f64 = m.revenus.iter().map(|l| l.banque).sum();
        let rev_cash: f64   = m.revenus.iter().map(|l| l.cash).sum();
        let rev_total       = rev_banque + rev_cash;

        let ret_a_retirer: f64 = m.retraits.iter().map(|l| l.banque).sum();
        let ret_retire: f64    = m.retraits.iter().map(|l| l.etat()).sum();

        let all_dep: Vec<&Line> = m.fixes.iter().chain(m.variables.iter()).collect();
        let dep_banque: f64 = all_dep.iter().map(|l| l.banque).sum();
        let dep_cash: f64   = all_dep.iter().map(|l| l.cash).sum();

        let rev_recu_b: f64 = m.revenus.iter().filter(|l| l.banque > 0.01).map(|l| l.etat()).sum();
        let rev_recu_c: f64 = m.revenus.iter().filter(|l| l.cash > 0.01).map(|l| l.etat()).sum();
        let paye_banque: f64 = all_dep.iter().filter(|l| l.banque > 0.01).map(|l| l.etat()).sum::<f64>() + ret_retire;
        let paye_cash: f64   = all_dep.iter().filter(|l| l.cash > 0.01).map(|l| l.etat()).sum();
        let paye_total       = paye_banque + paye_cash;

        let a_payer_b = dep_banque - all_dep.iter().filter(|l| l.banque > 0.01).map(|l| l.etat()).sum::<f64>();
        let a_payer_c = dep_cash   - all_dep.iter().filter(|l| l.cash > 0.01).map(|l| l.etat()).sum::<f64>();

        let prev_banque = rev_banque - dep_banque - ret_a_retirer;
        let prev_cash   = rev_cash + ret_a_retirer - dep_cash;
        let prev_total  = prev_banque + prev_cash;

        let solde_banque = rev_recu_b - paye_banque;
        let solde_cash   = ret_retire + rev_recu_c - paye_cash;
        let solde_total  = solde_banque + solde_cash;

        tot_rev     += rev_total;
        tot_ret     += ret_a_retirer;
        tot_paye    += paye_total;
        tot_a_payer += a_payer_b + a_payer_c;
        tot_prev    += prev_total;
        tot_solde   += solde_total;

        rows.push(RowData { rev: rev_total, dep: dep_banque + dep_cash, prev: prev_total });
    }

    // 6 summary cards
    window.set_charts_total_income(format!("{} {}", fmt_sign(tot_rev), currency).into());
    window.set_charts_total_paid(format!("{} {}", fmt(tot_paye), currency).into());
    window.set_charts_total_to_pay(format!("{} {}", fmt(tot_a_payer), currency).into());
    window.set_charts_total_withdrawals(format!("{} {}", fmt(tot_ret), currency).into());
    window.set_charts_total_forecast(format!("{} {}", fmt_sign(tot_prev), currency).into());
    window.set_charts_total_balance(format!("{} {}", fmt_sign(tot_solde), currency).into());

    window.set_charts_income_color(color_for(tot_rev));
    window.set_charts_paid_color(hex_color("#85CDCA")); // teal
    window.set_charts_to_pay_color(color_for(-tot_a_payer));
    window.set_charts_withdrawals_color(hex_color("#F2D388")); // amber
    window.set_charts_forecast_color(color_for(tot_prev));
    window.set_charts_balance_color(color_for(tot_solde));

    // Monthly rows
    let mut r_months: Vec<SharedString> = Vec::with_capacity(12);
    let mut r_incomes: Vec<SharedString> = Vec::with_capacity(12);
    let mut r_expenses: Vec<SharedString> = Vec::with_capacity(12);
    let mut r_forecasts: Vec<SharedString> = Vec::with_capacity(12);
    let mut r_fcols: Vec<slint::Color> = Vec::with_capacity(12);

    let mut cumul = 0.0_f64;
    for (i, rd) in rows.iter().enumerate() {
        cumul += rd.prev;
        r_months.push(month_labels[i].into());
        r_incomes.push(fmt(rd.rev).into());
        r_expenses.push(fmt(rd.dep).into());
        r_forecasts.push(fmt_sign(rd.prev).into());
        r_fcols.push(color_for(rd.prev));
    }

    window.set_charts_row_months(ModelRc::new(VecModel::from(r_months)));
    window.set_charts_row_incomes(ModelRc::new(VecModel::from(r_incomes)));
    window.set_charts_row_expenses(ModelRc::new(VecModel::from(r_expenses)));
    window.set_charts_row_forecasts(ModelRc::new(VecModel::from(r_forecasts)));
    window.set_charts_row_forecast_colors(ModelRc::new(VecModel::from(r_fcols)));

    // Annual cumulative
    window.set_charts_annual_cumul(format!("{}", fmt_sign(cumul)).into());
    window.set_charts_annual_cumul_color(color_for(cumul));
}

// --- Push debts data to window (port of build_dettes_view)
fn push_debts(window: &AppWindow, state: &AppState) {
    let dettes = &state.storage.data.dettes;

    let total_due: f64 = dettes.iter().map(|d| d.solde).sum();
    let total_neg: f64 = dettes.iter().map(|d| d.solde_ok).sum();
    let settled: i32   = dettes.iter().filter(|d| d.solde_ok >= d.solde && d.solde > 0.0).count() as i32;

    window.set_debts_total_due(fmt(total_due).into());
    window.set_debts_total_neg(fmt(total_neg).into());
    window.set_debts_settled(settled);

    let items: Vec<DebtItem> = dettes.iter().enumerate().map(|(i, d)| {
        DebtItem {
            rep:        d.rep.clone().into(),
            creditor:   d.creancier.clone().into(),
            pursuit:    d.poursuite.clone().into(),
            balance:    d.solde as f32,
            balance_ok: d.solde_ok as f32,
            status:     d.etat.clone().into(),
            date:       d.date.clone().into(),
            idx:        i as i32,
        }
    }).collect();
    window.set_debts_list(ModelRc::new(VecModel::from(items)));
}

// --- Push savings data to window (projects with sub-rows)
fn push_savings(window: &AppWindow, state: &AppState) {
    let projects = &state.storage.data.epargne.savings;
    let items: Vec<SavingsEntry> = projects.iter().enumerate().map(|(i, p)| {
        let has_rows = !p.rows.is_empty();
        let rows_total: f64 = p.rows.iter().map(|r| r.montant).sum();
        let rows_cible: f64 = p.rows.iter().map(|r| r.cible).sum();
        let total = if has_rows { rows_total } else { p.montant };
        let cible = if has_rows && rows_cible > 0.01 { rows_cible } else { p.cible };
        let pct = if cible > 0.01 { ((total / cible) * 100.0).min(100.0) as i32 } else { 0 };

        let row_items: Vec<SavingsRowItem> = p.rows.iter().enumerate().map(|(ri, r)| {
            let rpct = if r.cible > 0.01 { ((r.montant / r.cible) * 100.0).min(100.0) as i32 } else { 0 };
            SavingsRowItem {
                name: r.name.clone().into(),
                montant: r.montant as f32,
                cible: r.cible as f32,
                percent: rpct,
                idx: ri as i32,
            }
        }).collect();

        SavingsEntry {
            label:   p.label.clone().into(),
            montant: total as f32,
            cible:   cible as f32,
            percent: pct,
            idx:     i as i32,
            open:    p.open,
            rows:    ModelRc::new(VecModel::from(row_items)),
        }
    }).collect();
    window.set_savings_list(ModelRc::new(VecModel::from(items)));
}

// --- Sync Frais from monthly data (fully automatic, replaces manual frais entries)
fn sync_frais_from_months(state: &mut AppState) {
    use model::FraisLine;
    use std::collections::HashMap;

    // sec_idx → (name → [f64; 12])
    let mut fixes:    HashMap<String, [f64; 12]> = HashMap::new();
    let mut ponctuels: HashMap<String, [f64; 12]> = HashMap::new();
    let mut retraits:  HashMap<String, [f64; 12]> = HashMap::new();

    for (mi, &mk) in MONTHS.iter().enumerate() {
        let m = match state.storage.data.months.get(mk) {
            Some(m) => m,
            None => continue,
        };
        for line in &m.fixes {
            let entry = fixes.entry(line.name.clone()).or_insert([0.0; 12]);
            entry[mi] += line.banque + line.cash;
        }
        for line in &m.variables {
            let entry = ponctuels.entry(line.name.clone()).or_insert([0.0; 12]);
            entry[mi] += line.banque + line.cash;
        }
        for line in &m.retraits {
            let entry = retraits.entry(line.name.clone()).or_insert([0.0; 12]);
            entry[mi] += line.banque;
        }
    }

    let to_vec = |map: HashMap<String, [f64; 12]>| -> Vec<FraisLine> {
        let mut v: Vec<FraisLine> = map.into_iter().map(|(name, monthly)| FraisLine { name, monthly }).collect();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    };

    state.storage.data.frais.fixes    = to_vec(fixes);
    state.storage.data.frais.ponctuels = to_vec(ponctuels);
    state.storage.data.frais.retraits  = to_vec(retraits);
}

// --- Push expenses (frais) data to window
fn make_expense_section(label: &str, lines: &[FraisLine]) -> ExpenseSection {
    let mut sec_total = 0.0_f64;
    let el: Vec<ExpenseLine> = lines.iter().map(|fl| {
        let t: f64 = fl.monthly.iter().sum();
        sec_total += t;
        ExpenseLine {
            name:  fl.name.clone().into(),
            m0:  fl.monthly[0]  as f32, m1:  fl.monthly[1]  as f32,
            m2:  fl.monthly[2]  as f32, m3:  fl.monthly[3]  as f32,
            m4:  fl.monthly[4]  as f32, m5:  fl.monthly[5]  as f32,
            m6:  fl.monthly[6]  as f32, m7:  fl.monthly[7]  as f32,
            m8:  fl.monthly[8]  as f32, m9:  fl.monthly[9]  as f32,
            m10: fl.monthly[10] as f32, m11: fl.monthly[11] as f32,
            total: t as f32,
        }
    }).collect();
    ExpenseSection {
        label: label.into(),
        lines: ModelRc::new(VecModel::from(el)),
        total: sec_total as f32,
    }
}

fn push_expenses(window: &AppWindow, state: &AppState) {
    let frais = &state.storage.data.frais;
    let tr = i18n::get_translations(&state.storage.cfg.lang);
    let l = |k: &str| -> String { tr.get(k).copied().unwrap_or(k).to_string() };
    window.set_expenses_fixed(make_expense_section(&l("sec_fixed"), &frais.fixes));
    window.set_expenses_occasional(make_expense_section(&l("sec_variable"), &frais.ponctuels));
    window.set_expenses_withdrawals(make_expense_section(&l("sec_withdrawals"), &frais.retraits));
}

// --- Push viability data to window
fn push_viability(window: &AppWindow, state: &AppState) {
    let vc = &state.storage.data.viabilite;

    let cols: Vec<ViaColumn> = vc.colonnes.iter().enumerate().map(|(i, c)| {
        ViaColumn {
            name: c.name.clone().into(),
            is_income: c.is_income,
            delta_type: c.delta_type.clone().into(),
            idx: i as i32,
        }
    }).collect();
    window.set_via_columns(ModelRc::new(VecModel::from(cols)));

    let pals: Vec<ViaPalier> = vc.paliers.iter().enumerate().map(|(pi, p)| {
        let cells: Vec<ViaPalierCell> = p.valeurs.iter().map(|&v| {
            ViaPalierCell { value: v as f32 }
        }).collect();
        let balance: f64 = vc.colonnes.iter().enumerate().map(|(ci, c)| {
            let v = p.valeurs.get(ci).copied().unwrap_or(0.0);
            if c.is_income { v } else { -v }
        }).sum();
        ViaPalier {
            cells: ModelRc::new(VecModel::from(cells)),
            balance: balance as f32,
            idx: pi as i32,
        }
    }).collect();
    window.set_via_paliers(ModelRc::new(VecModel::from(pals)));
    window.set_via_n_paliers(vc.n_paliers as i32);
}

// --- Apply theme colors to Palette global
fn apply_theme(window: &AppWindow, is_dark: bool) {
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

// --- Push settings data to window
fn push_settings(window: &AppWindow, state: &AppState) {
    let cfg = &state.storage.cfg;
    let active_slug = &cfg.active;
    let items: Vec<ProfileItem> = cfg.profiles.iter().enumerate().map(|(i, p)| {
        ProfileItem {
            name:   p.name.clone().into(),
            slug:   p.slug.clone().into(),
            active: p.slug == *active_slug,
            idx:    i as i32,
        }
    }).collect();
    window.set_settings_profiles(ModelRc::new(VecModel::from(items)));

    let prof = state.storage.active_profile();
    // DAV1
    window.set_settings_dav_url(prof.dav_url.clone().into());
    window.set_settings_dav_user(prof.dav_user.clone().into());
    window.set_settings_dav_pass(prof.dav_pass.clone().into());
    // DAV2
    window.set_settings_dav2_url(prof.dav2_url.clone().into());
    window.set_settings_dav2_user(prof.dav2_user.clone().into());
    window.set_settings_dav2_pass(prof.dav2_pass.clone().into());
    window.set_settings_dav2_enabled(prof.dav2_enabled);

    window.set_settings_currency_edit(cfg.currency.clone().into());
    window.set_profile_name(prof.name.clone().into());
    window.set_currency(cfg.currency.clone().into());
    // Backup
    window.set_settings_backup_local(cfg.backup_local);
    window.set_settings_backup_webdav(cfg.backup_webdav);
    window.set_settings_data_dir_display(state.storage.data_dir_display().into());
    window.set_settings_backup_dir_display(state.storage.backup_dir_display().into());
    window.set_settings_data_dir_edit(cfg.data_dir.clone().into());
}

// --- Push i18n strings to I18n global
fn push_i18n(window: &AppWindow, lang: &str) {
    let tr = i18n::get_translations(lang);
    let i = I18n::get(window);
    macro_rules! set {
        ($prop:ident, $key:expr) => {
            if let Some(&v) = tr.get($key) {
                i.$prop(slint::SharedString::from(v));
            }
        };
    }
    set!(set_jan,"jan"); set!(set_feb,"feb"); set!(set_mar,"mar"); set!(set_apr,"apr");
    set!(set_mai,"mai"); set!(set_jun,"jun"); set!(set_jul,"jul"); set!(set_aug,"aug");
    set!(set_sep,"sep"); set!(set_oct,"oct"); set!(set_nov,"nov"); set!(set_dec,"dec");
    set!(set_tab_debts,"tab_debts"); set!(set_tab_savings,"tab_savings");
    set!(set_tab_expenses,"tab_expenses"); set!(set_tab_viability,"tab_viability");
    set!(set_tab_charts,"tab_charts"); set!(set_tab_config,"tab_config");
    set!(set_card_income,"card_income"); set!(set_card_withdrawals,"card_withdrawals");
    set!(set_card_paid,"card_paid"); set!(set_card_to_pay,"card_to_pay");
    set!(set_card_forecast,"card_forecast"); set!(set_card_balance,"card_balance");
    set!(set_col_bank,"col_bank"); set!(set_col_cash,"col_cash"); set!(set_col_total,"col_total");
    set!(set_col_to_withdraw,"col_to_withdraw"); set!(set_col_withdrawn,"col_withdrawn");
    set!(set_sec_income,"sec_income"); set!(set_sec_withdrawals,"sec_withdrawals");
    set!(set_sec_fixed,"sec_fixed"); set!(set_sec_variable,"sec_variable");
    set!(set_col_bank_hdr,"col_bank_hdr"); set!(set_col_cash_hdr,"col_cash_hdr");
    set!(set_col_paid,"col_paid"); set!(set_col_left,"col_left");
    set!(set_chart_budget_vs,"chart_budget_vs"); set!(set_chart_withdrawals,"chart_withdrawals");
    set!(set_chart_fixed,"chart_fixed"); set!(set_chart_variable,"chart_variable");
    set!(set_reg_title,"reg_title"); set!(set_reg_date_asc,"reg_date_asc"); set!(set_reg_date_desc,"reg_date_desc");
    set!(set_reg_date,"reg_date"); set!(set_reg_label,"reg_label");
    set!(set_reg_section,"reg_section"); set!(set_reg_amount,"reg_amount");
    set!(set_reg_no_payments,"reg_no_payments");
    set!(set_pay_date,"pay_date"); set!(set_pay_amount,"pay_amount"); set!(set_pay_no,"pay_no");
    set!(set_add_entry,"add_entry"); set!(set_new_entry,"new_entry");
    set!(set_rec_title,"rec_title"); set!(set_rec_frequency,"rec_frequency");
    set!(set_rec_every_1,"rec_every_1"); set!(set_rec_every_2,"rec_every_2");
    set!(set_rec_every_3,"rec_every_3"); set!(set_rec_every_6,"rec_every_6");
    set!(set_rec_every_12,"rec_every_12"); set!(set_rec_past,"rec_past");
    set!(set_rec_cancel,"rec_cancel"); set!(set_rec_disable,"rec_disable"); set!(set_rec_apply,"rec_apply");
    set!(set_deb_title,"deb_title"); set!(set_deb_total_due,"deb_total_due");
    set!(set_deb_negotiated,"deb_negotiated"); set!(set_deb_settled,"deb_settled");
    set!(set_deb_add,"deb_add"); set!(set_deb_rep,"deb_rep"); set!(set_deb_pursuit,"deb_pursuit");
    set!(set_deb_due,"deb_due"); set!(set_deb_neg,"deb_neg");
    set!(set_deb_status,"deb_status"); set!(set_deb_date,"deb_date");
    set!(set_sav_title,"sav_title"); set!(set_sav_add_project,"sav_add_project"); set!(set_sav_add_entry,"sav_add_entry");
    set!(set_exp_title,"exp_title"); set!(set_exp_name,"exp_name"); set!(set_exp_total,"exp_total");
    set!(set_via_title,"via_title"); set!(set_via_subtitle,"via_subtitle");
    set!(set_via_columns,"via_columns"); set!(set_via_add_col,"via_add_col");
    set!(set_via_generate,"via_generate"); set!(set_via_clear,"via_clear");
    set!(set_via_add_bracket,"via_add_bracket"); set!(set_via_no_brackets,"via_no_brackets");
    set!(set_via_balance,"via_balance");
    set!(set_charts_title,"charts_title"); set!(set_charts_income,"charts_income");
    set!(set_charts_paid,"charts_paid"); set!(set_charts_to_pay,"charts_to_pay");
    set!(set_charts_withdrawals,"charts_withdrawals"); set!(set_charts_forecast,"charts_forecast");
    set!(set_charts_balance,"charts_balance"); set!(set_charts_cumul,"charts_cumul");
    set!(set_charts_exp,"charts_exp");
    set!(set_cfg_title,"cfg_title"); set!(set_cfg_profiles,"cfg_profiles");
    set!(set_cfg_add_profile,"cfg_add_profile"); set!(set_cfg_use,"cfg_use");
    set!(set_cfg_currency,"cfg_currency"); set!(set_cfg_theme,"cfg_theme");
    set!(set_cfg_dark_to_light,"cfg_dark_to_light"); set!(set_cfg_light_to_dark,"cfg_light_to_dark");
    set!(set_cfg_webdav,"cfg_webdav"); set!(set_cfg_url,"cfg_url");
    set!(set_cfg_user,"cfg_user"); set!(set_cfg_password,"cfg_password");
    set!(set_cfg_save,"cfg_save"); set!(set_cfg_test,"cfg_test"); set!(set_cfg_clear,"cfg_clear");
    set!(set_cfg_export,"cfg_export"); set!(set_cfg_export_btn,"cfg_export_btn");
    set!(set_cfg_import,"cfg_import"); set!(set_cfg_import_btn,"cfg_import_btn");
    set!(set_cfg_data,"cfg_data"); set!(set_cfg_reset,"cfg_reset");
    set!(set_cfg_export_desc,"cfg_export_desc"); set!(set_cfg_import_desc,"cfg_import_desc");
    set!(set_cfg_statement_title,"cfg_statement_title");
    set!(set_cfg_statement_import_desc,"cfg_statement_import_desc");
    set!(set_cfg_statement_export_desc,"cfg_statement_export_desc");
    set!(set_cfg_import_csv,"cfg_import_csv"); set!(set_cfg_import_ofx,"cfg_import_ofx");
    set!(set_cfg_export_csv,"cfg_export_csv"); set!(set_cfg_export_ofx,"cfg_export_ofx");
    set!(set_cfg_hash_title,"cfg_hash_title"); set!(set_cfg_hash_desc,"cfg_hash_desc");
    set!(set_cfg_hash_data,"cfg_hash_data"); set!(set_cfg_hash_exe,"cfg_hash_exe");
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    if let Some(path) = app.internal_data_path() {
        storage::set_android_data_dir(path);
    }
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
            {
                let st = state_ref.lock().unwrap();
                match tab {
                    Tab::Charts   => push_charts(&w, &st),
                    Tab::Debts    => push_debts(&w, &st),
                    Tab::Savings  => push_savings(&w, &st),
                    Tab::Expenses  => push_expenses(&w, &st),
                    Tab::Viability => push_viability(&w, &st),
                    Tab::Config    => push_settings(&w, &st),
                    _ => {}
                }
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
                sync_frais_from_months(&mut st);
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
                sync_frais_from_months(&mut st);
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
            st.sec_lines_mut(si as usize).sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            st.lines_expanded[si as usize] = vec![];
            st.ensure_expanded();
            sync_frais_from_months(&mut st);
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
            let mk = st.current_month.clone();
            let src_mi = MONTHS.iter().position(|&m| m == mk.as_str()).unwrap_or(0);
            let sec_key = match si as usize { 0 => "revenus", 1 => "retraits", 2 => "fixes", _ => "variables" };

            let recurring_info = st.sec_lines(si as usize).get(li as usize)
                .and_then(|l| l.recurring.as_ref().map(|r| (l.name.clone(), r.freq as usize, r.start.clone())));
            if let Some((name, old_freq, start)) = recurring_info {
                let start_mi = MONTHS.iter().position(|&m| m == start.as_str()).unwrap_or(src_mi);
                if start_mi != src_mi {
                    let origin_key = MONTHS[start_mi].to_string();
                    if let Some(om) = st.storage.data.months.get_mut(&origin_key) {
                        let sec = om.section_mut(sec_key);
                        if start_mi < src_mi {
                            for l in sec.iter_mut() { if l.name == name { l.recurring = None; } }
                        } else {
                            sec.retain(|l| !(l.name == name && l.recurring.is_some()));
                        }
                    }
                }
                for step in 1..13 {
                    let target_mi = (start_mi + step * old_freq) % 12;
                    if target_mi == start_mi { break; }
                    if target_mi == src_mi { continue; }
                    let target_key = MONTHS[target_mi].to_string();
                    if let Some(tm) = st.storage.data.months.get_mut(&target_key) {
                        let sec = tm.section_mut(sec_key);
                        if target_mi < src_mi {
                            for l in sec.iter_mut() { if l.name == name { l.recurring = None; } }
                        } else {
                            sec.retain(|l| !(l.name == name && l.recurring.is_some()));
                        }
                    }
                }
            }

            let sec = st.sec_lines_mut(si as usize);
            if (li as usize) < sec.len() { sec.remove(li as usize); }
            st.lines_expanded[si as usize] = vec![];
            st.ensure_expanded();
            sync_frais_from_months(&mut st);
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
            let new_name = "New entry";
            if st.sec_lines(si as usize).iter().any(|l| l.name == new_name) {
                return;
            }
            st.sec_lines_mut(si as usize).push(Line::new(new_name));
            st.sec_lines_mut(si as usize).sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            st.sections_open[si as usize] = true;
            st.lines_expanded[si as usize] = vec![];
            st.ensure_expanded();
            sync_frais_from_months(&mut st);
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
            sync_frais_from_months(&mut st);
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
                sync_frais_from_months(&mut st);
                st.storage.save();
                push_month(&w, &st);
            }
        });
    }

    // Set recurring
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_set_recurring(move |si, li, freq, include_past| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let mk = st.current_month.clone();
            let src_mi = MONTHS.iter().position(|&m| m == mk.as_str()).unwrap_or(0);

            if freq <= 0 {
                let line = &st.sec_lines(si as usize)[li as usize];
                let name = line.name.clone();
                let old_freq = line.recurring.as_ref().map(|r| r.freq as usize).unwrap_or(3);
                let start = line.recurring.as_ref().map(|r| r.start.clone()).unwrap_or(mk.clone());
                let sec_key = match si as usize { 0 => "revenus", 1 => "retraits", 2 => "fixes", _ => "variables" };
                let start_mi = MONTHS.iter().position(|&m| m == start.as_str()).unwrap_or(src_mi);

                let line = &mut st.sec_lines_mut(si as usize)[li as usize];
                line.recurring = None;

                if start_mi != src_mi {
                    let origin_key = MONTHS[start_mi].to_string();
                    if let Some(om) = st.storage.data.months.get_mut(&origin_key) {
                        let sec = om.section_mut(sec_key);
                        if !include_past && start_mi < src_mi {
                            for l in sec.iter_mut() { if l.name == name { l.recurring = None; } }
                        } else {
                            sec.retain(|l| !(l.name == name && l.recurring.is_some()));
                        }
                    }
                }

                for step in 1..13 {
                    let target_mi = (start_mi + step * old_freq) % 12;
                    if target_mi == start_mi { break; }
                    if target_mi == src_mi { continue; }
                    let target_key = MONTHS[target_mi].to_string();
                    if let Some(target_month) = st.storage.data.months.get_mut(&target_key) {
                        let sec = target_month.section_mut(sec_key);
                        if !include_past && target_mi < src_mi {
                            for l in sec.iter_mut() { if l.name == name { l.recurring = None; } }
                        } else {
                            sec.retain(|l| !(l.name == name && l.recurring.is_some()));
                        }
                    }
                }
            } else {
                let line = &st.sec_lines(si as usize)[li as usize];
                let banque = line.banque;
                let cash = line.cash;
                let name = line.name.clone();
                let sec_key = match si as usize { 0 => "revenus", 1 => "retraits", 2 => "fixes", _ => "variables" };

                let line = &mut st.sec_lines_mut(si as usize)[li as usize];
                line.recurring = Some(Recurring {
                    freq: freq as u32,
                    start: mk.clone(),
                });

                let f = freq as usize;
                for step in 1..13 {
                    let target_mi = (src_mi + step * f) % 12;
                    if target_mi == src_mi { break; }
                    if !include_past && target_mi < src_mi { continue; }

                    let target_key = MONTHS[target_mi].to_string();
                    if let Some(target_month) = st.storage.data.months.get_mut(&target_key) {
                        let sec = target_month.section_mut(sec_key);
                        if let Some(existing) = sec.iter_mut().find(|l| l.name == name) {
                            existing.banque = banque;
                            existing.cash = cash;
                            existing.recurring = Some(Recurring { freq: freq as u32, start: mk.clone() });
                        } else {
                            sec.push(Line {
                                name: name.clone(),
                                banque, cash,
                                payments: vec![],
                                recurring: Some(Recurring { freq: freq as u32, start: mk.clone() }),
                            });
                            sec.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                        }
                    }
                }
            }

            sync_frais_from_months(&mut st);
            st.storage.save();
            push_month(&w, &st);
        });
    }

    // Toggle register sort
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_toggle_register_sort(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.register_asc = !st.register_asc;
            push_register(&w, &st);
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
            show_toast(&w, "Saved");
        });
    }

    // Toggle theme
    {
        let ww = window.as_weak();
        window.on_toggle_theme(move || {
            let w = ww.unwrap();
            let new_dark = !w.get_is_dark();
            w.set_is_dark(new_dark);
            apply_theme(&w, new_dark);
        });
    }

    // Toggle lang
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

    // Font scale down
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

    // Font scale up
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

    // Go config
    {
        let ww = window.as_weak();
        window.on_go_config(move || { ww.unwrap().set_current_tab(Tab::Config); });
    }

    // Add debt
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_debt(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.data.dettes.push(Dette::default());
            st.storage.save();
            push_debts(&w, &st);
        });
    }

    // Delete debt
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_debt(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.dettes.len() {
                st.storage.data.dettes.remove(i);
                st.storage.save();
                push_debts(&w, &st);
            }
        });
    }

    // Update debt field
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_debt_field(move |idx, field, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.dettes.len() {
                let d = &mut st.storage.data.dettes[i];
                let f: String = field.into();
                let v: String = val.into();
                match f.as_str() {
                    "creditor"   => d.creancier  = v,
                    "rep"        => d.rep         = v,
                    "pursuit"    => d.poursuite   = v,
                    "balance"    => { if let Ok(n) = v.parse::<f64>() { d.solde = n; } }
                    "balance-ok" => { if let Ok(n) = v.parse::<f64>() { d.solde_ok = n; } }
                    "status"     => d.etat        = v,
                    "date"       => d.date         = v,
                    _ => {}
                }
                st.storage.save();
                push_debts(&w, &st);
            }
        });
    }

    // Add savings project
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_saving(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.data.epargne.savings.push(SavingsProject::default());
            st.storage.save();
            push_savings(&w, &st);
        });
    }

    // Delete savings project
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_saving(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.epargne.savings.len() {
                st.storage.data.epargne.savings.remove(i);
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

    // Update savings project field
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_saving_field(move |idx, field, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.epargne.savings.len() {
                let p = &mut st.storage.data.epargne.savings[i];
                let f: String = field.into();
                let v: String = val.into();
                match f.as_str() {
                    "label"   => p.label = v,
                    "montant" => { if let Ok(n) = v.parse::<f64>() { p.montant = n; } }
                    "cible"   => { if let Ok(n) = v.parse::<f64>() { p.cible = n; } }
                    _ => {}
                }
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

    // Toggle savings project open/close
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_toggle_saving(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.epargne.savings.len() {
                st.storage.data.epargne.savings[i].open = !st.storage.data.epargne.savings[i].open;
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

    // Add savings row
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_saving_row(move |pi| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = pi as usize;
            if i < st.storage.data.epargne.savings.len() {
                st.storage.data.epargne.savings[i].rows.push(SavingsRow::default());
                st.storage.data.epargne.savings[i].rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

    // Delete savings row
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_saving_row(move |pi, ri| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let p = pi as usize;
            let r = ri as usize;
            if p < st.storage.data.epargne.savings.len() && r < st.storage.data.epargne.savings[p].rows.len() {
                st.storage.data.epargne.savings[p].rows.remove(r);
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

    // Update savings row field
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_saving_row_field(move |pi, ri, field, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let p = pi as usize;
            let r = ri as usize;
            if p < st.storage.data.epargne.savings.len() && r < st.storage.data.epargne.savings[p].rows.len() {
                let row = &mut st.storage.data.epargne.savings[p].rows[r];
                let f: String = field.into();
                let v: String = val.into();
                match f.as_str() {
                    "name"    => {
                        row.name = v;
                        st.storage.data.epargne.savings[p].rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    }
                    "montant" => { if let Ok(n) = v.parse::<f64>() { row.montant = n; } }
                    "cible"   => { if let Ok(n) = v.parse::<f64>() { row.cible = n; } }
                    _ => {}
                }
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

    // ── Viability callbacks ────────────────────────────────────────────

    // Add palier
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_via_palier(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let nc = st.storage.data.viabilite.colonnes.len();
            use model::ViabilitePalier;
            st.storage.data.viabilite.paliers.push(ViabilitePalier { valeurs: vec![0.0; nc] });
            st.storage.save();
            push_viability(&w, &st);
        });
    }

    // Delete palier
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_via_palier(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.viabilite.paliers.len() {
                st.storage.data.viabilite.paliers.remove(i);
                st.storage.save();
                push_viability(&w, &st);
            }
        });
    }

    // Update cell
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_via_cell(move |pi, ci, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let p = pi as usize;
            let c = ci as usize;
            let v: String = val.into();
            if p < st.storage.data.viabilite.paliers.len() {
                let pal = &mut st.storage.data.viabilite.paliers[p];
                while pal.valeurs.len() <= c { pal.valeurs.push(0.0); }
                if let Ok(n) = v.parse::<f64>() { pal.valeurs[c] = n; }
                st.storage.save();
                push_viability(&w, &st);
            }
        });
    }

    // Generate paliers
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_generate_via(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let vc = &st.storage.data.viabilite;
            let n = vc.n_paliers as usize;
            let cols = &vc.colonnes;
            if cols.is_empty() { return; }

            let base: Vec<f64> = cols.iter().map(|c| c.valeur).collect();
            let mut paliers = vec![];
            for step in 0..n {
                let vals: Vec<f64> = cols.iter().enumerate().map(|(ci, c)| {
                    let b = base[ci];
                    if c.delta_type == "pct" {
                        (b * (1.0 + c.delta_val / 100.0).powi(step as i32) * 100.0).round() / 100.0
                    } else {
                        ((b + c.delta_val * step as f64) * 100.0).round() / 100.0
                    }
                }).collect();
                use model::ViabilitePalier;
                paliers.push(ViabilitePalier { valeurs: vals });
            }
            st.storage.data.viabilite.paliers = paliers;
            st.storage.save();
            push_viability(&w, &st);
        });
    }

    // Clear paliers
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_clear_via(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.data.viabilite.paliers.clear();
            st.storage.save();
            push_viability(&w, &st);
        });
    }

    // Add column
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_via_column(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            use model::ViabiliteColonne;
            st.storage.data.viabilite.colonnes.push(ViabiliteColonne {
                name: "New".into(), valeur: 0.0, is_income: false,
                delta_type: "fixed".into(), delta_val: 0.0,
            });
            for p in &mut st.storage.data.viabilite.paliers {
                p.valeurs.push(0.0);
            }
            st.storage.save();
            push_viability(&w, &st);
        });
    }

    // Delete column
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_via_column(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.viabilite.colonnes.len() {
                st.storage.data.viabilite.colonnes.remove(i);
                for p in &mut st.storage.data.viabilite.paliers {
                    if i < p.valeurs.len() { p.valeurs.remove(i); }
                }
                st.storage.save();
                push_viability(&w, &st);
            }
        });
    }

    // Update column name
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_via_column_name(move |idx, name| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.viabilite.colonnes.len() {
                st.storage.data.viabilite.colonnes[i].name = name.into();
                st.storage.save();
                push_viability(&w, &st);
            }
        });
    }

    // Toggle column income
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_toggle_via_column_income(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.viabilite.colonnes.len() {
                st.storage.data.viabilite.colonnes[i].is_income = !st.storage.data.viabilite.colonnes[i].is_income;
                st.storage.save();
                push_viability(&w, &st);
            }
        });
    }

    // ── Settings callbacks ─────────────────────────────────────────────

    // Save WebDAV1 config
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_save_dav(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url: String  = w.get_settings_dav_url().into();
            let user: String = w.get_settings_dav_user().into();
            let pass: String = w.get_settings_dav_pass().into();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav(&slug, &url, &user, &pass);
            w.set_settings_dav_status("Config saved ✓".into());
            w.set_sync_status(status_str(st.storage.status()).into());
            show_toast(&w, "Config saved");
        });
    }

    // Test WebDAV1
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_test_dav(move || {
            let ww2 = ww.clone();
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url: String  = w.get_settings_dav_url().into();
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
                    let ww_ = $ww.clone();
                    let msg_ = $msg.to_string();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = ww_.upgrade() { w.set_settings_dav_status(msg_.into()); }
                    });
                }};
            }

            std::thread::spawn(move || {
                let client = match storage::make_client() {
                    Ok(c) => c,
                    Err(e) => { step!(ww3, format!("1/4 FAIL client: {}", e)); return; }
                };
                step!(ww3, "2/4 TCP 1.1.1.1…");

                if let Err(e) = std::net::TcpStream::connect_timeout(
                    &"1.1.1.1:443".parse().unwrap(),
                    std::time::Duration::from_secs(4),
                ) {
                    step!(ww3, format!("2/4 FAIL réseau: {}", e)); return;
                }
                step!(ww3, "3/4 DNS…");

                let host = {
                    let url2 = profile.dav_url.trim().to_string();
                    let s = url2.strip_prefix("https://").or_else(|| url2.strip_prefix("http://")).unwrap_or(&url2).to_string();
                    s[..s.find('/').unwrap_or(s.len())].to_string()
                };
                let (tx2, rx2) = std::sync::mpsc::channel();
                let host2 = host.clone();
                std::thread::spawn(move || {
                    let r = std::net::ToSocketAddrs::to_socket_addrs(&(host2.as_str(), 443u16))
                        .map(|mut it| it.next().map(|a| a.ip().to_string()));
                    let _ = tx2.send(r);
                });
                let ip = match rx2.recv_timeout(std::time::Duration::from_secs(6)) {
                    Ok(Ok(Some(ip))) => ip,
                    Ok(Ok(None))     => { step!(ww3, format!("3/4 FAIL DNS vide: {}", host)); return; }
                    Ok(Err(e))       => { step!(ww3, format!("3/4 FAIL DNS: {}", e)); return; }
                    Err(_)           => { step!(ww3, "3/4 FAIL DNS timeout 6s".to_string()); return; }
                };
                step!(ww3, format!("4/4 HTTPS {} → {}…", host, ip));

                let (ok, msg) = storage::dav_test_http(&profile, client);
                let msg2 = msg.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = ww3.upgrade() {
                        w.set_settings_dav_status(format!("4/4 {}", msg2).into());
                    }
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

    // Clear WebDAV1
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

    // Save WebDAV2 config
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_save_dav2(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url: String     = w.get_settings_dav2_url().into();
            let user: String    = w.get_settings_dav2_user().into();
            let pass: String    = w.get_settings_dav2_pass().into();
            let enabled: bool   = w.get_settings_dav2_enabled();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav2(&slug, &url, &user, &pass, enabled);
            w.set_settings_dav2_status("Config saved ✓".into());
            show_toast(&w, "WebDAV2 saved");
        });
    }

    // Test WebDAV2
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_test_dav2(move || {
            let ww2 = ww.clone();
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let url: String     = w.get_settings_dav2_url().into();
            let user: String    = w.get_settings_dav2_user().into();
            let pass: String    = w.get_settings_dav2_pass().into();
            let enabled: bool   = w.get_settings_dav2_enabled();
            let slug = st.storage.cfg.active.clone();
            st.storage.save_profile_dav2(&slug, &url, &user, &pass, enabled);
            // Construire un profil temporaire avec les crédentials DAV2 en position primaire
            let profile = st.storage.active_profile().as_dav2_profile();
            drop(st);
            w.set_settings_dav2_status("1/4 init client…".into());

            let state_ref2 = state_ref.clone();
            let ww3 = ww2.clone();

            macro_rules! step {
                ($ww:expr, $msg:expr) => {{
                    let ww_ = $ww.clone();
                    let msg_ = $msg.to_string();
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w) = ww_.upgrade() { w.set_settings_dav2_status(msg_.into()); }
                    });
                }};
            }

            std::thread::spawn(move || {
                let client = match storage::make_client() {
                    Ok(c) => c,
                    Err(e) => { step!(ww3, format!("1/4 FAIL client: {}", e)); return; }
                };
                step!(ww3, "2/4 TCP 1.1.1.1…");

                if let Err(e) = std::net::TcpStream::connect_timeout(
                    &"1.1.1.1:443".parse().unwrap(),
                    std::time::Duration::from_secs(4),
                ) {
                    step!(ww3, format!("2/4 FAIL réseau: {}", e)); return;
                }
                step!(ww3, "3/4 DNS…");

                let host = {
                    let url2 = profile.dav_url.trim().to_string();
                    let s = url2.strip_prefix("https://").or_else(|| url2.strip_prefix("http://")).unwrap_or(&url2).to_string();
                    s[..s.find('/').unwrap_or(s.len())].to_string()
                };
                let (tx2, rx2) = std::sync::mpsc::channel();
                let host2 = host.clone();
                std::thread::spawn(move || {
                    let r = std::net::ToSocketAddrs::to_socket_addrs(&(host2.as_str(), 443u16))
                        .map(|mut it| it.next().map(|a| a.ip().to_string()));
                    let _ = tx2.send(r);
                });
                let ip = match rx2.recv_timeout(std::time::Duration::from_secs(6)) {
                    Ok(Ok(Some(ip))) => ip,
                    Ok(Ok(None))     => { step!(ww3, format!("3/4 FAIL DNS vide: {}", host)); return; }
                    Ok(Err(e))       => { step!(ww3, format!("3/4 FAIL DNS: {}", e)); return; }
                    Err(_)           => { step!(ww3, "3/4 FAIL DNS timeout 6s".to_string()); return; }
                };
                step!(ww3, format!("4/4 HTTPS {} → {}…", host, ip));

                let (ok, msg) = storage::dav_test_http(&profile, client);
                let msg2 = msg.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = ww3.upgrade() {
                        w.set_settings_dav2_status(format!("4/4 {}", msg2).into());
                    }
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

    // Clear WebDAV2
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

    // Save currency
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

    // Export JSON — file save dialog
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_do_export(move || {
            let w = ww.unwrap();
            let st = state_ref.lock().unwrap();
            let json = st.storage.export_json();
            let fname = format!("oxycash_{}.json", st.storage.cfg.active);
            drop(st);
            #[cfg(not(target_os = "android"))]
            {
                let path = rfd::FileDialog::new()
                    .set_file_name(&fname)
                    .add_filter("JSON", &["json"])
                    .save_file();
                match path {
                    Some(p) => {
                        match std::fs::write(&p, &json) {
                            Ok(_) => show_toast(&w, "Exported ✓"),
                            Err(e) => show_toast(&w, &format!("Error: {}", e)),
                        }
                    }
                    None => {}
                }
            }
            #[cfg(target_os = "android")]
            {
                let dir = dirs::data_local_dir().unwrap_or_default();
                let path = dir.join(&fname);
                match std::fs::write(&path, &json) {
                    Ok(_) => show_toast(&w, "Exported ✓"),
                    Err(e) => show_toast(&w, &format!("Error: {}", e)),
                }
            }
        });
    }

    // Import JSON — file open dialog
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_do_import(move || {
            let w = ww.unwrap();
            #[cfg(not(target_os = "android"))]
            {
                let path = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file();
                match path {
                    Some(p) => {
                        match std::fs::read_to_string(&p) {
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
                        }
                    }
                    None => {}
                }
            }
            #[cfg(target_os = "android")]
            show_toast(&w, "Import not available on Android");
        });
    }

    // Reset
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

    // Switch profile
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

    // Delete profile
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

    // Add profile
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

    // Backup local toggle
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

    // Backup WebDAV toggle
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

    // Set data dir
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

    // Backup à la fermeture
    {
        let state_ref = state.clone();
        window.window().on_close_requested(move || {
            let st = state_ref.lock().unwrap();
            st.storage.backup_on_exit();
            slint::CloseRequestResponse::HideWindow
        });
    }

    window.run().unwrap();
}
