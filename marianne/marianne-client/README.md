# Marianne Client

Application desktop Electron pour Marianne AI. Se connecte au serveur `marianne-server` pour accéder à l'IA.

## Fonctionnalités

- 🖥️ Application desktop native (Windows, macOS, Linux)
- 📁 Accès au système de fichiers
- 💻 Exécution de commandes terminal
- 🔌 Connexion au serveur marianne-server
- 🎨 Interface Svelte moderne

## Prérequis

- Node.js 18+
- marianne-server en cours d'exécution

## Développement

```bash
# Installer les dépendances
npm install

# Lancer en mode développement
npm run dev

# Build pour production
npm run build

# Packager l'application
npm run package           # Détecte automatiquement l'OS
npm run package:win       # Windows
npm run package:mac       # macOS
npm run package:linux     # Linux
```

## Configuration

Au premier lancement, configurez l'URL du serveur dans les paramètres :
- Host : `localhost` (ou IP du serveur distant)
- Port : `3000`
- Protocol : `http` ou `https`

## API Electron

L'application expose une API sécurisée via le contexte isolé :

```typescript
// Fichiers
await window.electronAPI.file.openDialog();
await window.electronAPI.file.read(filePath);
await window.electronAPI.file.write(filePath, content);

// Terminal
await window.electronAPI.terminal.exec('echo "Hello"');

// Serveur
await window.electronAPI.server.getConfig();
await window.electronAPI.server.testConnection();
```

## Architecture

```
marianne-client/
├── src/
│   ├── main/           # Process principal Electron
│   │   ├── index.ts    # Point d'entrée
│   │   ├── window.ts   # Gestion fenêtre
│   │   └── ipc/        # Handlers IPC
│   ├── preload/        # Script preload sécurisé
│   └── renderer/       # UI Svelte
├── dist/               # Build output
└── release/            # Packages finaux
```
