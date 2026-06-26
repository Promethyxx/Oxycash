// oxycash-rs - push.rs
// push_* functions: transfer AppState data into Slint window properties.
use slint::{Global, ModelRc, SharedString, VecModel};

use crate::{AppWindow, DebtItem, ExpenseLine, ExpenseSection, I18n, ProfileItem,
            RegisterEntry, SavingsEntry, SavingsRowItem, ViaColumn, ViaPalier, ViaPalierCell};
use crate::compute::compute_summary;
use crate::model::{fmt, fmt_sign, FraisLine, Line, MONTHS};
use crate::state::AppState;
use crate::ui_helpers::{color_for, hex_color, make_bool_model, make_line_items, make_payment_items};
use crate::i18n;

// ── Month view ────────────────────────────────────────────────────────────────

pub fn push_month(window: &AppWindow, state: &AppState) {
    let Some(m) = state.month() else { return };

    // Summary cards
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

    // Per-section totals
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

    // Translated month name
    let month_keys = ["jan","feb","mar","apr","mai","jun","jul","aug","sep","oct","nov","dec"];
    let mi = MONTHS.iter().position(|&x| x == state.current_month.as_str()).unwrap_or(0);
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

    push_register(window, state);
}

// ── Register ──────────────────────────────────────────────────────────────────

pub fn push_register(window: &AppWindow, state: &AppState) {
    let Some(m) = state.month() else { return };
    let sec_names = ["Income", "Withdrawal", "Fixed", "Variable"];
    let sections: [&Vec<Line>; 4] = [&m.revenus, &m.retraits, &m.fixes, &m.variables];

    let mut entries: Vec<(String, String, String, f64)> = vec![];
    for (si, lines) in sections.iter().enumerate() {
        for line in lines.iter() {
            for p in &line.payments {
                entries.push((p.date.clone(), line.name.clone(), sec_names[si].to_string(), p.amount));
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

// ── Annual charts ─────────────────────────────────────────────────────────────

pub fn push_charts(window: &AppWindow, state: &AppState) {
    let data     = &state.storage.data;
    let currency = &state.storage.cfg.currency;

    let mut tot_rev     = 0.0_f64;
    let mut tot_paye    = 0.0_f64;
    let mut tot_a_payer = 0.0_f64;
    let mut tot_ret     = 0.0_f64;
    let mut tot_prev    = 0.0_f64;
    let mut tot_solde   = 0.0_f64;

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

        let rev_recu_b: f64  = m.revenus.iter().filter(|l| l.banque > 0.01).map(|l| l.etat()).sum();
        let rev_recu_c: f64  = m.revenus.iter().filter(|l| l.cash > 0.01).map(|l| l.etat()).sum();
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

    // Summary cards
    window.set_charts_total_income(format!("{} {}", fmt_sign(tot_rev), currency).into());
    window.set_charts_total_paid(format!("{} {}", fmt(tot_paye), currency).into());
    window.set_charts_total_to_pay(format!("{} {}", fmt(tot_a_payer), currency).into());
    window.set_charts_total_withdrawals(format!("{} {}", fmt(tot_ret), currency).into());
    window.set_charts_total_forecast(format!("{} {}", fmt_sign(tot_prev), currency).into());
    window.set_charts_total_balance(format!("{} {}", fmt_sign(tot_solde), currency).into());

    window.set_charts_income_color(color_for(tot_rev));
    window.set_charts_paid_color(hex_color("#85CDCA"));
    window.set_charts_to_pay_color(color_for(-tot_a_payer));
    window.set_charts_withdrawals_color(hex_color("#F2D388"));
    window.set_charts_forecast_color(color_for(tot_prev));
    window.set_charts_balance_color(color_for(tot_solde));

    // Monthly rows
    let mut r_months:    Vec<SharedString>  = Vec::with_capacity(12);
    let mut r_incomes:   Vec<SharedString>  = Vec::with_capacity(12);
    let mut r_expenses:  Vec<SharedString>  = Vec::with_capacity(12);
    let mut r_forecasts: Vec<SharedString>  = Vec::with_capacity(12);
    let mut r_fcols:     Vec<slint::Color>  = Vec::with_capacity(12);

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

    window.set_charts_annual_cumul(fmt_sign(cumul).into());
    window.set_charts_annual_cumul_color(color_for(cumul));
}

// ── Debts ─────────────────────────────────────────────────────────────────────

pub fn push_debts(window: &AppWindow, state: &AppState) {
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

// ── Savings ───────────────────────────────────────────────────────────────────

pub fn push_savings(window: &AppWindow, state: &AppState) {
    let projects = &state.storage.data.epargne.savings;
    let items: Vec<SavingsEntry> = projects.iter().enumerate().map(|(i, p)| {
        let has_rows   = !p.rows.is_empty();
        let rows_total: f64 = p.rows.iter().map(|r| r.montant).sum();
        let rows_cible: f64 = p.rows.iter().map(|r| r.cible).sum();
        let total = if has_rows { rows_total } else { p.montant };
        let cible = if has_rows && rows_cible > 0.01 { rows_cible } else { p.cible };
        let pct   = if cible > 0.01 { ((total / cible) * 100.0).min(100.0) as i32 } else { 0 };

        let row_items: Vec<SavingsRowItem> = p.rows.iter().enumerate().map(|(ri, r)| {
            let rpct = if r.cible > 0.01 { ((r.montant / r.cible) * 100.0).min(100.0) as i32 } else { 0 };
            SavingsRowItem { name: r.name.clone().into(), montant: r.montant as f32, cible: r.cible as f32, percent: rpct, idx: ri as i32 }
        }).collect();

        SavingsEntry {
            label: p.label.clone().into(), montant: total as f32, cible: cible as f32,
            percent: pct, idx: i as i32, open: p.open,
            rows: ModelRc::new(VecModel::from(row_items)),
        }
    }).collect();
    window.set_savings_list(ModelRc::new(VecModel::from(items)));
}

// ── Expenses ──────────────────────────────────────────────────────────────────

fn make_expense_section(label: &str, lines: &[FraisLine]) -> ExpenseSection {
    let mut sec_total = 0.0_f64;
    let el: Vec<ExpenseLine> = lines.iter().map(|fl| {
        let t: f64 = fl.monthly.iter().sum();
        sec_total += t;
        ExpenseLine {
            name: fl.name.clone().into(),
            m0:  fl.monthly[0]  as f32, m1:  fl.monthly[1]  as f32,
            m2:  fl.monthly[2]  as f32, m3:  fl.monthly[3]  as f32,
            m4:  fl.monthly[4]  as f32, m5:  fl.monthly[5]  as f32,
            m6:  fl.monthly[6]  as f32, m7:  fl.monthly[7]  as f32,
            m8:  fl.monthly[8]  as f32, m9:  fl.monthly[9]  as f32,
            m10: fl.monthly[10] as f32, m11: fl.monthly[11] as f32,
            total: t as f32,
        }
    }).collect();
    ExpenseSection { label: label.into(), lines: ModelRc::new(VecModel::from(el)), total: sec_total as f32 }
}

pub fn push_expenses(window: &AppWindow, state: &AppState) {
    let frais = &state.storage.data.frais;
    let tr = i18n::get_translations(&state.storage.cfg.lang);
    let l = |k: &str| -> String { tr.get(k).copied().unwrap_or(k).to_string() };
    window.set_expenses_fixed(make_expense_section(&l("sec_fixed"), &frais.fixes));
    window.set_expenses_occasional(make_expense_section(&l("sec_variable"), &frais.ponctuels));
    window.set_expenses_withdrawals(make_expense_section(&l("sec_withdrawals"), &frais.retraits));
}

// ── Viability ─────────────────────────────────────────────────────────────────

pub fn push_viability(window: &AppWindow, state: &AppState) {
    let vc = &state.storage.data.viabilite;

    let cols: Vec<ViaColumn> = vc.colonnes.iter().enumerate().map(|(i, c)| {
        ViaColumn { name: c.name.clone().into(), is_income: c.is_income, delta_type: c.delta_type.clone().into(), idx: i as i32 }
    }).collect();
    window.set_via_columns(ModelRc::new(VecModel::from(cols)));

    let pals: Vec<ViaPalier> = vc.paliers.iter().enumerate().map(|(pi, p)| {
        let cells: Vec<ViaPalierCell> = p.valeurs.iter().map(|&v| ViaPalierCell { value: v as f32 }).collect();
        let balance: f64 = vc.colonnes.iter().enumerate().map(|(ci, c)| {
            let v = p.valeurs.get(ci).copied().unwrap_or(0.0);
            if c.is_income { v } else { -v }
        }).sum();
        ViaPalier { cells: ModelRc::new(VecModel::from(cells)), balance: balance as f32, idx: pi as i32 }
    }).collect();
    window.set_via_paliers(ModelRc::new(VecModel::from(pals)));
    window.set_via_n_paliers(vc.n_paliers as i32);
}

// ── Settings ──────────────────────────────────────────────────────────────────

pub fn push_settings(window: &AppWindow, state: &AppState) {
    let cfg         = &state.storage.cfg;
    let active_slug = &cfg.active;

    let items: Vec<ProfileItem> = cfg.profiles.iter().enumerate().map(|(i, p)| {
        ProfileItem { name: p.name.clone().into(), slug: p.slug.clone().into(), active: p.slug == *active_slug, idx: i as i32 }
    }).collect();
    window.set_settings_profiles(ModelRc::new(VecModel::from(items)));

    // DAV credentials are global
    window.set_settings_dav_url(cfg.dav_url.clone().into());
    window.set_settings_dav_user(cfg.dav_user.clone().into());
    window.set_settings_dav_pass(cfg.dav_pass.clone().into());
    window.set_settings_dav2_url(cfg.dav2_url.clone().into());
    window.set_settings_dav2_user(cfg.dav2_user.clone().into());
    window.set_settings_dav2_pass(cfg.dav2_pass.clone().into());
    window.set_settings_dav2_enabled(cfg.dav2_enabled);

    let prof = state.storage.active_profile();
    window.set_settings_currency_edit(cfg.currency.clone().into());
    window.set_profile_name(prof.name.clone().into());
    window.set_currency(cfg.currency.clone().into());
    window.set_settings_backup_local(cfg.backup_local);
    window.set_settings_backup_webdav(cfg.backup_webdav);
    window.set_settings_data_dir_display(state.storage.data_dir_display().into());
    window.set_settings_backup_dir_display(state.storage.backup_dir_display().into());
    window.set_settings_data_dir_edit(cfg.data_dir.clone().into());
}

// ── i18n ──────────────────────────────────────────────────────────────────────

pub fn push_i18n(window: &AppWindow, lang: &str) {
    let tr = i18n::get_translations(lang);
    let i  = I18n::get(window);
    macro_rules! set {
        ($prop:ident, $key:expr) => {
            if let Some(&v) = tr.get($key) { i.$prop(slint::SharedString::from(v)); }
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
