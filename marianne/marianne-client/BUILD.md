# Build Marianne Client

Guide pour compiler et packager l'application Electron.

## Pré-requis

- Node.js 20+
- npm 10+

## Installation

```bash
npm install
```

## Développement

```bash
# Lancer le serveur de développement
npm run dev

# Vite sera accessible sur http://localhost:5173
# Electron s'ouvrira automatiquement après que Vite soit prêt
```

## Build

### Compilation TypeScript + Svelte

```bash
# Build du renderer (Svelte + Vite)
npm run build:renderer

# Build du main process (TypeScript)
npm run build:main

# Build complet
npm run build
```

Les fichiers compilés sont dans `dist/`:
- `dist/main/` - Main process Electron
- `dist/preload/` - Preload script
- `dist/renderer/` - Application Svelte compilée

### Packaging

Créer des exécutables distribu
ables :

```bash
# Windows (.exe + .msi)
npm run package:win

# Linux (AppImage, .deb, .rpm)
npm run package:linux

# macOS (.dmg, .zip)
npm run package:mac

# Toutes les plateformes supportées (nécessite les outils natifs)
npm run package
```

Les packages sont créés dans `release/`.

## Structure du projet

```
marianne-client/
├── src/
│   ├── main/          # Main process Electron
│   │   ├── index.ts   # Entry point
│   │   ├── window.ts  # Window management
│   │   └── ipc/       # IPC handlers (files, terminal, server)
│   ├── preload/       # Preload script (API bridge)
│   └── renderer/      # Renderer process (Svelte UI)
│       ├── App.svelte
│       ├── components/
│       └── lib/
├── dist/              # Compiled output
├── release/           # Packaged applications
├── package.json
├── tsconfig.json      # Renderer TypeScript config
├── tsconfig.main.json # Main process TypeScript config
└── vite.config.ts     # Vite config
```

## Configuration electron-builder

Dans `package.json`, section `"build"`:

- **appId**: `fr.gouv.marianne`
- **productName**: `Marianne AI`
- **Formats**:
  - Windows: NSIS installer + portable
  - Linux: AppImage, .deb, .rpm
  - macOS: DMG, ZIP

## Dépannage

### Electron ne se lance pas en dev

Vérifiez que :
1. Vite tourne sur le port 5173
2. Le main process est compilé (`npm run build:main`)
3. Les imports ESM ont les extensions `.js`

### Erreur "Cannot find module"

Recompilez le main process :
```bash
npm run build:main
```

### Build échoue sur Linux

Installez les dépendances système :
```bash
sudo apt-get install -y \
  libgtk-3-dev \
  libnotify-dev \
  libnss3 \
  libxtst6
```

### Build échoue sur macOS sans certificat

Ajoutez dans les variables d'environnement :
```bash
export CSC_IDENTITY_AUTO_DISCOVERY=false
npm run package:mac
```

## CI/CD

Le workflow GitHub Actions `.github/workflows/build-client.yml` :
- Build multi-plateforme (Windows, Linux, macOS)
- Upload des artéfacts
- Intégration avec le workflow de release

Pour déclencher un build manuellement :
```bash
gh workflow run build-client.yml
```
