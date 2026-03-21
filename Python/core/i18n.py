"""
Oxycash – i18n (English default, French option)
Usage: from core.i18n import T, set_lang
"""
from __future__ import annotations

LANGS = ['en', 'fr']

_EN = {
    # ── Months ──
    'months_long': ['January','February','March','April','May','June',
                    'July','August','September','October','November','December'],
    'months_short': ['JAN','FEB','MAR','APR','MAY','JUN',
                     'JUL','AUG','SEP','OCT','NOV','DEC'],

    # ── Tabs ──
    'tab_debts':      'Debts',
    'tab_savings':    'Savings',
    'tab_expenses':   'Expenses',
    'tab_viability':  'Viability',
    'tab_config':     '⚙️',

    # ── Section names ──
    'sec_income':     'Income',
    'sec_withdrawals':'Withdrawals',
    'sec_fixed':      'Fixed expenses',
    'sec_variable':   'Occasional expenses',

    # ── Column headers ──
    'col_bank':       'Bank',
    'col_cash':       'Cash',
    'col_state':      'State',
    'col_balance':    'Balance',
    'col_total':      'Total',
    'col_name':       'Name',

    # ── Summary cards ──
    'card_income':      '💰 INCOME',
    'card_withdrawals': '🏧 WITHDRAWALS',
    'card_paid':        '✅ PAID',
    'card_to_pay':      '⏳ TO PAY',
    'card_forecast':    '📊 FORECAST',
    'card_balance':     '🏦 BALANCE',
    'card_received':    'Received',
    'card_to_withdraw': 'To withdraw',
    'card_withdrawn':   'Withdrawn',
    'card_budget_vs':   'BUDGET VS PAID',
    'card_budget':      '▓ Budget',
    'card_paid_lbl':    '█ Paid',

    # ── Sections labels in chart ──
    'chart_withdrawals': 'Withdrawals',
    'chart_fixed':       'Fixed exp.',
    'chart_variable':    'Occasional',

    # ── Actions ──
    'add_expense':    'Add an expense',
    'add_income':     'Add income',
    'add_withdrawal': 'Add withdrawal',
    'add_fixed':      'Add fixed expense',

    # ── Registre ──
    'reg_title':      '📋 Register',
    'dash_title':     '📊 Dashboard',
    'reg_date_asc':   '↑ Date ascending',
    'reg_date_desc':  '↓ Date descending',
    'reg_new_tx':     'New transaction',
    'reg_label':      'Label',
    'reg_add':        '+ Add',
    'reg_section':    'Section',
    'reg_variable':   'Occasional',
    'reg_fixed':      'Fixed exp.',
    'reg_withdrawal': 'Withdrawal',
    'reg_income':     'Income',

    # ── Payments ──
    'no_payments':    'No payments recorded',
    'pay_deleted':    '✕ Deleted',
    'pay_added':      '✓ Added',
    'pay_invalid':    '⚠️ Invalid amount',
    'pay_date_hint':  'YYYY-MM-DD',
    'pay_amt_hint':   'Amount',

    # ── Recurring ──
    'rec_title':      '🔁 Recurrence',
    'rec_frequency':  'Repeat frequency',
    'rec_every_1':    'Every month',
    'rec_every_2':    'Every 2 months',
    'rec_every_3':    'Every 3 months',
    'rec_every_6':    'Every 6 months',
    'rec_every_12':   'Annually',
    'rec_apply':      'Apply',
    'rec_disable':    'Disable',
    'rec_cancel':     'Cancel',
    'rec_active':     '🔁 Recurring every {n} months',
    'rec_disabled':   '🔁 Recurrence disabled',
    'rec_na':         '⚠️ Dialog unavailable',
    'rec_include_past': 'Also apply to past months',

    # ── Debts ──
    'deb_title':        'Debts',
    'deb_subtitle':     'Tracking of judgments and debts',
    'deb_total_due':    'TOTAL DUE',
    'deb_negotiated':   'NEGOTIATED',
    'deb_settled':      'SETTLED',
    'deb_representative':'Representative',
    'deb_pursuit':      'Judgment N°',
    'deb_due_chf':      'Due (CHF)',
    'deb_neg_chf':      'Negotiated (CHF)',
    'deb_status':       'Status',
    'deb_date':         'Date',
    'deb_settled_lbl':  '✓ Settled',
    'deb_add':          'Add a debt',
    'deb_deleted':      'Deleted',

    # ── Savings ──
    'sav_title':        'Savings',
    'sav_surveys':      'Active surveys',
    'sav_wishlist':     'Wishlist',
    'sav_legacy':       'Legacy PC (resale)',
    'sav_net':          'Net balance',
    'sav_total_chf':    'Total CHF',
    'sav_goal_chf':     'Goal CHF',
    'sav_price_chf':    'Price CHF',
    'sav_url_hint':     'URL to monitor price',
    'sav_add_survey':   'Add a survey',
    'sav_add_item':     'Add an item',
    'sav_add_part':     'Add a part',
    'sav_add_project':  '+ New project',
    'sav_deleted':      'Deleted',

    # ── Expenses ──
    'exp_title':        'Annual expenses',
    'exp_fixed':        '📌 Fixed expenses',
    'exp_occasional':   '🗓️ Occasional expenses',
    'exp_withdrawals':  '🏧 Withdrawals',
    'exp_add_line':     'Add a line',
    'exp_post':         'Item',
    'exp_total':        'Total',

    # ── Viability ──
    'via_title':        'Viability',
    'via_subtitle':     'Monthly simulation by salary bracket',
    'via_salary':       'Salary',
    'via_rent':         'Rent',
    'via_insurance':    'Insur.',
    'via_expenses':     'Exp.',
    'via_tax':          'Tax',
    'via_subsidy':      'Subsidy',
    'via_balance':      'Balance',
    'via_add':          'Add a bracket',
    'via_columns':      'Columns',
    'via_income_legend':'Current / Increase',
    'via_add_col':      '+ Column',
    'via_generate':     'Generate',
    'via_paliers':      'brackets',
    'via_gen_btn':      '▶ Go',
    'via_need_first':   'Add a base bracket first',
    'via_empty':        'No brackets yet — click ▶ Go to generate.',
    'via_clear_all':    'Clear all',
    'via_income_lbl':   'Current',
    'via_charge_lbl':   'Increase',
    'via_pct_base':     'Base for %',
    'exp_add_line':     'Add a line',

    # ── Config ──
    'cfg_title':        'Configuration',
    'cfg_subtitle':     'WebDAV sync settings',
    'cfg_webdav':       'WebDAV (Nextcloud, kDrive…)',
    'cfg_url':          'WebDAV URL',
    'cfg_user':         'Username',
    'cfg_password':     'Password',
    'cfg_save':         'Save',
    'cfg_test':         'Test',
    'cfg_clear':        'Clear',
    'cfg_saved':        'Config saved',
    'cfg_cleared':      'Config cleared',
    'cfg_export':       'Export',
    'cfg_export_lbl':   'Export JSON',
    'cfg_export_sub':   'Saved to user folder',
    'cfg_exported':     'Exported: {name}',
    'cfg_import':       'Import',
    'cfg_import_sub':   'Browse or paste the full path to the JSON file',
    'cfg_browse':       'Browse',
    'cfg_import_btn':   'Import',
    'cfg_import_ok':    'Imported from {name}',
    'cfg_import_invalid': 'Invalid JSON format',
    'cfg_import_notfound': 'File not found',
    'cfg_import_nopath': 'Enter a path or use Browse',
    'cfg_import_na':    'Browser unavailable — enter path manually',
    'cfg_data':         'Data',
    'cfg_reset':        'Reset',
    'cfg_reset_done':   'Reset',
    'cfg_lang':         'Language',


    # ── Charts ──
    'tab_charts':         'Charts',
    'chart_income_vs_exp':'INCOME VS EXPENSES',
    'chart_cumul_balance':'CUMULATIVE BALANCE',
    'chart_total_income': 'TOTAL INCOME',
    'chart_total_exp':    'TOTAL EXPENSES',
    'chart_net':          'NET',
    'chart_final':        'Year-end balance',
    'chart_exp':          'Expenses',
    'chart_annual_cumul': 'Annual cumulative',

    # ── Config currency ──
    'cfg_currency':       'Currency',
    'cfg_profiles':     'Profiles',
    'cfg_switch':       'Use',
    'cfg_add_profile':  '+ Add',
    'cfg_profile_hint': 'New profile name',
    'cfg_theme':        'Theme',
    'cfg_dark':         'Dark',
    'cfg_light':        'Light',
    'new_entry':        'New',


    # ── Toasts ──
    'toast_saved':      '✅ Saved',
    'toast_deleted':    '🗑️ Deleted',
    'toast_added':      '✓ Added',
    'toast_error':      '⚠️ Error',
    'toast_import_err': 'Import error',
    'toast_invalid':    '⚠️ Invalid amount',
    'toast_label_req':  'Label required',
    'toast_reset':      '🔄 Reset',
}

_FR = {
    # ── Months ──
    'months_long': ['Janvier','Février','Mars','Avril','Mai','Juin',
                    'Juillet','Août','Septembre','Octobre','Novembre','Décembre'],
    'months_short': ['JAN','FEV','MAR','AVR','MAI','JUN',
                     'JUL','AOÛ','SEP','OCT','NOV','DEC'],

    # ── Tabs ──
    'tab_debts':      'Dettes',
    'tab_savings':    'Épargne',
    'tab_expenses':   'Frais',
    'tab_viability':  'Viabilité',
    'tab_config':     '⚙️',

    # ── Section names ──
    'sec_income':     'Revenus',
    'sec_withdrawals':'Retraits',
    'sec_fixed':      'Frais fixes',
    'sec_variable':   'Frais ponctuels',

    # ── Column headers ──
    'col_bank':       'Banque',
    'col_cash':       'Cash',
    'col_state':      'État',
    'col_balance':    'Solde',
    'col_total':      'Total',
    'col_name':       'Nom',

    # ── Summary cards ──
    'card_income':      '💰 REVENUS',
    'card_withdrawals': '🏧 RETRAITS',
    'card_paid':        '✅ PAYÉ',
    'card_to_pay':      '⏳ À PAYER',
    'card_forecast':    '📊 PRÉVISION',
    'card_balance':     '🏦 SOLDE',
    'card_received':    'Reçu',
    'card_to_withdraw': 'À retirer',
    'card_withdrawn':   'Retiré',
    'card_budget_vs':   'BUDGET VS PAYÉ',
    'card_budget':      '▓ Budget',
    'card_paid_lbl':    '█ Payé',

    # ── Sections labels in chart ──
    'chart_withdrawals': 'Retraits',
    'chart_fixed':       'Frais fixes',
    'chart_variable':    'Ponctuels',

    # ── Actions ──
    'add_expense':    'Ajouter une dépense',
    'add_income':     'Ajouter un revenu',
    'add_withdrawal': 'Ajouter un retrait',
    'add_fixed':      'Ajouter une charge fixe',

    # ── Registre ──
    'reg_title':      '📋 Registre',
    'dash_title':     '📊 Dashboard',
    'reg_date_asc':   '↑ Date croissante',
    'reg_date_desc':  '↓ Date décroissante',
    'reg_new_tx':     'Nouvelle transaction',
    'reg_label':      'Libellé',
    'reg_add':        '+ Ajouter',
    'reg_section':    'Section',
    'reg_variable':   'Occasional',
    'reg_fixed':      'Frais fixes',
    'reg_withdrawal': 'Retrait',
    'reg_income':     'Revenu',

    # ── Payments ──
    'no_payments':    'Aucun paiement enregistré',
    'pay_deleted':    '✕ Supprimé',
    'pay_added':      '✓ Ajouté',
    'pay_invalid':    '⚠️ Montant invalide',
    'pay_date_hint':  'YYYY-MM-DD',
    'pay_amt_hint':   'Montant',

    # ── Recurring ──
    'rec_title':      '🔁 Récurrence',
    'rec_frequency':  'Fréquence de répétition',
    'rec_every_1':    'Chaque mois',
    'rec_every_2':    'Tous les 2 mois',
    'rec_every_3':    'Tous les 3 mois',
    'rec_every_6':    'Tous les 6 mois',
    'rec_every_12':   'Annuel',
    'rec_apply':      'Appliquer',
    'rec_disable':    'Désactiver',
    'rec_cancel':     'Annuler',
    'rec_active':     '🔁 Récurrent tous les {n} mois',
    'rec_disabled':   '🔁 Récurrence désactivée',
    'rec_na':         '⚠️ Dialog non disponible',
    'rec_include_past': 'Appliquer aussi aux mois passés',

    # ── Debts ──
    'deb_title':        'Dettes',
    'deb_subtitle':     'Suivi des poursuites et créances',
    'deb_total_due':    'TOTAL DÛ',
    'deb_negotiated':   'NÉGOCIÉ',
    'deb_settled':      'SOLDÉES',
    'deb_representative':'Représentant',
    'deb_pursuit':      'N° Poursuite',
    'deb_due_chf':      'Solde dû (CHF)',
    'deb_neg_chf':      'Négocié (CHF)',
    'deb_status':       'État',
    'deb_date':         'Date',
    'deb_settled_lbl':  '✓ Soldé',
    'deb_add':          'Ajouter une dette',
    'deb_deleted':      'Supprimé',

    # ── Savings ──
    'sav_title':        'Épargne',
    'sav_surveys':      'Sondages en cours',
    'sav_wishlist':     'Wishlist',
    'sav_legacy':       'PC Legacy (revente)',
    'sav_net':          'Solde net',
    'sav_total_chf':    'Total CHF',
    'sav_goal_chf':     'Objectif CHF',
    'sav_price_chf':    'Prix CHF',
    'sav_url_hint':     'URL pour surveiller le prix',
    'sav_add_survey':   'Ajouter un sondage',
    'sav_add_item':     'Ajouter un article',
    'sav_add_part':     'Ajouter une pièce',
    'sav_add_project':  '+ Nouveau projet',
    'sav_deleted':      'Supprimé',

    # ── Expenses ──
    'exp_title':        'Frais annuels',
    'exp_fixed':        '📌 Frais fixes',
    'exp_occasional':   '🗓️ Frais ponctuels',
    'exp_withdrawals':  '🏧 Retraits',
    'exp_add_line':     'Ajouter une ligne',
    'exp_post':         'Poste',
    'exp_total':        'Total',

    # ── Viability ──
    'via_title':        'Viabilité',
    'via_subtitle':     'Simulation mensuelle par palier de salaire',
    'via_salary':       'Salaire',
    'via_rent':         'Loyer',
    'via_insurance':    'Assur.',
    'via_expenses':     'Frais',
    'via_tax':          'Impôt',
    'via_subsidy':      'Subside',
    'via_balance':      'Solde',
    'via_add':          'Ajouter un palier de base',
    'via_columns':      'Colonnes',
    'via_income_legend':'Actuel / Augmentation',
    'via_add_col':      '+ Colonne',
    'via_generate':     'Générer',
    'via_paliers':      'paliers',
    'via_gen_btn':      '▶ Go',
    'via_need_first':   'Ajoutez un palier de base dabord',
    'via_empty':        'Aucun palier — cliquez ▶ Go pour générer.',
    'via_clear_all':    'Tout supprimer',
    'via_income_lbl':   'Actuel',
    'via_charge_lbl':   'Augmentation',
    'via_pct_base':     'Base %',
    'exp_add_line':     'Ajouter une ligne',

    # ── Config ──
    'cfg_title':        'Configuration',
    'cfg_subtitle':     'Connexion WebDAV pour la synchronisation',
    'cfg_webdav':       'WebDAV (Nextcloud, kDrive…)',
    'cfg_url':          'URL WebDAV',
    'cfg_user':         'Utilisateur',
    'cfg_password':     'Mot de passe',
    'cfg_save':         'Sauver',
    'cfg_test':         'Tester',
    'cfg_clear':        'Effacer',
    'cfg_saved':        'Config sauvée',
    'cfg_cleared':      'Config effacée',
    'cfg_export':       'Export',
    'cfg_export_lbl':   'Exporter JSON',
    'cfg_export_sub':   'Sauvegardé dans le dossier utilisateur',
    'cfg_exported':     'Exporté: {name}',
    'cfg_import':       'Import',
    'cfg_import_sub':   'Parcourir ou coller le chemin complet du fichier JSON',
    'cfg_browse':       'Parcourir',
    'cfg_import_btn':   'Importer',
    'cfg_import_ok':    'Importé depuis {name}',
    'cfg_import_invalid': 'Format JSON invalide',
    'cfg_import_notfound': 'Fichier non trouvé',
    'cfg_import_nopath': 'Entre un chemin ou utilise Parcourir',
    'cfg_import_na':    'Explorateur non disponible — entrer le chemin manuellement',
    'cfg_data':         'Données',
    'cfg_reset':        'Réinitialiser',
    'cfg_reset_done':   'Réinitialisé',
    'cfg_lang':         'Langue',


    # ── Charts ──
    'tab_charts':         'Graphiques',
    'chart_income_vs_exp':'REVENUS VS DÉPENSES',
    'chart_cumul_balance':'SOLDE CUMULATIF',
    'chart_total_income': 'TOTAL REVENUS',
    'chart_total_exp':    'TOTAL DÉPENSES',
    'chart_net':          'NET',
    'chart_final':        "Solde fin d'année",
    'chart_exp':          'Dépenses',
    'chart_annual_cumul': 'Cumul annuel',

    # ── Config currency ──
    'cfg_currency':       'Devise',
    'cfg_profiles':     'Profils',
    'cfg_switch':       'Utiliser',
    'cfg_add_profile':  '+ Ajouter',
    'cfg_profile_hint': 'Nom du nouveau profil',
    'cfg_theme':        'Thème',
    'cfg_dark':         'Sombre',
    'cfg_light':        'Clair',
    'new_entry':        'Nouveau',


    # ── Toasts ──
    'toast_saved':      '✅ Sauvé',
    'toast_deleted':    '🗑️ Supprimé',
    'toast_added':      '✓ Ajouté',
    'toast_error':      '⚠️ Erreur',
    'toast_import_err': 'Erreur import',
    'toast_invalid':    '⚠️ Montant invalide',
    'toast_label_req':  'Libellé requis',
    'toast_reset':      '🔄 Réinitialisé',
}

_TRANSLATIONS = {'en': _EN, 'fr': _FR}
_current_lang  = ['en']


class _T:
    """Proxy — T['key'] returns translation for current lang."""
    def __getitem__(self, key: str) -> str:
        d = _TRANSLATIONS[_current_lang[0]]
        return d.get(key, _EN.get(key, key))

    def get(self, key: str, default: str = '') -> str:
        d = _TRANSLATIONS[_current_lang[0]]
        val = d.get(key, _EN.get(key, None))
        return val if val is not None else default

    def fmt(self, key: str, **kwargs) -> str:
        return self[key].format(**kwargs)

    @property
    def lang(self) -> str:
        return _current_lang[0]

    @property
    def months_long(self) -> list:
        return self['months_long']

    @property
    def months_short(self) -> list:
        return self['months_short']


T = _T()


def set_lang(lang: str):
    if lang in LANGS:
        _current_lang[0] = lang


def get_lang() -> str:
    return _current_lang[0]


def toggle_lang():
    _current_lang[0] = 'fr' if _current_lang[0] == 'en' else 'en'