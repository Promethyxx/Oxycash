// oxycash-rs - callbacks_budget.rs
// Callbacks: tab navigation, section/line toggles, budget line mutations,
// payment mutations, recurring setup, register sort.
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

use crate::{AppWindow, Tab};
use crate::compute::sync_frais_from_months;
use crate::model::{Line, Payment, Recurring, MONTHS};
use crate::push::{push_month, push_register, push_charts, push_debts, push_savings, push_expenses, push_viability, push_settings};
use crate::state::AppState;
use crate::ui_helpers::tab_to_month_key;
use crate::model::today;

pub fn register(window: &AppWindow, state: &Arc<Mutex<AppState>>) {

    // Tab change
    {
        let state_ref = state.clone();
        let ww = window.as_weak();
        window.on_tab_changed(move |tab| {
            let w = ww.unwrap();
            w.set_current_tab(tab);
            if let Some(key) = tab_to_month_key(tab) {
                let mut st = state_ref.lock().unwrap();
                st.current_month   = key.to_string();
                st.sections_open   = [false; 4];
                st.lines_expanded  = [vec![], vec![], vec![], vec![]];
                st.ensure_expanded();
                push_month(&w, &st);
            }
            {
                let st = state_ref.lock().unwrap();
                match tab {
                    Tab::Charts    => push_charts(&w, &st),
                    Tab::Debts     => push_debts(&w, &st),
                    Tab::Savings   => push_savings(&w, &st),
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
            let mk     = st.current_month.clone();
            let src_mi = MONTHS.iter().position(|&m| m == mk.as_str()).unwrap_or(0);
            let sec_key = match si as usize { 0 => "revenus", 1 => "retraits", 2 => "fixes", _ => "variables" };

            // Remove recurring copies from other months if line has recurring
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
                    if target_mi == src_mi   { continue; }
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
            if st.sec_lines(si as usize).iter().any(|l| l.name == new_name) { return; }
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
            let mk     = st.current_month.clone();
            let src_mi = MONTHS.iter().position(|&m| m == mk.as_str()).unwrap_or(0);

            if freq <= 0 {
                // Disable recurring
                let line     = &st.sec_lines(si as usize)[li as usize];
                let name     = line.name.clone();
                let old_freq = line.recurring.as_ref().map(|r| r.freq as usize).unwrap_or(3);
                let start    = line.recurring.as_ref().map(|r| r.start.clone()).unwrap_or(mk.clone());
                let sec_key  = match si as usize { 0 => "revenus", 1 => "retraits", 2 => "fixes", _ => "variables" };
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
                    if target_mi == src_mi   { continue; }
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
                // Enable recurring
                let line    = &st.sec_lines(si as usize)[li as usize];
                let banque  = line.banque;
                let cash    = line.cash;
                let name    = line.name.clone();
                let sec_key = match si as usize { 0 => "revenus", 1 => "retraits", 2 => "fixes", _ => "variables" };

                let line = &mut st.sec_lines_mut(si as usize)[li as usize];
                line.recurring = Some(Recurring { freq: freq as u32, start: mk.clone() });

                let f = freq as usize;
                for step in 1..13 {
                    let target_mi = (src_mi + step * f) % 12;
                    if target_mi == src_mi { break; }
                    if !include_past && target_mi < src_mi { continue; }
                    let target_key = MONTHS[target_mi].to_string();
                    if let Some(target_month) = st.storage.data.months.get_mut(&target_key) {
                        let sec = target_month.section_mut(sec_key);
                        if let Some(existing) = sec.iter_mut().find(|l| l.name == name) {
                            existing.banque    = banque;
                            existing.cash      = cash;
                            existing.recurring = Some(Recurring { freq: freq as u32, start: mk.clone() });
                        } else {
                            sec.push(Line {
                                name: name.clone(), banque, cash, payments: vec![],
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
}
