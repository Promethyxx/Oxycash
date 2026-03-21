# Oxycash - Rust version in progress
- The python project will be unfollowed by now.

# Oxycash — Flet/Python

Monthly budget tracker.

## Stack

- Flet (Flutter + Python) — cross-platform UI

- WebDAV (Nextcloud / kDrive) — primary sync

- Local fallback — ~/.oxycash/oxycash.json

## Structure
```
oxycash/
├── main.py              # Flet entry point
├── core/
│   ├── model.py         # dataclasses + business logic
│   ├── storage.py       # WebDAV + local JSON
│   └── theme.py         # dark/light theme
├── views/
│   ├── month_view.py    # monthly page (sections, payments)
│   └── special_views.py # Debts, Savings, Expenses, Viability, Settings
└── pyproject.toml
└── requirements.txt

```

## Run in development

```bash
pip install flet
python main.py
```

## Build Windows (exe)

```bash
pip install flet
flet build windows --project oxycash
# → build/windows/oxycash.exe
```

## Build Linux (AppImage / bundle)

```bash
flet build linux --project oxycash
# → build/linux/oxycash
```

## Build Android (APK)

Requires Flutter SDK + Android SDK installed.

```bash
flet build apk --project oxycash
# → build/apk/oxycash.apk
```


## Data

| Platforme | Local save |
|---|---|
| Linux / Windows | `~/.oxycash/oxycash.json` |
| Android | app private storage (managed by Flet) |

Config WebDAV is saved in `~/.oxycash/config.json`.

## WebDAV

In the ⚙️ Settings tab fill the folowing fields:
- **URL** : 
- **User** : 
- **Password** :

The oxycash.json file is read/written via HTTP PUT/GET.
