"""
Oxycash – Storage layer — multi-profile
  Each profile has its own JSON file: oxycash_<slug>.json
  Config: ~/.oxycash/config.json
    profiles: [{name, slug, dav_url, dav_user, dav_pass}, ...]
    active:   slug of active profile
    lang:     'en'|'fr'
    font_scale: int
    currency: str (default 'CHF')
"""
from __future__ import annotations
import json, pathlib, base64, re
from typing import Optional, Tuple, List
import urllib.request, urllib.error

from .model import AppData, default_data

LOCAL_DIR = pathlib.Path.home() / '.oxycash'
CONF_FILE = LOCAL_DIR / 'config.json'
UA        = 'Oxycash/1.0 (Python urllib)'


# ─── Config helpers ───────────────────────────────────────────────────────────

def _default_cfg() -> dict:
    return {
        'profiles': [{'name': 'Default', 'slug': 'default',
                      'dav_url': '', 'dav_user': '', 'dav_pass': ''}],
        'active':      'default',
        'lang':        'en',
        'font_scale':  0,
        'currency':    'CHF',
    }


def load_config() -> dict:
    if CONF_FILE.exists():
        try:
            d = json.loads(CONF_FILE.read_text(encoding='utf-8'))
            # migrate old flat config
            if 'profiles' not in d:
                d = {**_default_cfg(),
                     'lang':       d.get('lang', 'en'),
                     'font_scale': d.get('font_scale', 0),
                     'currency':   d.get('currency', 'CHF'),
                     'profiles': [{'name': 'Default', 'slug': 'default',
                                   'dav_url':  d.get('dav_url', ''),
                                   'dav_user': d.get('dav_user', ''),
                                   'dav_pass': d.get('dav_pass', '')}]}
            return d
        except Exception:
            pass
    return _default_cfg()


def save_config(cfg: dict):
    LOCAL_DIR.mkdir(parents=True, exist_ok=True)
    CONF_FILE.write_text(json.dumps(cfg, indent=2, ensure_ascii=False), encoding='utf-8')


def _slug(name: str) -> str:
    return re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_') or 'profile'


def _local_file(slug: str) -> pathlib.Path:
    return LOCAL_DIR / f'oxycash_{slug}.json'


def _dav_json_name(slug: str) -> str:
    return f'oxycash_{slug}.json'


# ─── WebDAV helpers ───────────────────────────────────────────────────────────

def _auth_header(user: str, pw: str) -> str:
    return 'Basic ' + base64.b64encode(f"{user}:{pw}".encode()).decode()


def _dav_req(url, method, user, pw, body=None):
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header('Authorization', _auth_header(user, pw))
    req.add_header('User-Agent', UA)
    req.add_header('OCS-APIREQUEST', 'true')
    if body:
        req.add_header('Content-Type', 'application/json; charset=utf-8')
    return req


def _dav_full_url(profile: dict) -> Optional[str]:
    url  = profile.get('dav_url', '').strip()
    user = profile.get('dav_user', '').strip()
    pw   = profile.get('dav_pass', '').strip()
    if not (url and user and pw):
        return None
    if not url.startswith('http://') and not url.startswith('https://'):
        url = 'https://' + url
    base = url if url.endswith('/') else url + '/'
    return base + _dav_json_name(profile.get('slug', 'default'))


def dav_test(profile: dict) -> Tuple[bool, str]:
    url  = profile.get('dav_url', '').strip()
    user = profile.get('dav_user', '').strip()
    pw   = profile.get('dav_pass', '').strip()
    if not (url and user and pw):
        return False, "Connexion échouée"
    full = _dav_full_url(profile)
    if not full:
        return False, "Connexion échouée"
    if not full.startswith('http://') and not full.startswith('https://'):
        full = 'https://' + full
    try:
        req = _dav_req(full, 'HEAD', user, pw)
        with urllib.request.urlopen(req, timeout=8) as r:
            return True, "Connecté ✓"
    except urllib.error.HTTPError as e:
        if e.code == 404:
            return True, "Connecté ✓"
        return False, "Connexion échouée"
    except Exception:
        return False, "Connexion échouée"


def _dav_load(profile: dict) -> Optional[AppData]:
    full = _dav_full_url(profile)
    if not full:
        return None
    user = profile['dav_user'].strip()
    pw   = profile['dav_pass'].strip()
    try:
        with urllib.request.urlopen(_dav_req(full, 'GET', user, pw), timeout=10) as r:
            d = json.loads(r.read().decode('utf-8'))
            if 'months' in d:
                return AppData.from_dict(d)
    except urllib.error.HTTPError as e:
        if e.code != 404: raise
    except Exception:
        pass
    return None


def _dav_save(profile: dict, data: AppData) -> bool:
    full = _dav_full_url(profile)
    if not full:
        return False
    user = profile['dav_user'].strip()
    pw   = profile['dav_pass'].strip()
    body = data.to_json().encode('utf-8')
    try:
        with urllib.request.urlopen(_dav_req(full, 'PUT', user, pw, body), timeout=10) as r:
            return r.status in (200, 201, 204)
    except Exception:
        return False


# ─── Storage manager ──────────────────────────────────────────────────────────

class Storage:
    def __init__(self):
        self.cfg      = load_config()
        self.dav_ok   = False
        self.data     = AppData()

    # ── profile helpers ───────────────────────────────────────────────────────

    @property
    def profiles(self) -> List[dict]:
        return self.cfg.setdefault('profiles', _default_cfg()['profiles'])

    @property
    def active_slug(self) -> str:
        return self.cfg.get('active', 'default')

    @property
    def active_profile(self) -> dict:
        slug = self.active_slug
        for p in self.profiles:
            if p['slug'] == slug:
                return p
        return self.profiles[0]

    @property
    def currency(self) -> str:
        return self.cfg.get('currency', 'CHF')

    def switch_profile(self, slug: str):
        self.cfg['active'] = slug
        save_config(self.cfg)
        self.load()

    def add_profile(self, name: str) -> str:
        base = _slug(name)
        slug = base
        existing = {p['slug'] for p in self.profiles}
        n = 2
        while slug in existing:
            slug = f"{base}_{n}"; n += 1
        self.profiles.append({'name': name, 'slug': slug,
                               'dav_url': '', 'dav_user': '', 'dav_pass': ''})
        save_config(self.cfg)
        return slug

    def rename_profile(self, slug: str, new_name: str):
        for p in self.profiles:
            if p['slug'] == slug:
                p['name'] = new_name
        save_config(self.cfg)

    def delete_profile(self, slug: str):
        if len(self.profiles) <= 1:
            return
        self.cfg['profiles'] = [p for p in self.profiles if p['slug'] != slug]
        f = _local_file(slug)
        if f.exists():
            f.unlink()
        if self.active_slug == slug:
            self.cfg['active'] = self.profiles[0]['slug']
        save_config(self.cfg)

    def save_profile_dav(self, slug: str, url: str, user: str, pw: str):
        for p in self.profiles:
            if p['slug'] == slug:
                p['dav_url']  = url
                p['dav_user'] = user
                p['dav_pass'] = pw
        save_config(self.cfg)

    def clear_config(self):
        prof = self.active_profile
        prof['dav_url']  = ''
        prof['dav_user'] = ''
        prof['dav_pass'] = ''
        self.dav_ok = False
        save_config(self.cfg)

    # ── load/save ─────────────────────────────────────────────────────────────

    def load(self) -> str:
        from .model import apply_recurring
        prof = self.active_profile
        slug = prof['slug']
        # try WebDAV
        try:
            app = _dav_load(prof)
            if app:
                self.data   = app
                self.dav_ok = True
                apply_recurring(self.data)
                _local_file(slug).parent.mkdir(parents=True, exist_ok=True)
                _local_file(slug).write_text(app.to_json(), encoding='utf-8')
                return 'dav'
        except Exception:
            pass
        self.dav_ok = False
        # try local
        lf = _local_file(slug)
        if lf.exists():
            try:
                d = json.loads(lf.read_text(encoding='utf-8'))
                if 'months' in d:
                    self.data = AppData.from_dict(d)
                    apply_recurring(self.data)
                    return 'local'
            except Exception:
                pass
        # default
        self.data = default_data()
        apply_recurring(self.data)
        _local_file(slug).parent.mkdir(parents=True, exist_ok=True)
        _local_file(slug).write_text(self.data.to_json(), encoding='utf-8')
        return 'default'

    def save(self) -> str:
        prof = self.active_profile
        slug = prof['slug']
        _local_file(slug).parent.mkdir(parents=True, exist_ok=True)
        _local_file(slug).write_text(self.data.to_json(), encoding='utf-8')
        if _dav_full_url(prof):
            ok = _dav_save(prof, self.data)
            self.dav_ok = ok
            if ok:
                return 'dav'
        self.dav_ok = False
        return 'local'

    def test_dav(self) -> Tuple[bool, str]:
        return dav_test(self.active_profile)

    def status(self) -> str:
        if self.dav_ok: return 'dav'
        if _dav_full_url(self.active_profile): return 'dav_err'
        return 'local'

    # ── lang / font / currency ────────────────────────────────────────────────

    def set_lang(self, lang: str):
        from .i18n import set_lang
        self.cfg['lang'] = lang
        save_config(self.cfg)
        set_lang(lang)

    def load_lang(self):
        from .i18n import set_lang
        set_lang(self.cfg.get('lang', 'en'))

    def set_currency(self, cur: str):
        self.cfg['currency'] = cur.strip() or 'CHF'
        save_config(self.cfg)

    # ── import / export ───────────────────────────────────────────────────────

    def export_json(self) -> str:
        return self.data.to_json()

    def import_json(self, raw: str) -> bool:
        try:
            app = AppData.from_json(raw)
            if app.months:
                self.data = app
                self.save()
                return True
        except Exception:
            pass
        return False

    def reset(self):
        self.data = default_data()
        self.save()