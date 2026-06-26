// oxycash-rs - callbacks_data.rs
// Callbacks: debt management, savings projects/rows, viability columns/paliers.
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

use crate::AppWindow;
use crate::model::{Dette, SavingsProject, SavingsRow, ViabiliteColonne, ViabilitePalier};
use crate::push::{push_debts, push_savings, push_viability};
use crate::state::AppState;

pub fn register(window: &AppWindow, state: &Arc<Mutex<AppState>>) {

    // ── Debts ─────────────────────────────────────────────────────────────────

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
                    "creditor"   => d.creancier = v,
                    "rep"        => d.rep       = v,
                    "pursuit"    => d.poursuite = v,
                    "balance"    => { if let Ok(n) = v.parse::<f64>() { d.solde    = n; } }
                    "balance-ok" => { if let Ok(n) = v.parse::<f64>() { d.solde_ok = n; } }
                    "status"     => d.etat = v,
                    "date"       => d.date = v,
                    _ => {}
                }
                st.storage.save();
                push_debts(&w, &st);
            }
        });
    }

    // ── Savings ───────────────────────────────────────────────────────────────

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
