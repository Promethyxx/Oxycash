# Oxycash — Flet/Python

Budget tracker mensuel. Port fidèle de la version HTML.

## Stack
- **Flet** (Flutter + Python) — UI cross-platform
- **WebDAV** (Nextcloud / kDrive) — sync primaire
- **Fallback local** — `~/.oxycash/oxycash.json`

## Structure
```
oxycash/
├── main.py              # entry point Flet
├── core/
│   ├── model.py         # dataclasses + logique métier
│   ├── storage.py       # WebDAV + local JSON
│   └── theme.py         # palette dark/light
├── views/
│   ├── month_view.py    # page mensuelle (sections, payments)
│   └── special_views.py # Dettes, Épargne, Frais, Viabilité, Config
└── pyproject.toml
```

## Lancer en dev

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

Nécessite Flutter SDK + Android SDK installés.

```bash
flet build apk --project oxycash
# → build/apk/oxycash.apk
```

Pour debug sur device branché :
```bash
flet run main.py --android
```

## Données

| Plateforme | Emplacement local |
|---|---|
| Linux / Windows | `~/.oxycash/oxycash.json` |
| Android | app private storage (géré par Flet) |

Config WebDAV stockée dans `~/.oxycash/config.json`.

## WebDAV Nextcloud / kDrive

Dans l'onglet ⚙️ Config :
- **URL** : `https://xxx.nl.tab.digital/remote.php/dav/files/user/Oxy/`
- **Utilisateur** : ton email Nextcloud
- **Mot de passe** : app password recommandé

Le fichier `oxycash.json` est lu/écrit via HTTP PUT/GET — même logique que la version HTML.
