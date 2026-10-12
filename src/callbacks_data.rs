// oxycash-rs - callbacks_data.rs
// Callbacks: debt management, savings projects/rows, viability columns/paliers.
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

use crate::AppWindow;
use crate::i18n::get_translations;
use crate::compute::{debts_for_export, recurrence_amounts, rename_debt, round2, set_debt_months, sync_frais_from_months};
use crate::model::{debts_from_json, debts_to_json, new_debt_id, savings_from_json, savings_to_json, Dette, SavingsProject, SavingsRow, ViabiliteColonne, ViabilitePalier};
use crate::push::{push_debts, push_month, push_savings, push_viability};
use crate::state::AppState;
use crate::ui_helpers::show_toast;

fn tr(lang: &str, key: &str) -> String {
    get_translations(lang).get(key).copied().unwrap_or(key).to_string()
}

fn lang_of(state: &Arc<Mutex<AppState>>) -> String {
    state.lock().unwrap().storage.cfg.lang.clone()
}

fn set_debt_field(st: &mut AppState, i: usize, f: &str, v: String) -> bool {
    if i >= st.storage.data.dettes.len() { return false; }
    let mut rename: Option<(String, String)> = None;
    {
        let d = &mut st.storage.data.dettes[i];
        match f {
            "creditor"   => { d.creancier = v.clone(); rename = Some((d.id.clone(), v)); }
            "rep"        => d.rep       = v,
            "pursuit"    => d.poursuite = v,
            "balance"    => { if let Ok(n) = v.trim().replace(',', ".").parse::<f64>() { d.solde    = n; } }
            "balance-ok" => { if let Ok(n) = v.trim().replace(',', ".").parse::<f64>() { d.solde_ok = n; } }
            "status"     => d.etat = v,
            "date"       => d.date = v,
            _ => {}
        }
    }
    if let Some((id, name)) = rename {
        rename_debt(&mut st.storage.data, &id, &name);
        st.lines_expanded[3] = vec![];
        st.ensure_expanded();
        sync_frais_from_months(st);
    }
    true
}

#[cfg(not(target_os = "android"))]
fn save_export(w: &AppWindow, lang: &str, fname: &str, json: &str) {
    let path = rfd::FileDialog::new()
        .set_file_name(fname)
        .add_filter("JSON", &["json"])
        .save_file();
    if let Some(p) = path {
        match std::fs::write(&p, json) {
            Ok(_)  => show_toast(w, &tr(lang, "t_exported")),
            Err(e) => show_toast(w, &format!("Error: {}", e)),
        }
    }
}

#[cfg(target_os = "android")]
fn save_export(w: &AppWindow, lang: &str, fname: &str, json: &str) {
    let path = dirs::data_local_dir().unwrap_or_default().join(fname);
    match std::fs::write(&path, json) {
        Ok(_)  => show_toast(w, &tr(lang, "t_exported")),
        Err(e) => show_toast(w, &format!("Error: {}", e)),
    }
}

#[cfg(not(target_os = "android"))]
fn pick_json(w: &AppWindow, _lang: &str) -> Option<String> {
    let p = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file()?;
    match std::fs::read_to_string(&p) {
        Ok(raw) => Some(raw),
        Err(e)  => { show_toast(w, &format!("Error: {}", e)); None }
    }
}

#[cfg(target_os = "android")]
fn pick_json(w: &AppWindow, lang: &str) -> Option<String> {
    show_toast(w, &tr(lang, "t_no_import"));
    None
}

pub fn register(window: &AppWindow, state: &Arc<Mutex<AppState>>) {

    // ── Debts ─────────────────────────────────────────────────────────────────

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_debt(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let id = new_debt_id(st.storage.data.dettes.len());
            st.storage.data.dettes.push(Dette { id, ..Dette::default() });
            st.storage.save();
            push_debts(&w, &st);
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_delete_debt(move |idx| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i < st.storage.data.dettes.len() {
                let id = st.storage.data.dettes[i].id.clone();
                if !id.is_empty() {
                    for m in st.storage.data.months.values_mut() {
                        for lines in [&mut m.revenus, &mut m.retraits, &mut m.fixes, &mut m.variables] {
                            for l in lines.iter_mut() {
                                if l.debt_id.as_deref() == Some(id.as_str()) { l.debt_id = None; }
                            }
                        }
                    }
                }
                st.storage.data.dettes.remove(i);
                st.storage.save();
                push_debts(&w, &st);
                push_month(&w, &st);
            }
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_update_debt_field(move |idx, field, val| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            if set_debt_field(&mut st, idx as usize, field.as_str(), val.to_string()) {
                st.storage.save();
                push_debts(&w, &st);
                push_month(&w, &st);
            }
        });
    }

    {
        let state_ref = state.clone();
        window.on_commit_debt_field(move |idx, field, val| {
            let mut st = state_ref.lock().unwrap();
            if set_debt_field(&mut st, idx as usize, field.as_str(), val.to_string()) {
                st.storage.save();
            }
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_debt_to_month(move |idx, creditor, amounts| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            if i >= st.storage.data.dettes.len() { return; }
            let mut values = [0.0f64; 12];
            for (m, part) in amounts.as_str().split(';').enumerate().take(12) {
                if let Ok(v) = part.trim().replace(',', ".").parse::<f64>() {
                    if v.is_finite() { values[m] = round2(v.max(0.0)); }
                }
            }
            let name = creditor.trim().to_string();
            if !name.is_empty() { st.storage.data.dettes[i].creancier = name.clone(); }
            set_debt_months(&mut st.storage.data, i, &values);
            let id = st.storage.data.dettes[i].id.clone();
            if !name.is_empty() { rename_debt(&mut st.storage.data, &id, &name); }
            st.lines_expanded[3] = vec![];
            st.ensure_expanded();
            sync_frais_from_months(&mut st);
            st.storage.save();
            push_month(&w, &st);
            push_debts(&w, &st);
            let msg = tr(&st.storage.cfg.lang, "t_updated");
            show_toast(&w, &msg);
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_apply_debt_recurrence(move |idx, creditor, amount, freq, start| {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let i = idx as usize;
            let amt = match amount.as_str().trim().replace(',', ".").parse::<f64>() {
                Ok(v) if v.is_finite() => v,
                _ => {
                    let msg = tr(&st.storage.cfg.lang, "t_enter_amount");
                    show_toast(&w, &msg);
                    return;
                }
            };
            let name = creditor.trim().to_string();
            if i < st.storage.data.dettes.len() && !name.is_empty() {
                st.storage.data.dettes[i].creancier = name.clone();
            }
            if i < st.storage.data.dettes.len() && st.storage.data.dettes[i].id.is_empty() {
                st.storage.data.dettes[i].id = new_debt_id(i);
            }
            let Some(values) = recurrence_amounts(&st.storage.data, i, round2(amt), freq.max(1) as usize, start.max(0) as usize) else {
                let msg = tr(&st.storage.cfg.lang, "t_enter_amount");
                show_toast(&w, &msg);
                return;
            };
            set_debt_months(&mut st.storage.data, i, &values);
            let id = st.storage.data.dettes[i].id.clone();
            if !name.is_empty() { rename_debt(&mut st.storage.data, &id, &name); }
            st.lines_expanded[3] = vec![];
            st.ensure_expanded();
            sync_frais_from_months(&mut st);
            st.storage.save();
            push_month(&w, &st);
            push_debts(&w, &st);
            let msg = tr(&st.storage.cfg.lang, "t_updated");
            show_toast(&w, &msg);
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_export_debts(move || {
            let w = ww.unwrap();
            let st = state_ref.lock().unwrap();
            let list  = debts_for_export(&st.storage.data);
            let fname = format!("oxycash_debts_{}.json", st.storage.cfg.active);
            let lang  = st.storage.cfg.lang.clone();
            drop(st);
            if list.is_empty() { show_toast(&w, &tr(&lang, "t_nothing")); return; }
            save_export(&w, &lang, &fname, &debts_to_json(&list));
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_import_debts(move || {
            let w = ww.unwrap();
            let lang = lang_of(&state_ref);
            let Some(raw) = pick_json(&w, &lang) else { return; };
            match debts_from_json(&raw) {
                Some(list) if !list.is_empty() => {
                    let mut st = state_ref.lock().unwrap();
                    let n = list.len();
                    for mut d in list {
                        d.id = new_debt_id(st.storage.data.dettes.len());
                        st.storage.data.dettes.push(d);
                    }
                    st.storage.save();
                    push_debts(&w, &st);
                    show_toast(&w, &format!("{} {} ✓", tr(&st.storage.cfg.lang, "t_imported"), n));
                }
                _ => show_toast(&w, &tr(&lang, "t_invalid")),
            }
        });
    }

    // ── Savings ───────────────────────────────────────────────────────────────

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_export_savings(move || {
            let w = ww.unwrap();
            let st = state_ref.lock().unwrap();
            let list  = st.storage.data.epargne.savings.clone();
            let fname = format!("oxycash_savings_{}.json", st.storage.cfg.active);
            let lang  = st.storage.cfg.lang.clone();
            drop(st);
            if list.is_empty() { show_toast(&w, &tr(&lang, "t_nothing")); return; }
            save_export(&w, &lang, &fname, &savings_to_json(&list));
        });
    }

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_import_savings(move || {
            let w = ww.unwrap();
            let lang = lang_of(&state_ref);
            let Some(raw) = pick_json(&w, &lang) else { return; };
            match savings_from_json(&raw) {
                Some(list) if !list.is_empty() => {
                    let mut st = state_ref.lock().unwrap();
                    let n = list.len();
                    st.storage.data.epargne.savings.extend(list);
                    st.storage.save();
                    push_savings(&w, &st);
                    show_toast(&w, &format!("{} {} ✓", tr(&st.storage.cfg.lang, "t_imported"), n));
                }
                _ => show_toast(&w, &tr(&lang, "t_invalid")),
            }
        });
    }

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
                    "cible"   => { if let Ok(n) = v.parse::<f64>() { p.cible   = n; } }
                    _ => {}
                }
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

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
                    "name" => {
                        row.name = v;
                        st.storage.data.epargne.savings[p].rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    }
                    "montant" => { if let Ok(n) = v.parse::<f64>() { row.montant = n; } }
                    "cible"   => { if let Ok(n) = v.parse::<f64>() { row.cible   = n; } }
                    _ => {}
                }
                st.storage.save();
                push_savings(&w, &st);
            }
        });
    }

    // ── Viability ─────────────────────────────────────────────────────────────

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_via_palier(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let nc = st.storage.data.viabilite.colonnes.len();
            st.storage.data.viabilite.paliers.push(ViabilitePalier { valeurs: vec![0.0; nc] });
            st.storage.save();
            push_viability(&w, &st);
        });
    }

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

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_generate_via(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            let vc   = &st.storage.data.viabilite;
            let n    = vc.n_paliers as usize;
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
                paliers.push(ViabilitePalier { valeurs: vals });
            }
            st.storage.data.viabilite.paliers = paliers;
            st.storage.save();
            push_viability(&w, &st);
        });
    }

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

    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_add_via_column(move || {
            let w = ww.unwrap();
            let mut st = state_ref.lock().unwrap();
            st.storage.data.viabilite.colonnes.push(ViabiliteColonne {
                name: "New".into(), valeur: 0.0, is_income: false,
                delta_type: "fixed".into(), delta_val: 0.0,
            });
            for p in &mut st.storage.data.viabilite.paliers { p.valeurs.push(0.0); }
            st.storage.save();
            push_viability(&w, &st);
        });
    }

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
}
