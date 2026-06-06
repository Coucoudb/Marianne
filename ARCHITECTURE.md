# Architecture Marianne AI

## Vue d'ensemble

Marianne AI utilise une architecture **client-serveur** où :
- **Le serveur** (`marianne-server`) héberge le modèle IA et traite les requêtes
- **Le client** (`marianne-client`) fournit l'interface utilisateur et se connecte au serveur

```
┌─────────────────────────────────────────────────────────────┐
│                     marianne-client                          │
│  (Application Electron - Windows, macOS, Linux)             │
│                                                              │
│  • Interface Svelte                                         │
│  • Accès fichiers locaux                                    │
│  • Exécution commandes terminal                             │
│  • Gestion sessions utilisateur                             │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       │ HTTP + WebSocket (SSE)
                       │
┌──────────────────────▼──────────────────────────────────────┐
│                     marianne-server                          │
│  (Serveur Rust - Axum + llama.cpp)                          │
│                                                              │
│  • Modèle LLM (GGUF via llama.cpp)                          │
│  • RAG hybride (LanceDB + graphe)                           │
│  • Corpus juridique français                                │
│  • Recherche web (sources officielles)                      │
│  • Historique conversations (SQLite)                        │
│  • API REST + streaming SSE                                 │
└─────────────────────────────────────────────────────────────┘
```

## Composants

### marianne-client (Electron)

**Technologie** : Electron + Svelte + TypeScript

**Responsabilités** :
- Interface utilisateur moderne et réactive
- Gestion de la connexion au serveur
- Accès au système de fichiers (documents PDF, etc.)
- Exécution de commandes terminal
- Configuration utilisateur locale

**Structure** :
```
marianne-client/
├── src/
│   ├── main/           # Process principal Electron
│   │   ├── index.ts    # Point d'entrée
│   │   ├── window.ts   # Gestion fenêtre
│   │   └── ipc/        # Handlers IPC
│   │       ├── files.ts      # Accès fichiers
│   │       ├── terminal.ts   # Exécution commandes
│   │       └── server.ts     # Configuration serveur
│   ├── preload/        # Bridge sécurisé
│   │   └── index.ts    # API exposée au renderer
│   └── renderer/       # UI Svelte
│       ├── lib/
│       │   ├── api.ts       # Client API HTTP/SSE
│       │   ├── types.ts     # Types TypeScript
│       │   ├── markdown.ts  # Parsing markdown
│       │   └── sources.ts   # Formattage sources
│       ├── components/
│       │   ├── ChatMessages.svelte
│       │   └── InputArea.svelte
│       └── App.svelte
└── package.json
```

**API Electron exposée** :
```typescript
window.electronAPI {
  file: {
    openDialog()
    saveDialog()
    read(path)
    write(path, content)
    listDir(path)
    stat(path)
  }
  terminal: {
    exec(command, cwd)
    create(sessionId, cwd)
    input(sessionId, input)
    close(sessionId)
  }
  server: {
    getConfig()
    setConfig(config)
    testConnection(config?)
  }
  app: {
    getVersion()
  }
}
```

### marianne-server (Rust)

**Technologie** : Rust + Axum + llama.cpp

**Responsabilités** :
- Héberger le modèle LLM (GGUF)
- Traiter les requêtes de chat avec RAG
- Gérer l'historique des conversations
- Fournir l'API REST + SSE
- Recherche web sur sources officielles
- Extraction de texte depuis PDF

**Structure** :
```
marianne-server/
├── src/
│   ├── main.rs         # Point d'entrée Axum
│   ├── state.rs        # AppState partagé
│   └── routes/
│       ├── chat.rs     # POST /chat (streaming SSE)
│       ├── history.rs  # GET/DELETE /history/...
│       ├── profile.rs  # GET/POST /profile
│       ├── models.rs   # GET /models/...
│       └── documents.rs # POST /documents/extract
└── Cargo.toml
```

**API REST** :
```
POST /chat                          # Chat avec streaming SSE
GET  /history/conversations         # Liste conversations
GET  /history/conversations/:id     # Conversation spécifique
DELETE /history/conversations/:id   # Supprimer conversation
GET  /profile                       # Profil utilisateur
POST /profile                       # Mettre à jour profil
GET  /models                        # Liste modèles disponibles
GET  /models/system-info            # Info système (GPU, etc.)
POST /models/select                 # Sélectionner modèle
POST /documents/extract             # Extraire texte PDF
GET  /health                        # Healthcheck
```

### marianne-core (Rust)

**Technologie** : Rust (bibliothèque)

**Responsabilités** :
- Logique métier partagée
- Pipeline de chat
- Moteur LLM (llama.cpp)
- RAG hybride (vectoriel + graphe)
- Corpus juridique
- Recherche web
- Extraction documents
- Historique SQLite
- Gestion profils utilisateurs

**Structure** :
```
marianne-core/
└── src/
    ├── chat.rs         # Pipeline chat
    ├── state.rs        # AppState
    ├── llm/            # Moteur llama.cpp
    ├── rag/            # RAG hybride
    ├── web/            # Recherche web
    ├── documents/      # Extraction PDF
    ├── prompts/        # Système de prompt
    ├── history/        # SQLite
    ├── profile/        # Profils
    ├── corpus/         # Corpus légal
    ├── network/        # Connectivité
    └── models.rs       # Registre modèles
```

## Communication client-serveur

### HTTP REST
Client → Serveur : requêtes JSON classiques
```typescript
// Get profile
const profile = await fetch('/profile').then(r => r.json());

// Update profile
await fetch('/profile', {
  method: 'POST',
  body: JSON.stringify(profile)
});
```

### Server-Sent Events (SSE)
Serveur → Client : streaming temps réel

```typescript
// Chat streaming
const response = await fetch('/chat', {
  method: 'POST',
  body: JSON.stringify({ prompt, conversation_id })
});

const reader = response.body.getReader();
const decoder = new TextDecoder();

while (true) {
  const { done, value } = await reader.read();
  if (done) break;

  const chunk = decoder.decode(value);
  // Parse SSE: "data: {...}\n\n"
  const lines = chunk.split('\n');
  for (const line of lines) {
    if (line.startsWith('data: ')) {
      const data = JSON.parse(line.slice(6));
      if (data.token) {
        // Afficher token
      }
      if (data.metadata) {
        // Afficher sources, stats, etc.
      }
    }
  }
}
```

## Déploiement

### Option 1 : Machine unique (dev)
```
localhost
  ├─ marianne-server (port 3000)
  └─ marianne-client → http://localhost:3000
```

### Option 2 : Serveur dédié
```
Machine puissante (GPU)
  └─ marianne-server (0.0.0.0:3000)

Laptop/Desktop
  └─ marianne-client → http://192.168.1.100:3000
```

### Option 3 : Plusieurs clients
```
Serveur NAS/Desktop (GPU)
  └─ marianne-server (0.0.0.0:3000)

Client 1 (laptop)
  └─ marianne-client → http://server:3000

Client 2 (desktop)
  └─ marianne-client → http://server:3000

Client 3 (autre laptop)
  └─ marianne-client → http://server:3000
```

## Avantages de l'architecture

### Pour l'utilisateur
- **Client léger** : ~100 Mo (vs ~2.2 Go avec Tauri)
- **Flexibilité** : serveur sur machine puissante, client sur laptop
- **Multi-clients** : plusieurs utilisateurs peuvent utiliser le même serveur
- **Mise à jour** : mise à jour du client sans re-télécharger le modèle

### Pour le développement
- **Séparation des responsabilités** : UI vs logique IA
- **Tests** : serveur testable indépendamment via API REST
- **Scalabilité** : possible d'ajouter load balancing, cache, etc.
- **Multi-plateforme** : client Electron universel

## Migration depuis Tauri

Voir [MIGRATION.md](MIGRATION.md) pour les détails de migration.

**Résumé** :
- ✅ UI Svelte réutilisée
- ✅ Fonctionnalités préservées (chat, RAG, corpus, etc.)
- ✅ Accès fichiers via IPC Electron
- ⚠️ Nécessite marianne-server en cours d'exécution
- ⚠️ Configuration initiale de l'URL serveur

## Sécurité

### Client (Electron)
- **Context Isolation** : activé
- **Node Integration** : désactivé dans renderer
- **Preload script** : bridge sécurisé IPC
- **CSP** : Content Security Policy configurée
- **Sandbox** : false (nécessaire pour accès fichiers)

### Serveur
- **Pas d'authentification** : pour MVP local
- **CORS** : configuré pour localhost uniquement
- **Rate limiting** : à implémenter pour production
- **HTTPS** : supporté (certificat auto-signé ou Let's Encrypt)

## Performances

### Client
- **Taille** : ~100 Mo installé
- **RAM** : ~150 Mo au repos
- **CPU** : minimal (UI uniquement)

### Serveur
- **RAM** : 4-8 Go (selon modèle LLM)
- **GPU VRAM** : 4-8 Go (selon modèle et quantization)
- **CPU** : 4+ cores recommandés
- **Stockage** : 2-4 Go (modèle) + historique

## Limitations actuelles

- ❌ Pas d'authentification multi-utilisateurs
- ❌ Pas de chiffrement end-to-end
- ❌ Pas de synchronisation historique entre clients
- ❌ Pas de gestion de permissions fichiers côté serveur

## Roadmap

### v0.2.0
- [ ] Authentification basique (token)
- [ ] Gestion multi-utilisateurs côté serveur
- [ ] Synchronisation historique
- [ ] Support HTTPS avec certificat

### v0.3.0
- [ ] Code signing pour les packages client
- [ ] Auto-updater Electron
- [ ] Cache intelligent côté client
- [ ] Notifications système

### v1.0.0
- [ ] Mode offline avec modèle local (option Tauri)
- [ ] Plugins pour extensions tierces
- [ ] API publique documentée
- [ ] Monitoring et analytics
