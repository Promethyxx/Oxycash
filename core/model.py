"""
Oxycash – Data model (mirrors the HTML data structure exactly)
"""
from __future__ import annotations
from dataclasses import dataclass, field
from typing import List, Optional
import json, copy, datetime

# Month keys — kept as original for backward compatibility with saved JSON files
MONTHS       = ['JAN','FEB','MAR','APR','MAI','JUN','JUL','AUG','SEP','OCT','NOV','DEC']
# Tab keys
SPECIAL_TABS = ['Debts','Savings','Expenses','Viability','Config']
ALL_TABS     = MONTHS + SPECIAL_TABS

# English month names for display (i18n overrides via T.months_long)
MNAMES = ['January','February','March','April','May','June',
          'July','August','September','October','November','December']


# ─── Primitives ───────────────────────────────────────────────────────────────

@dataclass
class Payment:
    date: str      # YYYY-MM-DD
    amount: float

    def to_dict(self):
        return {'date': self.date, 'amount': self.amount}

    @staticmethod
    def from_dict(d: dict) -> 'Payment':
        return Payment(date=d['date'], amount=float(d.get('amount', 0)))


@dataclass
class Line:
    name: str
    banque: float = 0.0
    cash: float = 0.0
    payments: List[Payment] = field(default_factory=list)
    # recurring: None or {'freq': int, 'start': 'JAN'} — freq in months
    recurring: Optional[dict] = None

    # ── computed ──
    def etat(self) -> float:
        return sum(p.amount for p in self.payments)

    def solde(self) -> float:
        return (self.banque + self.cash) - self.etat()

    def to_dict(self):
        d = {'name': self.name, 'banque': self.banque, 'cash': self.cash,
             'payments': [p.to_dict() for p in self.payments]}
        if self.recurring:
            d['recurring'] = self.recurring
        return d

    @staticmethod
    def from_dict(d: dict) -> 'Line':
        return Line(
            name=d.get('name', ''),
            banque=float(d.get('banque', 0)),
            cash=float(d.get('cash', 0)),
            payments=[Payment.from_dict(p) for p in d.get('payments', [])],
            recurring=d.get('recurring', None),
        )


def mk_line(name: str, banque: float = 0, cash: float = 0,
            payments: Optional[List[dict]] = None) -> Line:
    pays = [Payment.from_dict(p) for p in (payments or [])]
    return Line(name=name, banque=banque, cash=cash, payments=pays)


# ─── Month ────────────────────────────────────────────────────────────────────

@dataclass
class Month:
    name: str
    revenus:   List[Line] = field(default_factory=list)
    retraits:  List[Line] = field(default_factory=list)
    fixes:     List[Line] = field(default_factory=list)
    variables: List[Line] = field(default_factory=list)

    # section helpers
    def section(self, key: str) -> List[Line]:
        return getattr(self, key)

    def to_dict(self):
        return {
            'name': self.name,
            'revenus':   [l.to_dict() for l in self.revenus],
            'retraits':  [l.to_dict() for l in self.retraits],
            'fixes':     [l.to_dict() for l in self.fixes],
            'variables': [l.to_dict() for l in self.variables],
        }

    @staticmethod
    def from_dict(d: dict) -> 'Month':
        return Month(
            name=d.get('name', ''),
            revenus=[Line.from_dict(l) for l in d.get('revenus', [])],
            retraits=[Line.from_dict(l) for l in d.get('retraits', [])],
            fixes=[Line.from_dict(l) for l in d.get('fixes', [])],
            variables=[Line.from_dict(l) for l in d.get('variables', [])],
        )


# ─── Special sections ─────────────────────────────────────────────────────────

@dataclass
class Dette:
    rep: str = ''
    creancier: str = ''
    poursuite: str = ''
    solde: float = 0
    soldeOk: float = 0
    etat: str = ''
    date: str = ''

    def to_dict(self): return self.__dict__.copy()

    @staticmethod
    def from_dict(d): return Dette(**{k: d.get(k, v) for k, v in Dette().__dict__.items()})


@dataclass
class EpargneSondage:
    name: str = 'Nouveau'
    total: float = 0
    goal: float = 0
    def to_dict(self): return self.__dict__.copy()
    @staticmethod
    def from_dict(d): return EpargneSondage(**{k: d.get(k, 0) if k != 'name' else d.get(k, 'Nouveau') for k in ('name','total','goal')})


@dataclass
class EpargneAchat:
    name: str = 'Nouveau'
    prix: float = 0
    url: str = ''
    def to_dict(self): return {'name': self.name, 'prix': self.prix, 'url': self.url}
    @staticmethod
    def from_dict(d): return EpargneAchat(
        name=d.get('name','Nouveau'), prix=float(d.get('prix',0)), url=d.get('url',''))


@dataclass
class EpargnePcLegacy:
    name: str = 'Nouveau'
    val: float = 0
    def to_dict(self): return self.__dict__.copy()
    @staticmethod
    def from_dict(d): return EpargnePcLegacy(name=d.get('name','Nouveau'), val=float(d.get('val',0)))


@dataclass
class FraisLine:
    name: str = 'Nouveau'
    monthly: List[float] = field(default_factory=lambda: [0.0]*12)
    def to_dict(self): return {'name': self.name, 'monthly': self.monthly}
    @staticmethod
    def from_dict(d): return FraisLine(name=d.get('name',''), monthly=[float(x) for x in d.get('monthly',[0]*12)])


@dataclass
class ViabiliteColonne:
    """Définition d'une colonne : nom, valeur de base, delta_type, delta_val, couleur."""
    name: str = ''
    valeur: float = 0.0
    is_income: bool = False   # True = revenu, False = charge
    delta_type: str = 'fixed' # 'fixed' | 'pct'
    delta_val: float = 0.0

    def to_dict(self): return self.__dict__.copy()
    @staticmethod
    def from_dict(d):
        return ViabiliteColonne(
            name=d.get('name',''),
            valeur=float(d.get('valeur',0)),
            is_income=bool(d.get('is_income',False)),
            delta_type=d.get('delta_type','fixed'),
            delta_val=float(d.get('delta_val',0)),
        )

@dataclass
class ViabilitePalier:
    """Un palier = liste de valeurs dans l'ordre des colonnes de ViabiliteConfig."""
    valeurs: List[float] = field(default_factory=list)

    def to_dict(self): return {'valeurs': self.valeurs}
    @staticmethod
    def from_dict(d):
        # compat ancien format (champs nommés)
        if 'valeurs' in d:
            return ViabilitePalier(valeurs=[float(x) for x in d['valeurs']])
        # migration depuis ancien modèle
        return ViabilitePalier(valeurs=[
            float(d.get('salaire',0)), float(d.get('loyer',0)),
            float(d.get('assurance',0)), float(d.get('fraisMin',0)),
            float(d.get('impotM',0)), float(d.get('subside',0)),
        ])

@dataclass
class ViabiliteConfig:
    """Config globale : colonnes + paliers générés."""
    colonnes: List[ViabiliteColonne] = field(default_factory=list)
    paliers:  List[ViabilitePalier]  = field(default_factory=list)
    n_paliers: int = 5

    def to_dict(self):
        return {
            'colonnes':  [c.to_dict() for c in self.colonnes],
            'paliers':   [p.to_dict() for p in self.paliers],
            'n_paliers': self.n_paliers,
        }
    @staticmethod
    def from_dict(d):
        return ViabiliteConfig(
            colonnes=[ViabiliteColonne.from_dict(c) for c in d.get('colonnes',[])],
            paliers =[ViabilitePalier.from_dict(p)  for p in d.get('paliers', [])],
            n_paliers=int(d.get('n_paliers', 5)),
        )

def default_viabilite_config() -> ViabiliteConfig:
    cols = [
        ViabiliteColonne('Salaire',   3000, is_income=True,  delta_type='fixed', delta_val=500),
        ViabiliteColonne('Loyer',     1412, is_income=False, delta_type='fixed', delta_val=0),
        ViabiliteColonne('Assurance',  444, is_income=False, delta_type='fixed', delta_val=0),
        ViabiliteColonne('Frais',      500, is_income=False, delta_type='fixed', delta_val=0),
        ViabiliteColonne('Impôt',        0, is_income=False, delta_type='pct',   delta_val=12.5),
        ViabiliteColonne('Subside',      0, is_income=True,  delta_type='fixed', delta_val=0),
    ]
    return ViabiliteConfig(colonnes=cols, paliers=[], n_paliers=5)


# ─── Root data ────────────────────────────────────────────────────────────────

@dataclass
class AppData:
    months: dict[str, Month] = field(default_factory=dict)
    dettes: List[Dette] = field(default_factory=list)
    epargne: dict = field(default_factory=dict)
    frais: dict = field(default_factory=dict)
    viabilite: 'ViabiliteConfig' = field(default_factory=lambda: ViabiliteConfig())

    def to_dict(self):
        ep = {
            'sondages':  [x.to_dict() for x in self.epargne.get('sondages', [])],
            'wishlists': [{'label': wl['label'],
                           'items': [a.to_dict() for a in wl.get('items', [])]}
                          for wl in self.epargne.get('wishlists', [])],
            'pc_legacy': [x.to_dict() for x in self.epargne.get('pc_legacy', [])],
            'savings':   self.epargne.get('savings', []),
        }
        fr = {
            'fixes':     [x.to_dict() for x in self.frais.get('fixes', [])],
            'ponctuels': [x.to_dict() for x in self.frais.get('ponctuels', [])],
            'retraits':  [x.to_dict() for x in self.frais.get('retraits', [])],
        }
        return {
            'months': {k: v.to_dict() for k, v in self.months.items()},
            'dettes': [d.to_dict() for d in self.dettes],
            'epargne': ep,
            'frais': fr,
            'viabilite': self.viabilite.to_dict(),
        }

    @staticmethod
    def from_dict(d: dict) -> 'AppData':
        months = {k: Month.from_dict(v) for k, v in d.get('months', {}).items()}
        dettes = [Dette.from_dict(x) for x in d.get('dettes', [])]
        ep_raw = d.get('epargne', {})
        # migrate old 'achats' list to new 'wishlists' format
        if ep_raw.get('wishlists') is not None:
            wishlists = [{'label': wl.get('label', 'Projet'),
                          'items': [EpargneAchat.from_dict(a) for a in wl.get('items', [])]}
                         for wl in ep_raw['wishlists']]
        else:
            old_items = [EpargneAchat.from_dict(x) for x in ep_raw.get('achats', [])]
            wishlists = [{'label': 'Projet', 'items': old_items}] if old_items else []
        epargne = {
            'sondages':  [EpargneSondage.from_dict(x) for x in ep_raw.get('sondages', [])],
            'wishlists': wishlists,
            'pc_legacy': [EpargnePcLegacy.from_dict(x) for x in ep_raw.get('pc_legacy', [])],
            'savings':   ep_raw.get('savings', []),
        }
        fr_raw = d.get('frais', {})
        frais = {
            'fixes':     [FraisLine.from_dict(x) for x in fr_raw.get('fixes', [])],
            'ponctuels': [FraisLine.from_dict(x) for x in fr_raw.get('ponctuels', [])],
            'retraits':  [FraisLine.from_dict(x) for x in fr_raw.get('retraits', [])],
        }
        via_raw = d.get('viabilite', {})
        if isinstance(via_raw, list):
            # migration ancien format
            from .model import default_viabilite_config
            vc = default_viabilite_config()
            vc.paliers = [ViabilitePalier.from_dict(x) for x in via_raw]
        else:
            vc = ViabiliteConfig.from_dict(via_raw)
        viabilite = vc
        return AppData(months=months, dettes=dettes, epargne=epargne, frais=frais, viabilite=viabilite)

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), indent=2, ensure_ascii=False)

    @staticmethod
    def from_json(s: str) -> 'AppData':
        return AppData.from_dict(json.loads(s))


# ─── Default data factory (mirrors defaultData() in JS) ───────────────────────

def _def_month(mi: int) -> Month:
    return Month(
        name=MNAMES[mi],  # display name updated at runtime via i18n
        revenus=[],
        retraits=[],
        fixes=[],
        variables=[],
    )


def default_data() -> AppData:
    """Données vierges pour un nouvel utilisateur."""
    d = AppData()
    for i, m in enumerate(MONTHS):
        d.months[m] = _def_month(i)

    d.dettes    = []
    d.frais     = {'fixes': [], 'ponctuels': [], 'retraits': []}
    d.epargne   = {'sondages': [], 'wishlists': [], 'pc_legacy': [], 'savings': []}
    d.viabilite = default_viabilite_config()
    return d


def demo_data() -> AppData:
    """Données de démonstration — NE PAS embarquer dans une release publique."""
    d = AppData()
    for i, m in enumerate(MONTHS):
        d.months[m] = _def_month(i)

    d.dettes = []
    d.frais = {
        'fixes': [
            FraisLine('Loyer',   [900]*12),
            FraisLine('Courses', [460]*12),
            FraisLine('Loisirs', [380]*12),
        ],
        'ponctuels': [],
    }
    d.epargne   = {'sondages': [], 'wishlists': [{'label': 'Projet', 'items': []}], 'pc_legacy': []}
    d.viabilite = []

    # ── Viabilité ──
    viab = []
    for s in range(3000, 17000, 500):
        assurance = 520 if s <= 6000 else 495 if s <= 7000 else 444
        subside   = 331 if s <= 3500 else 0
        viab.append(ViabilitePalier(
            salaire=s, loyer=1412, assurance=assurance,
            fraisMin=500, impotM=round(s*0.125), subside=subside
        ))
    d.viabilite = viab

    return d


# ─── Recurring propagation ────────────────────────────────────────────────────

def apply_recurring(data) -> None:
    """
    Propagate recurring lines to target months.
    ONLY creates missing lines — NEVER overwrites existing amounts or payments.
    Each month owns its own data independently.
    """
    for src_key, src_month in data.months.items():
        src_mi = MONTHS.index(src_key)
        for sec_key in ('revenus', 'retraits', 'fixes', 'variables'):
            for line in list(src_month.section(sec_key)):
                rec = line.recurring
                if not rec:
                    continue
                freq     = int(rec.get('freq', 3))
                start    = rec.get('start', src_key)
                start_mi = MONTHS.index(start) if start in MONTHS else src_mi
                for step in range(1, 13):
                    target_mi = (start_mi + step * freq) % 12
                    if target_mi == src_mi:
                        break
                    target_key   = MONTHS[target_mi]
                    target_month = data.months.get(target_key)
                    if target_month is None:
                        continue
                    target_sec = target_month.section(sec_key)
                    if line.name not in [l.name for l in target_sec]:
                        target_sec.append(Line(
                            name=line.name,
                            banque=line.banque,
                            cash=line.cash,
                            recurring={'freq': freq, 'start': start},
                        ))
                        target_sec.sort(key=lambda l: l.name.lower())


def sync_frais_from_line(data, month_key: str, sec_key: str, line) -> None:
    """
    Update ONE cell in Frais for a recurring line in a specific month.
    Call after user edits banque/cash on a recurring line.
    Never touches other months.
    Mapping: fixes → frais.fixes, variables → frais.ponctuels, retraits → frais.retraits
    Revenus are NOT synced to frais.
    """
    if not line.recurring or data.frais is None:
        return
    if sec_key == 'revenus':
        return  # revenus never go into frais
    mi = MONTHS.index(month_key)
    if sec_key == 'fixes':
        frais_cat = 'fixes'
    elif sec_key == 'retraits':
        frais_cat = 'retraits'
    else:
        frais_cat = 'ponctuels'
    frais_list = data.frais.setdefault(frais_cat, [])
    fl = next((f for f in frais_list if f.name == line.name), None)
    if fl is None:
        fl = FraisLine(name=line.name, monthly=[0.0]*12)
        frais_list.append(fl)
    fl.monthly[mi] = line.banque + line.cash


# ─── Budget month detection ────────────────────────────────────────────────────

def detect_budget_month() -> str:
    now = datetime.date.today()
    mi  = now.month - 1          # 0-based
    if now.day < 25:
        mi = (mi - 1) % 12
    return MONTHS[mi]


# ─── Formatting helpers ────────────────────────────────────────────────────────

def fmt(n: float) -> str:
    s = f"{n:.2f}"
    if s.endswith('00'): return s[:-3]
    if s.endswith('0'):  return s[:-1]
    return s

def fmt_sign(n: float) -> str:
    return ('+' if n >= 0 else '') + fmt(n)

def today() -> str:
    return datetime.date.today().isoformat()