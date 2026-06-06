# Guide de démarrage rapide - Marianne Client

## Installation et test

### 1. Installer les dépendances

```bash
cd marianne/marianne-client
npm install
```

### 2. Démarrer marianne-server

Avant de lancer le client, assurez-vous que le serveur est en cours d'exécution :

```bash
cd marianne/marianne-server
cargo run --release
```

Le serveur devrait démarrer sur `http://localhost:3000`.

### 3. Lancer le client en mode développement

```bash
cd marianne/marianne-client
npm run dev
```

Cela va :
- Démarrer Vite dev server sur le port 5173
- Lancer l'application Electron qui se connecte au dev server
- Activer le hot-reload pour les modifications du code

### 4. Tester les fonctionnalités

Une fois l'application lancée :

1. **Vérifier la connexion au serveur**
   - Par défaut: `http://localhost:3000`
   - Cliquez sur "Tester la connexion"
   - Le statut devrait passer à "● Connecté"

2. **Tester l'accès aux fichiers**
   - Cliquez sur "📁 Ouvrir un fichier"
   - Sélectionnez un fichier (PDF, TXT, MD...)
   - Vérifiez la console DevTools (F12)

3. **Tester l'exécution de commandes**
   - Cliquez sur "💻 Exécuter une commande"
   - Vérifiez la sortie dans la console

## Build pour production

```bash
# Build et package pour votre OS
npm run package

# Ou spécifique par OS
npm run package:win     # Windows (.exe, .msi)
npm run package:mac     # macOS (.dmg)
npm run package:linux   # Linux (.AppImage, .deb, .rpm)
```

Les fichiers packagés seront dans `marianne-client/release/`.

## Structure du projet

```
marianne-client/
├── src/
│   ├── main/              # Process principal Electron (Node.js)
│   │   ├── index.ts       # Point d'entrée
│   │   ├── window.ts      # Gestion de la fenêtre
│   │   └── ipc/           # Handlers IPC (files, terminal, server)
│   ├── preload/           # Script preload (bridge sécurisé)
│   │   └── index.ts       # Expose l'API au renderer
│   └── renderer/          # Interface utilisateur (Svelte)
│       ├── App.svelte     # Composant principal
│       ├── main.ts        # Point d'entrée Svelte
│       └── app.css        # Styles globaux
├── dist/                  # Build output
│   ├── main/              # JS compilé (main process)
│   ├── preload/           # JS compilé (preload)
│   └── renderer/          # Build Vite (HTML/CSS/JS)
└── release/               # Packages finaux (.exe, .dmg, etc.)
```

## API Electron disponible

Dans les composants Svelte, utilisez `window.electronAPI` :

```typescript
// Fichiers
const files = await window.electronAPI.file.openDialog();
const content = await window.electronAPI.file.read(filePath);
await window.electronAPI.file.write(filePath, content);
const items = await window.electronAPI.file.listDir(dirPath);

// Terminal
const result = await window.electronAPI.terminal.exec('ls -la');
const session = await window.electronAPI.terminal.create('session-1');
await window.electronAPI.terminal.input('session-1', 'echo hello');

// Serveur
const config = await window.electronAPI.server.getConfig();
await window.electronAPI.server.setConfig({ host, port, protocol });
const status = await window.electronAPI.server.testConnection();

// App
const version = await window.electronAPI.app.getVersion();
```

## Prochaines étapes

1. **Migrer les composants UI de marianne-web**
   - Chat interface
   - History sidebar
   - Document viewer
   - Profile settings

2. **Implémenter la communication avec marianne-server**
   - API HTTP client
   - WebSocket pour le streaming
   - Gestion des sessions

3. **CI/CD**
   - Workflow GitHub Actions
   - Build multi-plateforme
   - Code signing
   - Auto-updater

## Dépannage

### Le serveur n'est pas accessible
- Vérifiez que marianne-server est en cours d'exécution
- Vérifiez l'URL dans la configuration (host, port, protocol)

### Erreur de compilation TypeScript
```bash
npm run build:main
```

### DevTools ne s'ouvre pas
Modifiez `src/main/window.ts` :
```typescript
window.webContents.openDevTools({ mode: 'detach' });
```

### Build Electron échoue
- Vérifiez que toutes les dépendances sont installées
- Supprimez `node_modules` et `dist`, puis réinstallez

```bash
rm -rf node_modules dist
npm install
```
