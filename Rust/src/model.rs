// oxycash-rs - model.rs
// Direct mapping of core/model.py
use chrono::{Datelike, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Constants
pub const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAI", "JUN",
    "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

pub const MNAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

// --- Primitives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub date: String,   // YYYY-MM-DD
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Recurring {
    pub freq: u32,      // en mois
    pub start: String,  // month key e.g. "JAN"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub name: String,
    #[serde(default)]
    pub banque: f64,
    #[serde(default)]
    pub cash: f64,
    #[serde(default)]
    pub payments: Vec<Payment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurring: Option<Recurring>,
}

impl Line {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            banque: 0.0,
            cash: 0.0,
            payments: vec![],
            recurring: None,
        }
    }

    /// Sum of recorded payments
    pub fn etat(&self) -> f64 {
        self.payments.iter().map(|p| p.amount).sum()
    }

    /// Remaining budget
    pub fn solde(&self) -> f64 {
        (self.banque + self.cash) - self.etat()
    }
}

// --- Month
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Month {
    pub name: String,
    #[serde(default)]
    pub revenus: Vec<Line>,
    #[serde(default)]
    pub retraits: Vec<Line>,
    #[serde(default)]
    pub fixes: Vec<Line>,
    #[serde(default)]
    pub variables: Vec<Line>,
}

impl Month {
    pub fn section(&self, key: &str) -> &Vec<Line> {
        match key {
            "revenus"   => &self.revenus,
            "retraits"  => &self.retraits,
            "fixes"     => &self.fixes,
            "variables" => &self.variables,
            _           => panic!("Unknown section: {}", key),
        }
    }

    pub fn section_mut(&mut self, key: &str) -> &mut Vec<Line> {
        match key {
            "revenus"   => &mut self.revenus,
            "retraits"  => &mut self.retraits,
            "fixes"     => &mut self.fixes,
            "variables" => &mut self.variables,
            _           => panic!("Unknown section: {}", key),
        }
    }
}

// --- Debts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dette {
    #[serde(default)]
    pub rep: String,
    #[serde(default)]
    pub creancier: String,
    #[serde(default)]
    pub poursuite: String,
    #[serde(default)]
    pub solde: f64,
    #[serde(default, rename = "soldeOk")]
    pub solde_ok: f64,
    #[serde(default)]
    pub etat: String,
    #[serde(default)]
    pub date: String,
}

// --- Savings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpargneSondage {
    #[serde(default = "default_nouveau")]
    pub name: String,
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub goal: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpargneAchat {
    #[serde(default = "default_nouveau")]
    pub name: String,
    #[serde(default)]
    pub prix: f64,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wishlist {
    pub label: String,
    #[serde(default)]
    pub items: Vec<EpargneAchat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Epargne {
    #[serde(default)]
    pub sondages: Vec<EpargneSondage>,
    #[serde(default)]
    pub wishlists: Vec<Wishlist>,
    #[serde(default)]
    pub savings: Vec<serde_json::Value>, // flexible, not yet typed
}

fn default_nouveau() -> String {
    "Nouveau".to_string()
}

// --- Expenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraisLine {
    #[serde(default = "default_nouveau")]
    pub name: String,
    /// 12 valeurs, une par mois
    pub monthly: [f64; 12],
}

impl Default for FraisLine {
    fn default() -> Self {
        Self { name: "Nouveau".into(), monthly: [0.0; 12] }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Frais {
    #[serde(default)]
    pub fixes: Vec<FraisLine>,
    #[serde(default)]
    pub ponctuels: Vec<FraisLine>,
    #[serde(default)]
    pub retraits: Vec<FraisLine>,
}

// --- Viability
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViabiliteColonne {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub valeur: f64,
    #[serde(default)]
    pub is_income: bool,
    #[serde(default = "default_fixed")]
    pub delta_type: String, // "fixed" | "pct"
    #[serde(default)]
    pub delta_val: f64,
}

fn default_fixed() -> String { "fixed".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ViabilitePalier {
    pub valeurs: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViabiliteConfig {
    #[serde(default)]
    pub colonnes: Vec<ViabiliteColonne>,
    #[serde(default)]
    pub paliers: Vec<ViabilitePalier>,
    #[serde(default = "default_n_paliers")]
    pub n_paliers: u32,
}

fn default_n_paliers() -> u32 { 5 }

impl Default for ViabiliteConfig {
    fn default() -> Self {
        Self {
            colonnes: default_viabilite_colonnes(),
            paliers: vec![],
            n_paliers: 5,
        }
    }
}

fn default_viabilite_colonnes() -> Vec<ViabiliteColonne> {
    vec![
        ViabiliteColonne { name: "Salaire".into(),   valeur: 0.0, is_income: true,  delta_type: "fixed".into(), delta_val: 0.0 },
        ViabiliteColonne { name: "Loyer".into(),      valeur: 0.0, is_income: false, delta_type: "fixed".into(), delta_val: 0.0 },
        ViabiliteColonne { name: "Assurance".into(),  valeur: 0.0, is_income: false, delta_type: "fixed".into(), delta_val: 0.0 },
        ViabiliteColonne { name: "Frais min.".into(), valeur: 0.0, is_income: false, delta_type: "fixed".into(), delta_val: 0.0 },
        ViabiliteColonne { name: "Tax/mo".into(),   valeur: 0.0, is_income: false, delta_type: "pct".into(),   delta_val: 0.0 },
        ViabiliteColonne { name: "Subside".into(),    valeur: 0.0, is_income: true,  delta_type: "fixed".into(), delta_val: 0.0 },
    ]
}

// --- AppData (root)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    #[serde(default)]
    pub months: HashMap<String, Month>,
    #[serde(default)]
    pub dettes: Vec<Dette>,
    #[serde(default)]
    pub epargne: Epargne,
    #[serde(default)]
    pub frais: Frais,
    #[serde(default)]
    pub viabilite: ViabiliteConfig,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            months: HashMap::new(),
            dettes: vec![],
            epargne: Epargne::default(),
            frais: Frais::default(),
            viabilite: ViabiliteConfig::default(),
        }
    }
}

impl AppData {
    pub fn new_empty() -> Self {
        let mut data = Self::default();
        for (i, &key) in MONTHS.iter().enumerate() {
            data.months.insert(key.to_string(), Month {
                name: MNAMES[i].to_string(),
                ..Default::default()
            });
        }
        data
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

// --- Recurring propagation
pub fn apply_recurring(data: &mut AppData) {
    // Collect propagations first to avoid borrow conflicts
    struct Prop {
        target_key: String,
        section: String,
        line: Line,
    }
    let mut props: Vec<Prop> = vec![];

    for (src_key, src_month) in &data.months {
        let src_mi = match MONTHS.iter().position(|&m| m == src_key.as_str()) {
            Some(i) => i,
            None => continue,
        };
        for sec in &["revenus", "retraits", "fixes", "variables"] {
            for line in src_month.section(sec) {
                let rec = match &line.recurring {
                    Some(r) => r,
                    None => continue,
                };
                let freq = rec.freq as usize;
                let start_mi = MONTHS.iter().position(|&m| m == rec.start.as_str())
                    .unwrap_or(src_mi);

                for step in 1..13 {
                    let target_mi = (start_mi + step * freq) % 12;
                    if target_mi == src_mi { break; }
                    let target_key = MONTHS[target_mi].to_string();
                    props.push(Prop {
                        target_key,
                        section: sec.to_string(),
                        line: Line {
                            name: line.name.clone(),
                            banque: line.banque,
                            cash: line.cash,
                            payments: vec![],
                            recurring: Some(Recurring {
                                freq: rec.freq,
                                start: rec.start.clone(),
                            }),
                        },
                    });
                }
            }
        }
    }

    for p in props {
        if let Some(target_month) = data.months.get_mut(&p.target_key) {
            let sec = target_month.section_mut(&p.section);
            if !sec.iter().any(|l| l.name == p.line.name) {
                sec.push(p.line);
                sec.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
        }
    }
}

// --- Current month detection
pub fn detect_budget_month() -> &'static str {
    let now = Local::now();
    let mi = if now.day() < 25 {
        (now.month0() as i32 - 1).rem_euclid(12) as usize
    } else {
        now.month0() as usize
    };
    MONTHS[mi]
}

// --- Formatting
pub fn fmt(n: f64) -> String {
    let s = format!("{:.2}", n);
    if s.ends_with("00") { return s[..s.len()-3].to_string(); }
    if s.ends_with('0')  { return s[..s.len()-1].to_string(); }
    s
}

pub fn fmt_sign(n: f64) -> String {
    if n >= 0.0 { format!("+{}", fmt(n)) } else { fmt(n) }
}

pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}
