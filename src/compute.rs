// oxycash-rs - compute.rs
// Pure data computations: monthly summary and frais sync.
use std::collections::HashMap;

use crate::model::{new_debt_id, AppData, Dette, FraisLine, Line, Month, MNAMES, MONTHS};
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

// ── Debts ─────────────────────────────────────────────────────────────────────

pub struct DebtStats {
    pub planned: f64,
    pub paid:    f64,
}

pub fn debt_stats(data: &AppData, id: &str) -> DebtStats {
    let mut s = DebtStats { planned: 0.0, paid: 0.0 };
    if id.is_empty() { return s; }
    for m in data.months.values() {
        for lines in [&m.revenus, &m.retraits, &m.fixes, &m.variables] {
            for l in lines.iter() {
                if l.debt_id.as_deref() != Some(id) { continue; }
                s.planned += l.banque + l.cash;
                s.paid    += l.etat();
            }
        }
    }
    s
}

pub fn round2(v: f64) -> f64 { (v * 100.0).round() / 100.0 }

pub fn debt_month_amounts(data: &AppData, id: &str) -> [f64; 12] {
    let mut out = [0.0; 12];
    if id.is_empty() { return out; }
    for (mi, &mk) in MONTHS.iter().enumerate() {
        if let Some(m) = data.months.get(mk) {
            for lines in [&m.revenus, &m.retraits, &m.fixes, &m.variables] {
                for l in lines.iter() {
                    if l.debt_id.as_deref() == Some(id) { out[mi] += l.banque + l.cash; }
                }
            }
        }
    }
    out
}

pub fn set_debt_months(data: &mut AppData, di: usize, amounts: &[f64; 12]) -> bool {
    if di >= data.dettes.len() { return false; }
    if data.dettes[di].id.is_empty() { data.dettes[di].id = new_debt_id(di); }
    let id   = data.dettes[di].id.clone();
    let name = if data.dettes[di].creancier.trim().is_empty() { "Debt".to_string() } else { data.dettes[di].creancier.clone() };
    let current = debt_month_amounts(data, &id);
    for mi in 0..12 {
        let target = round2(amounts[mi]).max(0.0);
        if (target - round2(current[mi])).abs() < 0.005 { continue; }
        let month = data.months.entry(MONTHS[mi].to_string())
            .or_insert_with(|| Month { name: MNAMES[mi].to_string(), ..Default::default() });
        for lines in [&mut month.revenus, &mut month.retraits, &mut month.fixes, &mut month.variables] {
            lines.retain(|l| l.debt_id.as_deref() != Some(id.as_str()) || !l.payments.is_empty());
        }
        let mut kept = 0.0;
        for lines in [&month.revenus, &month.retraits, &month.fixes, &month.variables] {
            for l in lines.iter() {
                if l.debt_id.as_deref() == Some(id.as_str()) { kept += l.banque + l.cash; }
            }
        }
        let need = round2(target - kept);
        if need > 0.005 {
            month.variables.push(Line {
                name: name.clone(), banque: need, cash: 0.0,
                payments: vec![], recurring: None, debt_id: Some(id.clone()),
            });
            month.variables.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
    }
    true
}

pub fn recurrence_amounts(data: &AppData, di: usize, amount: f64, freq: usize, start: usize) -> Option<[f64; 12]> {
    if di >= data.dettes.len() || freq == 0 || start >= 12 || !(amount >= 0.0) { return None; }
    let d = &data.dettes[di];
    let mut out = debt_month_amounts(data, &d.id);
    let target = if d.solde_ok > 0.0 { d.solde_ok } else { d.solde };
    let before: f64 = out[..start].iter().sum();
    let mut remaining = if target > 0.0 { round2(target - before) } else { f64::MAX };
    for v in out[start..].iter_mut() { *v = 0.0; }
    if amount < 0.005 { return Some(out); }
    let mut mi = start;
    while mi < 12 && remaining > 0.005 {
        let v = round2(amount.min(remaining));
        out[mi] = v;
        remaining = round2(remaining - v);
        mi += freq;
    }
    Some(out)
}

pub fn debts_for_export(data: &AppData) -> Vec<Dette> {
    data.dettes.iter().filter_map(|d| {
        let paid   = debt_stats(data, &d.id).paid;
        let target = if d.solde_ok > 0.0 { d.solde_ok } else { d.solde };
        let left   = round2(target - paid);
        if left <= 0.005 { return None; }
        Some(Dette {
            id: String::new(),
            rep: d.rep.clone(),
            creancier: d.creancier.clone(),
            poursuite: d.poursuite.clone(),
            solde: left,
            solde_ok: 0.0,
            etat: d.etat.clone(),
            date: d.date.clone(),
        })
    }).collect()
}

pub fn rename_debt(data: &mut AppData, id: &str, name: &str) {
    if id.is_empty() || name.trim().is_empty() { return; }
    for d in data.dettes.iter_mut() {
        if d.id == id { d.creancier = name.to_string(); }
    }
    for m in data.months.values_mut() {
        for lines in [&mut m.revenus, &mut m.retraits, &mut m.fixes, &mut m.variables] {
            let mut touched = false;
            for l in lines.iter_mut() {
                if l.debt_id.as_deref() == Some(id) && l.name != name {
                    l.name = name.to_string();
                    touched = true;
                }
            }
            if touched { lines.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())); }
        }
    }
}
