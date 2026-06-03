# Marianne Server API Documentation

Documentation complète de l'API REST du serveur HTTP `marianne-server`.

**Base URL**: `http://localhost:3000`  
**Version**: 1.0  
**Format**: JSON (sauf `/api/v1/chat` qui utilise SSE)

---

## Table des matières

- [Health Check](#health-check)
- [Chat](#chat)
- [Historique](#historique)
- [Profil utilisateur](#profil-utilisateur)
- [Documents](#documents)
- [Système](#système)
- [Gestion des modèles](#gestion-des-modèles)

---

## Health Check

### GET `/health`

Vérifie que le serveur est opérationnel.

**Réponse**:
```
ok
```

**Status**: `200 OK`

**Exemple**:
```bash
curl http://localhost:3000/health
# Réponse: ok
```

---

## Chat

### POST `/api/v1/chat`

Envoie un message à Marianne et reçoit une réponse en streaming via Server-Sent Events (SSE).

**Content-Type**: `application/json`  
**Response-Type**: `text/event-stream` (SSE)

**Corps de la requête** (`ChatRequest`):
```json
{
  "user_message": "Comment demander le RSA ?",
  "conversation_id": "conv-123",
  "use_rag": true,
  "use_web_search": false
}
```

**Paramètres**:
| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `user_message` | `string` | ✅ | Message de l'utilisateur |
| `conversation_id` | `string` | ❌ | ID de la conversation (généré si absent) |
| `use_rag` | `boolean` | ❌ | Activer la recherche dans le corpus local (défaut: `true`) |
| `use_web_search` | `boolean` | ❌ | Activer la recherche web (défaut: `false`) |

**Événements SSE**:

#### `stream-token`
Token de texte généré par le modèle (streaming temps réel).
```json
{
  "token": " le",
  "conversation_id": "conv-123"
}
```

#### `generation-done`
Génération terminée avec métadonnées complètes.
```json
{
  "assistant_message": "Pour demander le RSA, vous devez...",
  "conversation_id": "conv-123",
  "tokens_generated": 250,
  "generation_time_ms": 2340
}
```

#### `confidence-info`
Niveau de confiance de la réponse.
```json
{
  "score": 0.85,
  "level": "High",
  "explanation": "Réponse appuyée par 3 sources du corpus légal"
}
```

#### `contradiction-warning`
Alerte en cas de contradiction détectée.
```json
{
  "message": "Attention : incohérence détectée entre les sources"
}
```

#### `web-search-status`
Statut de la recherche web (si activée).
```json
{
  "status": "searching",
  "query": "RSA conditions 2026"
}
```

#### `offline-mode`
Notification du mode hors-ligne.
```json
{
  "message": "Mode hors-ligne : pas d'accès Internet"
}
```

**Exemple avec EventSource (JavaScript)**:
```javascript
const response = await fetch('http://localhost:3000/api/v1/chat', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    user_message: "Comment demander le RSA ?",
    use_rag: true
  })
});

const reader = response.body.getReader();
const decoder = new TextDecoder();

while (true) {
  const { done, value } = await reader.read();
  if (done) break;
  
  const chunk = decoder.decode(value);
  // Parser les événements SSE
  console.log(chunk);
}
```

**Status**:
- `200 OK`: Stream SSE ouvert
- `500 Internal Server Error`: Erreur de génération

---

## Historique

### GET `/api/v1/history/:conversation_id`

Récupère l'historique complet d'une conversation (tous les tours user/assistant).

**Paramètres URL**:
| Paramètre | Type | Description |
|-----------|------|-------------|
| `conversation_id` | `string` | Identifiant unique de la conversation |

**Réponse** (`Vec<ConversationTurn>`):
```json
[
  {
    "role": "user",
    "content": "Comment demander le RSA ?",
    "timestamp": 1717200000
  },
  {
    "role": "assistant",
    "content": "Pour demander le RSA...",
    "timestamp": 1717200005
  }
]
```

**Status**:
- `200 OK`: Historique récupéré
- `404 Not Found`: Conversation introuvable
- `500 Internal Server Error`: Erreur de lecture

**Exemple**:
```bash
curl http://localhost:3000/api/v1/history/conv-123
```

---

## Profil utilisateur

### GET `/api/v1/profile`

Récupère le profil utilisateur complet (préférences, informations personnelles).

**Réponse** (`UserProfile`):
```json
{
  "first_name": "Marie",
  "age": 32,
  "professional_status": "Salarie",
  "family_status": {
    "Parent": {
      "children_count": 2
    }
  },
  "department": "75",
  "topics_of_interest": ["logement", "impots"],
  "language_level": "Standard",
  "device_preference": "Gpu",
  "gpu_selection": "Auto",
  "selected_model": "phi-3-mini-q4",
  "updated_at": 1717200000
}
```

**Status**: `200 OK`

**Exemple**:
```bash
curl http://localhost:3000/api/v1/profile
```

---

### PUT `/api/v1/profile`

Met à jour le profil utilisateur.

**Content-Type**: `application/json`

**Corps de la requête** (`UserProfile`):
```json
{
  "first_name": "Marie",
  "age": 33,
  "professional_status": "Salarie",
  "family_status": "Celibataire",
  "department": "75",
  "topics_of_interest": ["logement"],
  "language_level": "Simple",
  "device_preference": "Cpu",
  "gpu_selection": "Auto",
  "selected_model": "phi-3-mini-q4",
  "updated_at": 1717286400
}
```

**Réponse**: Aucun contenu (corps vide)

**Status**:
- `204 No Content`: Profil mis à jour avec succès
- `500 Internal Server Error`: Erreur de sauvegarde

**Exemple**:
```bash
curl -X PUT http://localhost:3000/api/v1/profile \
  -H "Content-Type: application/json" \
  -d '{"first_name":"Marie","age":33,...}'
```

---

## Documents

### POST `/api/v1/documents/extract`

Extrait le contenu textuel d'un document (PDF, TXT) pour analyse.

**Content-Type**: `application/json`

**Corps de la requête** (`ExtractRequest`):
```json
{
  "file_path": "C:\\Users\\Marie\\Documents\\contrat.pdf",
  "question": "Quelles sont les clauses importantes ?"
}
```

**Paramètres**:
| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `file_path` | `string` | ✅ | Chemin absolu vers le fichier |
| `question` | `string` | ❌ | Question sur le document (défaut: "Résume ce document.") |

**Réponse** (`ExtractedDocument`):
```json
{
  "text": "CONTRAT DE TRAVAIL\n\nEntre...",
  "file_name": "contrat.pdf",
  "char_count": 12543,
  "prompt": "Document : contrat.pdf\n\nContenu:\n...\n\nQuestion : Quelles sont les clauses importantes ?\nRéponse :"
}
```

**Status**:
- `200 OK`: Document extrait avec succès
- `404 Not Found`: Fichier introuvable
- `403 Forbidden`: Accès refusé (répertoire système protégé)
- `422 Unprocessable Entity`: Format non supporté ou erreur d'extraction
- `500 Internal Server Error`: Erreur interne

**Formats supportés**:
- ✅ PDF (`.pdf`)
- ✅ Texte (`.txt`, `.md`, `.json`, etc.)
- ❌ Images (nécessite modèle multimodal)

**Sécurité**:
Accès bloqué aux répertoires système :
- Windows: `C:\Windows`, `C:\Program Files`
- Linux/macOS: `/etc`, `/usr`, `/bin`, `/sbin`, `/var`

**Exemple**:
```bash
curl -X POST http://localhost:3000/api/v1/documents/extract \
  -H "Content-Type: application/json" \
  -d '{"file_path":"C:\\Users\\Marie\\contrat.pdf","question":"Résume ce contrat"}'
```

---

## Système

### GET `/api/v1/system/info`

Récupère les informations système et du modèle (utilisé par le panneau Settings en mode web).

**Réponse** (`SystemInfo`):
```json
{
  "device": {
    "backend": "gpu",
    "label": "GPU (NVIDIA RTX 3060)",
    "gpu_available": true
  },
  "model": {
    "name": "Phi-3 Mini (Q4)",
    "active": true
  },
  "preference": {
    "device": "Gpu",
    "gpu_selection": "Auto"
  },
  "gpu_devices": [
    {
      "index": 0,
      "name": "NVIDIA GeForce RTX 3060",
      "device_type": "gpu",
      "vram_free_mb": 8192
    }
  ]
}
```

**Champs de la réponse**:

#### `device`
| Champ | Type | Description |
|-------|------|-------------|
| `backend` | `string` | `"gpu"` ou `"cpu"` |
| `label` | `string` | Description lisible (ex: "GPU (NVIDIA RTX 3060)") |
| `gpu_available` | `boolean` | GPU détecté sur la machine |

#### `model`
| Champ | Type | Description |
|-------|------|-------------|
| `name` | `string` | Nom du modèle actif (ex: "Phi-3 Mini (Q4)") |
| `active` | `boolean` | Modèle chargé en mémoire |

#### `preference`
| Champ | Type | Description |
|-------|------|-------------|
| `device` | `"Gpu"` \| `"Cpu"` | Préférence utilisateur |
| `gpu_selection` | `"Auto"` \| `"AllGpus"` \| `{"Specific": 0}` | Sélection GPU |

#### `gpu_devices`
Liste des GPU disponibles (vide si aucun GPU).
| Champ | Type | Description |
|-------|------|-------------|
| `index` | `number` | Index du GPU (pour sélection spécifique) |
| `name` | `string` | Nom du GPU (ex: "NVIDIA GeForce RTX 3060") |
| `device_type` | `string` | `"gpu"`, `"integrated_gpu"`, ou `"accelerator"` |
| `vram_free_mb` | `number` | VRAM libre en Mo |

**Status**: `200 OK`

**Exemple**:
```bash
curl http://localhost:3000/api/v1/system/info | jq .
```

---

## Gestion des modèles

### GET `/api/v1/models/status`

Récupère le statut des modèles téléchargés et du modèle actuellement chargé en mémoire.

**Réponse** (`ModelsStatus`):
```json
{
  "downloaded_models": [
    {
      "id": "phi-3-mini-q4",
      "name": "Phi-3 Mini (Q4)",
      "repo_id": "microsoft/Phi-3-mini-4k-instruct-gguf",
      "filename": "Phi-3-mini-4k-instruct-q4.gguf",
      "size_mb": 2200
    }
  ],
  "loaded_model": {
    "id": "phi-3-mini-q4",
    "name": "Phi-3 Mini (Q4)",
    "device": "gpu",
    "device_label": "GPU (NVIDIA RTX 3060)"
  }
}
```

**Champs de la réponse**:

#### `downloaded_models`
Liste des modèles téléchargés et disponibles.
| Champ | Type | Description |
|-------|------|-------------|
| `id` | `string` | Identifiant unique du modèle |
| `name` | `string` | Nom lisible |
| `repo_id` | `string` | Repo HuggingFace |
| `filename` | `string` | Nom du fichier GGUF |
| `size_mb` | `number` | Taille en Mo |

#### `loaded_model`
Modèle actuellement chargé en mémoire (null si aucun).
| Champ | Type | Description |
|-------|------|-------------|
| `id` | `string` | Identifiant du modèle actif |
| `name` | `string` | Nom du modèle |
| `device` | `string` | `"gpu"` ou `"cpu"` |
| `device_label` | `string` | Description lisible (ex: "GPU (NVIDIA RTX 3060)") |

**Status**: `200 OK`

**Exemple**:
```bash
curl http://localhost:3000/api/v1/models/status | jq .
```

---

### POST `/api/v1/models/download`

Télécharge un modèle depuis HuggingFace en arrière-plan.

**Content-Type**: `application/json`

**Corps de la requête** (`DownloadRequest`):
```json
{
  "repo_id": "microsoft/Phi-3.5-mini-instruct-gguf",
  "filename": "Phi-3.5-mini-instruct-Q4_K_M.gguf",
  "name": "Phi-3.5 Mini"
}
```

**Paramètres**:
| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `repo_id` | `string` | ✅ | Repo HuggingFace (ex: "microsoft/Phi-3-mini-4k-instruct-gguf") |
| `filename` | `string` | ✅ | Nom du fichier GGUF à télécharger |
| `name` | `string` | ✅ | Nom lisible du modèle |

**Réponse**:
```json
{
  "status": "started",
  "model_id": "microsoft_Phi-3.5-mini-instruct-gguf_Phi-3.5-mini-instruct-Q4_K_M"
}
```

**Status**:
- `200 OK`: Téléchargement démarré en arrière-plan
- `400 Bad Request`: Paramètres invalides
- `500 Internal Server Error`: Erreur de téléchargement

**Notes**:
- Le téléchargement s'effectue en arrière-plan
- Utiliser `/api/v1/models/status` pour vérifier l'avancement
- Support de la reprise automatique (HTTP Range)

**Exemple**:
```bash
curl -X POST http://localhost:3000/api/v1/models/download \
  -H "Content-Type: application/json" \
  -d '{
    "repo_id": "microsoft/Phi-3.5-mini-instruct-gguf",
    "filename": "Phi-3.5-mini-instruct-Q4_K_M.gguf",
    "name": "Phi-3.5 Mini"
  }'
```

---

### POST `/api/v1/models/load`

Charge un modèle téléchargé en mémoire (décharge l'ancien modèle si présent).

**Content-Type**: `application/json`

**Corps de la requête** (`LoadRequest`):
```json
{
  "model_id": "phi-3-mini-q4"
}
```

**Paramètres**:
| Champ | Type | Requis | Description |
|-------|------|--------|-------------|
| `model_id` | `string` | ✅ | Identifiant du modèle à charger |

**Réponse**:
```json
{
  "status": "loaded",
  "model_name": "Phi-3 Mini (Q4)",
  "device": "gpu"
}
```

**Status**:
- `200 OK`: Modèle chargé avec succès
- `400 Bad Request`: model_id manquant
- `404 Not Found`: Modèle non téléchargé
- `500 Internal Server Error`: Erreur de chargement

**Notes**:
- Décharge automatiquement l'ancien modèle avant de charger le nouveau
- Nécessite ~3 Go de RAM/VRAM selon le modèle
- Le chargement peut prendre 5-10 secondes

**Exemple**:
```bash
curl -X POST http://localhost:3000/api/v1/models/load \
  -H "Content-Type: application/json" \
  -d '{"model_id": "phi-3-mini-q4"}'
```

---

### POST `/api/v1/models/setup`

Réexécute la séquence d'initialisation complète (download + load + RAG).

**Content-Type**: `application/json`

**Corps de la requête** (optionnel):
```json
{}
```

**Réponse**:
```json
{
  "status": "completed",
  "model": "Phi-3 Mini (Q4)",
  "rag_chunks": 248
}
```

**Status**:
- `200 OK`: Initialisation complète réussie
- `500 Internal Server Error`: Erreur durant l'initialisation

**Notes**:
- Utilisé principalement pour le débogage ou la réinitialisation
- Télécharge le modèle par défaut si absent
- Charge le modèle en mémoire
- Réindexe le corpus RAG

**Exemple**:
```bash
curl -X POST http://localhost:3000/api/v1/models/setup
```

---

## Codes d'erreur HTTP

| Code | Description |
|------|-------------|
| `200 OK` | Requête réussie |
| `204 No Content` | Mise à jour réussie sans corps de réponse |
| `400 Bad Request` | Paramètres invalides |
| `403 Forbidden` | Accès refusé |
| `404 Not Found` | Ressource introuvable |
| `422 Unprocessable Entity` | Format de document non supporté |
| `500 Internal Server Error` | Erreur serveur |

---

## CORS et Sécurité

- **CORS**: Permissif en mode développement (`CorsLayer::permissive()`)
- **Tracing**: Logs HTTP via `TraceLayer`
- **Validation**: Chemins de fichiers validés (pas d'accès aux répertoires système)

---

## Démarrage du serveur

```bash
# Démarrage par défaut (127.0.0.1:3000)
cargo run --release

# Personnaliser le port
cargo run --release -- --bind 0.0.0.0:8080

# Spécifier le répertoire de données
cargo run --release -- --data-dir /chemin/vers/data
```

### 🚀 Installation automatique au premier lancement

Au premier démarrage, le serveur effectue automatiquement :

1. **Téléchargement de Phi-3 Mini** (~2.2 Go) si absent
2. **Chargement du modèle en mémoire** (RAM/VRAM)
3. **Indexation du corpus légal** (~248 passages)

**Logs typiques** :
```
📥 Premier lancement : téléchargement de Phi-3 Mini (~2.2 Go)...
📥 Phi-3-mini-4k-instruct-q4.gguf : 1500 / 2200 Mo (68%)
✅ Phi-3-mini-4k-instruct-q4.gguf téléchargé et enregistré
🔄 Chargement du modèle en mémoire...
✅ Modèle phi-3-mini-q4 chargé en mémoire
📚 Indexation du corpus légal...
✅ RAG initialisé : 248 chunks
✅ Marianne prête !
Écoute sur http://127.0.0.1:3000
```

Le serveur est **immédiatement opérationnel** après le premier lancement (aucune configuration manuelle requise).

---

## Notes d'implémentation

- **Chat streaming**: Utilise tokio channels et SSE pour le streaming temps réel
- **I/O asynchrone**: Toutes les opérations disque/réseau sont `async` (tokio)
- **State partagé**: `ServerState` encapsule `AppState` de `marianne-core` via `Arc`
- **Historique**: Stocké dans SQLite (`history.db`)
- **Profil**: Sérialisé en JSON (`profile.json`)
- **Modèles**: Registre JSON (`models/registry.json`)

---

**Dernière mise à jour**: 1er juin 2026  
**Auteur**: Équipe Marianne  
**Licence**: Voir LICENSE
