// oxycash-rs - compute.rs
// Pure data computations: monthly summary and frais sync.
use std::collections::HashMap;

use crate::model::{FraisLine, Line, Month, MONTHS};
use crate::state::AppState;

// ── Monthly summary ───────────────────────────────────────────────────────────

pub struct MonthSummary {
    pub income: f64, pub income_b: f64, pub income_c: f64,
    pub paid:   f64, pub paid_b:   f64, pub paid_c:   f64,
    pub topay:  f64, pub topay_b:  f64, pub topay_c:  f64,
    pub forecast: f64, pub forecast_b: f64, pub forecast_c: f64,
    pub balance:  f64, pub balance_b:  f64, pub balance_c:  f64,
}

/// Exact port of month_view.py summary().
pub fn compute_summary(m: &Month) -> MonthSummary {
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
        paid:   paye_total,            paid_b:   paye_banque, paid_c: paye_cash,
        topay:  a_payer_b + a_payer_c, topay_b:  a_payer_b,  topay_c: a_payer_c,
        forecast: prev_total,   forecast_b: prev_banque, forecast_c: prev_cash,
        balance:  solde_total,  balance_b:  solde_banque, balance_c: solde_cash,
    }
}

// ── Frais sync ────────────────────────────────────────────────────────────────

/// Rebuild the frais (expenses) table entirely from monthly budget lines.
/// Called after any line mutation so the expenses tab stays in sync.
pub fn sync_frais_from_months(state: &mut AppState) {
    let mut fixes:     HashMap<String, [f64; 12]> = HashMap::new();
    let mut ponctuels: HashMap<String, [f64; 12]> = HashMap::new();
    let mut retraits:  HashMap<String, [f64; 12]> = HashMap::new();

    for (mi, &mk) in MONTHS.iter().enumerate() {
        let m = match state.storage.data.months.get(mk) {
            Some(m) => m,
            None => continue,
        };
        for line in &m.fixes {
            fixes.entry(line.name.clone()).or_insert([0.0; 12])[mi] += line.banque + line.cash;
        }
        for line in &m.variables {
            ponctuels.entry(line.name.clone()).or_insert([0.0; 12])[mi] += line.banque + line.cash;
        }
        for line in &m.retraits {
            retraits.entry(line.name.clone()).or_insert([0.0; 12])[mi] += line.banque;
        }
    }

    let to_vec = |map: HashMap<String, [f64; 12]>| -> Vec<FraisLine> {
        let mut v: Vec<FraisLine> = map.into_iter().map(|(name, monthly)| FraisLine { name, monthly }).collect();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    };

    state.storage.data.frais.fixes     = to_vec(fixes);
    state.storage.data.frais.ponctuels = to_vec(ponctuels);
    state.storage.data.frais.retraits  = to_vec(retraits);
}
