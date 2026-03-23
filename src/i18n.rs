// oxycash-rs - i18n.rs
use std::collections::HashMap;

pub fn get_translations(lang: &str) -> HashMap<&'static str, &'static str> {
    if lang == "fr" { fr() } else { en() }
}

fn en() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Months
    m.insert("jan","January"); m.insert("feb","February"); m.insert("mar","March");
    m.insert("apr","April"); m.insert("mai","May"); m.insert("jun","June");
    m.insert("jul","July"); m.insert("aug","August"); m.insert("sep","September");
    m.insert("oct","October"); m.insert("nov","November"); m.insert("dec","December");
    // Tabs
    m.insert("tab_debts","Debts"); m.insert("tab_savings","Savings"); m.insert("tab_expenses","Expenses");
    m.insert("tab_viability","Viability"); m.insert("tab_charts","Charts"); m.insert("tab_config","Config");
    // Cards
    m.insert("card_income","INCOME"); m.insert("card_withdrawals","WITHDRAWALS");
    m.insert("card_paid","PAID"); m.insert("card_to_pay","TO PAY");
    m.insert("card_forecast","FORECAST"); m.insert("card_balance","BALANCE");
    m.insert("col_bank","Bank"); m.insert("col_cash","Cash"); m.insert("col_total","Total");
    m.insert("col_to_withdraw","To withdraw"); m.insert("col_withdrawn","Withdrawn");
    // Sections
    m.insert("sec_income","Income"); m.insert("sec_withdrawals","Withdrawals");
    m.insert("sec_fixed","Fixed"); m.insert("sec_variable","Variable");
    m.insert("col_bank_hdr","BANK"); m.insert("col_cash_hdr","CASH");
    m.insert("col_paid","PAID"); m.insert("col_left","LEFT");
    // Chart
    m.insert("chart_budget_vs","BUDGET VS PAID"); m.insert("chart_withdrawals","Withdrawals");
    m.insert("chart_fixed","Fixed"); m.insert("chart_variable","Variable");
    // Register
    m.insert("reg_title","Register"); m.insert("reg_date_asc","↑ Date asc"); m.insert("reg_date_desc","↓ Date desc");
    m.insert("reg_date","Date"); m.insert("reg_label","Label"); m.insert("reg_section","Section");
    m.insert("reg_amount","Amount"); m.insert("reg_no_payments","No payments recorded");
    // Payments
    m.insert("pay_date","Date"); m.insert("pay_amount","Amount"); m.insert("pay_no","No payments");
    // Actions
    m.insert("add_entry","Add entry"); m.insert("new_entry","New entry");
    // Recurring
    m.insert("rec_title","Recurrence"); m.insert("rec_frequency","Repeat frequency");
    m.insert("rec_every_1","Every month"); m.insert("rec_every_2","Every 2 months");
    m.insert("rec_every_3","Every 3 months"); m.insert("rec_every_6","Every 6 months");
    m.insert("rec_every_12","Annual"); m.insert("rec_past","Also apply to past months");
    m.insert("rec_cancel","Cancel"); m.insert("rec_disable","Disable"); m.insert("rec_apply","Apply");
    // Debts
    m.insert("deb_title","Debts"); m.insert("deb_total_due","Total due");
    m.insert("deb_negotiated","Negotiated"); m.insert("deb_settled","Settled");
    m.insert("deb_add","+ Add debt"); m.insert("deb_rep","Rep"); m.insert("deb_pursuit","N°");
    m.insert("deb_due","Due"); m.insert("deb_neg","Neg"); m.insert("deb_status","Status"); m.insert("deb_date","Date");
    // Savings
    m.insert("sav_title","Savings"); m.insert("sav_add_project","+ New project"); m.insert("sav_add_entry","+ Add entry");
    // Expenses
    m.insert("exp_title","Expenses"); m.insert("exp_name","Name"); m.insert("exp_total","Total");
    // Viability
    m.insert("via_title","Viability"); m.insert("via_subtitle","Monthly simulation by salary bracket");
    m.insert("via_columns","Columns"); m.insert("via_add_col","+ Column");
    m.insert("via_generate","▶ Generate"); m.insert("via_clear","Clear all");
    m.insert("via_add_bracket","+ Add bracket"); m.insert("via_no_brackets","No brackets yet — click ▶ Generate");
    m.insert("via_balance","Balance");
    // Charts
    m.insert("charts_title","Charts"); m.insert("charts_income","Income"); m.insert("charts_paid","Paid");
    m.insert("charts_to_pay","To pay"); m.insert("charts_withdrawals","Withdrawals");
    m.insert("charts_forecast","Forecast"); m.insert("charts_balance","Balance");
    m.insert("charts_cumul","Annual cumulative"); m.insert("charts_exp","Exp.");
    // Config
    m.insert("cfg_title","Configuration"); m.insert("cfg_profiles","Profiles");
    m.insert("cfg_add_profile","+ Add"); m.insert("cfg_use","Use"); m.insert("cfg_currency","Currency");
    m.insert("cfg_theme","Theme"); m.insert("cfg_dark_to_light","Dark — switch to light");
    m.insert("cfg_light_to_dark","Light — switch to dark");
    m.insert("cfg_webdav","WebDAV (Nextcloud, kDrive…)");
    m.insert("cfg_url","WebDAV URL"); m.insert("cfg_user","Username"); m.insert("cfg_password","Password");
    m.insert("cfg_save","Save"); m.insert("cfg_test","Test"); m.insert("cfg_clear","Clear");
    m.insert("cfg_export","Export"); m.insert("cfg_export_btn","Export JSON");
    m.insert("cfg_import","Import"); m.insert("cfg_import_btn","Import JSON");
    m.insert("cfg_data","Data"); m.insert("cfg_reset","Reset all data");
    m
}

fn fr() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("jan","Janvier"); m.insert("feb","Février"); m.insert("mar","Mars");
    m.insert("apr","Avril"); m.insert("mai","Mai"); m.insert("jun","Juin");
    m.insert("jul","Juillet"); m.insert("aug","Août"); m.insert("sep","Septembre");
    m.insert("oct","Octobre"); m.insert("nov","Novembre"); m.insert("dec","Décembre");
    m.insert("tab_debts","Dettes"); m.insert("tab_savings","Épargne"); m.insert("tab_expenses","Frais");
    m.insert("tab_viability","Viabilité"); m.insert("tab_charts","Graphiques"); m.insert("tab_config","Config");
    m.insert("card_income","REVENUS"); m.insert("card_withdrawals","RETRAITS");
    m.insert("card_paid","PAYÉ"); m.insert("card_to_pay","À PAYER");
    m.insert("card_forecast","PRÉVISION"); m.insert("card_balance","SOLDE");
    m.insert("col_bank","Banque"); m.insert("col_cash","Cash"); m.insert("col_total","Total");
    m.insert("col_to_withdraw","À retirer"); m.insert("col_withdrawn","Retiré");
    m.insert("sec_income","Revenus"); m.insert("sec_withdrawals","Retraits");
    m.insert("sec_fixed","Frais fixes"); m.insert("sec_variable","Ponctuels");
    m.insert("col_bank_hdr","BANQUE"); m.insert("col_cash_hdr","CASH");
    m.insert("col_paid","PAYÉ"); m.insert("col_left","SOLDE");
    m.insert("chart_budget_vs","BUDGET VS PAYÉ"); m.insert("chart_withdrawals","Retraits");
    m.insert("chart_fixed","Frais fixes"); m.insert("chart_variable","Ponctuels");
    m.insert("reg_title","Registre"); m.insert("reg_date_asc","↑ Date croissante"); m.insert("reg_date_desc","↓ Date décroissante");
    m.insert("reg_date","Date"); m.insert("reg_label","Libellé"); m.insert("reg_section","Section");
    m.insert("reg_amount","Montant"); m.insert("reg_no_payments","Aucun paiement enregistré");
    m.insert("pay_date","Date"); m.insert("pay_amount","Montant"); m.insert("pay_no","Aucun paiement");
    m.insert("add_entry","Ajouter"); m.insert("new_entry","Nouveau");
    m.insert("rec_title","Récurrence"); m.insert("rec_frequency","Fréquence de répétition");
    m.insert("rec_every_1","Chaque mois"); m.insert("rec_every_2","Tous les 2 mois");
    m.insert("rec_every_3","Tous les 3 mois"); m.insert("rec_every_6","Tous les 6 mois");
    m.insert("rec_every_12","Annuel"); m.insert("rec_past","Appliquer aussi aux mois passés");
    m.insert("rec_cancel","Annuler"); m.insert("rec_disable","Désactiver"); m.insert("rec_apply","Appliquer");
    m.insert("deb_title","Dettes"); m.insert("deb_total_due","Total dû");
    m.insert("deb_negotiated","Négocié"); m.insert("deb_settled","Soldées");
    m.insert("deb_add","+ Ajouter une dette"); m.insert("deb_rep","Rep"); m.insert("deb_pursuit","N°");
    m.insert("deb_due","Dû"); m.insert("deb_neg","Nég"); m.insert("deb_status","État"); m.insert("deb_date","Date");
    m.insert("sav_title","Épargne"); m.insert("sav_add_project","+ Nouveau projet"); m.insert("sav_add_entry","+ Entrée");
    m.insert("exp_title","Frais annuels"); m.insert("exp_name","Poste"); m.insert("exp_total","Total");
    m.insert("via_title","Viabilité"); m.insert("via_subtitle","Simulation mensuelle par palier de salaire");
    m.insert("via_columns","Colonnes"); m.insert("via_add_col","+ Colonne");
    m.insert("via_generate","▶ Générer"); m.insert("via_clear","Tout supprimer");
    m.insert("via_add_bracket","+ Ajouter un palier"); m.insert("via_no_brackets","Aucun palier — cliquez ▶ Générer");
    m.insert("via_balance","Solde");
    m.insert("charts_title","Graphiques"); m.insert("charts_income","Revenus"); m.insert("charts_paid","Payé");
    m.insert("charts_to_pay","À payer"); m.insert("charts_withdrawals","Retraits");
    m.insert("charts_forecast","Prévision"); m.insert("charts_balance","Solde");
    m.insert("charts_cumul","Cumul annuel"); m.insert("charts_exp","Dép.");
    m.insert("cfg_title","Configuration"); m.insert("cfg_profiles","Profils");
    m.insert("cfg_add_profile","+ Ajouter"); m.insert("cfg_use","Utiliser"); m.insert("cfg_currency","Devise");
    m.insert("cfg_theme","Thème"); m.insert("cfg_dark_to_light","Sombre — passer en clair");
    m.insert("cfg_light_to_dark","Clair — passer en sombre");
    m.insert("cfg_webdav","WebDAV (Nextcloud, kDrive…)");
    m.insert("cfg_url","URL WebDAV"); m.insert("cfg_user","Utilisateur"); m.insert("cfg_password","Mot de passe");
    m.insert("cfg_save","Sauver"); m.insert("cfg_test","Tester"); m.insert("cfg_clear","Effacer");
    m.insert("cfg_export","Export"); m.insert("cfg_export_btn","Exporter JSON");
    m.insert("cfg_import","Import"); m.insert("cfg_import_btn","Importer JSON");
    m.insert("cfg_data","Données"); m.insert("cfg_reset","Réinitialiser");
    m
}
