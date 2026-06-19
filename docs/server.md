# Marianne Server (`marianne-server`)

Le composant `marianne-server` fournit une API REST et un flux Server-Sent Events (SSE) s'appuyant sur la bibliothèque `marianne-core`. 
Il a été conçu pour permettre une architecture client/serveur, où l'interface utilisateur (`marianne-client`) s'exécute séparément du moteur lourd de traitement de l'IA (qui requiert un GPU et de la RAM).

## Technologie

- **Framework Web** : [Axum](https://github.com/tokio-rs/axum) (basé sur l'écosystème Tokio).
- **Format** : Les requêtes et réponses classiques utilisent du `JSON`.
- **Streaming** : Les retours du modèle (génération LLM token-par-token) utilisent des flux **SSE** (`text/event-stream`).

## Authentification

Le serveur utilise un système de **clés API multi-utilisateurs** avec rôles stockées dans `api_keys.db` (SQLite). Chaque clé est stockée sous forme de hash SHA-256 — la clé brute n'est jamais persistée.

### Rôles

| Rôle | Accès |
|------|-------|
| `user` | Chat, historique, profil, documents, workspace |
| `admin` | Tout ce que `user` peut faire + gestion des modèles + gestion des clés |

### Utilisation

Toutes les requêtes sur les routes protégées doivent inclure le header :
```
Authorization: Bearer <votre-clé-api>
```

### Premier démarrage (bootstrap)

```bash
# Créer la première clé admin au démarrage
marianne-server --bootstrap-admin-key mk_<uuid>
# ou via variable d'environnement
MARIANNE_BOOTSTRAP_ADMIN_KEY=mk_... marianne-server
```

La clé bootstrap n'est insérée que si la table `api_keys` est vide. Ensuite, utilisez la route admin pour créer les clés des autres utilisateurs.

## Points d'API (Endpoints)

L'API est préfixée par `/api/v1/`. Tous les points d'entrée (handlers) sont gérés dans le dossier `src/routes/`.

### Routes publiques (sans authentification)

- `GET /health` : Vérification de santé du serveur.
- `GET /api/v1/system/info` : Informations système (RAM, CPU, GPU).
- `GET /api/v1/models/status` : Liste des modèles GGUF disponibles et modèle actif.

### Routes utilisateur (rôle `user` ou `admin`)

#### 1. Discussion & IA (`chat.rs`)
- `POST /api/v1/chat` : Envoie un message à un agent. Retourne un flux SSE asynchrone (tokens générés + exécution d'outils). L'historique est automatiquement scopé au `user_id` de la clé.

#### 2. Historique (`history.rs`)
- `GET /api/v1/history/conversations` : Liste les conversations de l'utilisateur courant.
- `GET /api/v1/history/:conversation_id` : Messages d'une conversation (déchiffrés à la volée).

#### 3. Profil & Documents
- `GET / PUT /api/v1/profile` : Gestion du profil utilisateur.
- `POST /api/v1/documents/extract` : Analyse de PDF et extraction de texte localement.

#### 4. Gestion Agentique (`workspace.rs`)
- `GET /api/v1/workspace/agents` : Liste les agents disponibles.
- `POST /api/v1/workspace/agents` : Crée ou modifie un agent.
- `DELETE /api/v1/workspace/agents/:id` : Supprime un agent.
- *(Similaire pour les `skills`)*.

### Routes admin (rôle `admin` uniquement)

#### 5. Gestion des clés API (`admin.rs`)
- `POST /api/v1/admin/keys` : Génère une nouvelle clé API. Corps : `{"user_id": "alice", "label": "laptop", "role": "user"}`. Retourne la clé brute **une seule fois**.
- `GET /api/v1/admin/keys` : Liste toutes les clés (hash + métadonnées, jamais la clé brute).
- `DELETE /api/v1/admin/keys/:key_hash` : Révoque une clé par son hash SHA-256.

#### 6. Modèles Dynamiques (`models.rs`)
- `POST /api/v1/models/download` : Lance le téléchargement asynchrone d'un modèle en tâche de fond.
- `POST /api/v1/models/load` : Charge un modèle en mémoire.
- `POST /api/v1/models/setup` : Installation et configuration automatique.
- `POST /api/v1/models/replace` : Télécharge un nouveau modèle HuggingFace, l'active, supprime l'ancien.
- `DELETE /api/v1/models/:id` : Supprime définitivement un modèle du système.
