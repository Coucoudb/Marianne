# Migration vers architecture Client-Serveur

## Changements architecturaux

### Avant (v0.0.x)
```
marianne/
├── marianne-web/          # Frontend web Svelte (statique)
├── src-tauri/             # Application Tauri (IA locale embarquée)
├── marianne-server/       # Serveur HTTP (IA embarquée)
└── marianne-core/         # Logique partagée
```

**Architecture Tauri :**
- Application desktop standalone avec IA locale
- Modèle téléchargé (~2.2 Go) dans l'app
- Variantes : CPU, CUDA, Vulkan, Metal

### Après (v0.1.0+)
```
marianne/
├── marianne-client/       # Client Electron (nouveau)
├── marianne-server/       # Serveur HTTP (IA embarquée)
└── marianne-core/         # Logique partagée
```

**Architecture Client-Serveur :**
- **marianne-client** : Application Electron légère
  - Se connecte obligatoirement à marianne-server
  - Accès fichiers locaux
  - Exécution commandes terminal
  - Interface Svelte réutilisée
  
- **marianne-server** : Serveur backend avec IA
  - API REST + WebSocket pour streaming
  - Modèle téléchargé côté serveur
  - Variantes GPU : CPU, CUDA, Vulkan, Metal

## Avantages

### Client léger
- Pas de téléchargement de modèle lourd
- Mises à jour plus rapides
- Moins d'espace disque requis

### Serveur centralisé
- Un seul serveur pour plusieurs clients
- IA sur machine puissante (GPU dédié)
- Mise à jour du modèle centralisée

### Flexibilité
- Client sur laptop, serveur sur NAS/Desktop
- Client sur plusieurs machines → même serveur
- Serveur distant accessible via réseau local/VPN

## Ce qui est conservé

- Interface utilisateur Svelte
- Thème Marianne (🇫🇷)
- Fonctionnalités :
  - Chat avec IA
  - RAG hybride (vectoriel + graphe)
  - Corpus juridique français
  - Recherche web temps réel
  - Analyse documents PDF/TXT/MD

## Ce qui change

### Pour l'utilisateur final

**Avant (Tauri) :**
1. Télécharger l'application (~2.2 Go avec modèle)
2. Premier lancement : téléchargement modèle
3. Utilisation standalone

**Après (Electron) :**
1. Télécharger marianne-server (~variante GPU)
2. Lancer le serveur : `./marianne-server`
3. Télécharger marianne-client (~100 Mo)
4. Configurer URL serveur dans le client
5. Utiliser le client

### Configuration requise

**Serveur** (machine puissante recommandée) :
- CPU : 4+ cores
- RAM : 8+ Go
- GPU (optionnel) : NVIDIA RTX/GTX, AMD/Intel (Vulkan), Apple Silicon (Metal)

**Client** (machine légère OK) :
- N'importe quel laptop/desktop moderne
- Connexion réseau au serveur (local ou distant)

## Migration depuis Tauri

Si vous utilisiez l'application Tauri :

1. **Arrêtez l'application Tauri**
2. **Installez marianne-server** sur votre machine puissante :
   ```bash
   # Extraire l'archive
   tar xzf marianne-server-linux-x64-cuda.tar.gz  # ou autre variante
   
   # Lancer le serveur
   ./marianne-server --bind 0.0.0.0:3000
   ```

3. **Installez marianne-client** sur votre/vos machine(s) :
   - Windows : `.exe`
   - Linux : `.AppImage` / `.deb`
   - macOS : `.dmg`

4. **Configurez le client** :
   - Lancez marianne-client
   - Configurez l'URL : `http://IP-DU-SERVEUR:3000`
   - Testez la connexion

5. **Migration des données** (optionnel) :
   - Les conversations Tauri sont dans `~/.local/share/marianne` (Linux) ou équivalent
   - Vous pouvez les réimporter via l'interface du client

## Développement

### Lancer en mode dev

**Serveur** :
```bash
cd marianne/marianne-server
cargo run --release
```

**Client** :
```bash
cd marianne/marianne-client
npm install
npm run dev
```

### Build

**Serveur** :
```bash
cd marianne/marianne-server
cargo build --release --no-default-features --features fastembed,vectordb,cuda
```

**Client** :
```bash
cd marianne/marianne-client
npm run build
npm run package:win    # ou package:linux, package:mac
```

## Prochaines étapes

- [ ] Migrer les composants UI complets de marianne-web
- [ ] Implémenter communication WebSocket pour streaming
- [ ] Ajouter gestion de sessions/authentification
- [ ] Supporter multi-utilisateurs côté serveur
- [ ] Code signing pour les packages client
- [ ] Auto-updater Electron

## Questions / Support

Pour toute question sur la migration :
- Ouvrez une issue GitHub
- Consultez la documentation : `marianne-client/README.md`
- Guide de démarrage rapide : `marianne-client/QUICKSTART.md`
