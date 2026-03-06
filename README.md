# CPAPersonal

![CPAPersonal Logo](./assets/logomain.png)

Local-first desktop manager for CLIProxyAPI workflows.

## Features

- Local desktop control panel for service lifecycle, providers, auth files and keys
- Built-in logs page, usage dashboards, and lightweight updater from GitHub Releases
- Chinese/English UI with settings for autostart and log/stat toggles

## Repositories

- Main repository: https://github.com/WEP-56/CPAPersonal
- Upstream project: https://github.com/router-for-me/CLIProxyAPI

## Desktop Build

```bash
cd desktop
npm install
npm run tauri build
```

Bundled Windows icon source is `assets/logo1.ico` (synced to `desktop/src-tauri/icons/icon.ico`).
