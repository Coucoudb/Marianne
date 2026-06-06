# Marianne Server (`marianne-server`)

Le composant `marianne-server` fournit une API REST et un flux Server-Sent Events (SSE) s'appuyant sur la bibliothèque `marianne-core`. 
Il a été conçu pour permettre une architecture client/serveur, où l'interface utilisateur (`marianne-client`) s'exécute séparément du moteur lourd de traitement de l'IA (qui requiert un GPU et de la RAM).

## Technologie

- **Framework Web** : [Axum](https://github.com/tokio-rs/axum) (basé sur l'écosystème Tokio).
- **Format** : Les requêtes et réponses classiques utilisent du `JSON`.
- **Streaming** : Les retours du modèle (génération LLM token-par-token) utilisent des flux **SSE** (`text/event-stream`).

## Points d'API (Endpoints)

L'API est préfixée par `/api/v1/`. Tous les points d'entrée (handlers) sont gérés dans le dossier `src/routes/`.

### 1. Discussion & IA (`chat.rs`)
- `POST /api/v1/chat` : Envoie un message à un agent (principal ou spécialisé). Retourne un flux SSE asynchrone contenant le streaming des mots générés par l'IA et l'exécution d'outils.

### 2. Modèles Dynamiques (`models.rs`)
- `GET /api/v1/models/status` : Retourne la liste des modèles GGUF téléchargés et le modèle actuellement actif en mémoire.
- `POST /api/v1/models/download` : Lance le téléchargement asynchrone d'un modèle en tâche de fond.
- `POST /api/v1/models/replace` : Télécharge un nouveau modèle HuggingFace, l'active, supprime l'ancien du disque et recharge le moteur.
- `DELETE /api/v1/models/:id` : Supprime définitivement un modèle du système.

### 3. Gestion Agentique (`workspace.rs`)
- `GET /api/v1/workspace/agents` : Liste les agents disponibles.
- `POST /api/v1/workspace/agents` : Crée ou modifie un agent.
- `DELETE /api/v1/workspace/agents/:id` : Supprime un agent.
- *(Similaire pour les `skills`)*.

### 4. Paramètres & Historique
- `GET / PUT /api/v1/profile` : Gestion du profil utilisateur.
- `GET /api/v1/history/:id` : Récupère les messages passés d'une conversation.
- `POST /api/v1/documents/extract` : Analyse de PDF et extraction de texte localement.
