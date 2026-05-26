# 🇫🇷 Marianne AI — Assistant Administratif Français

[![Release](https://img.shields.io/github/v/release/Coucoudb/Marianne?style=flat-square)](https://github.com/Coucoudb/Marianne/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> **v0.0.2** — Application desktop souveraine, 100% locale, sans cloud, sans serveur, avec LLM embarqué.
> Vos données ne quittent jamais votre ordinateur.

## Présentation

**Marianne AI** est une intelligence artificielle locale qui aide les citoyens français à :

- 📄 Comprendre un courrier administratif en langage clair
- ⚖️ Connaître leurs droits (travail, CAF, URSSAF, logement, retraite, santé)
- ✍️ Rédiger des lettres de réclamation / contestation
- 🧭 Naviguer dans les démarches administratives
- 📑 Analyser des documents PDF (relevés, courriers officiels)

Le tout **sans internet obligatoire**, sans compte, sans données envoyées nulle part.

## Fonctionnalités

- **LLM local** — Inférence via llama.cpp (accélération GPU CUDA/Vulkan/Metal)
- **Catalogue de modèles** — Téléchargement depuis HuggingFace, registre local
- **RAG hybride** — Base vectorielle LanceDB + graphe de connaissances petgraph
- **Corpus juridique** — 13 fiches thématiques (CAF, travail, impôts, santé, retraite…)
- **Recherche web** — Sources officielles uniquement (service-public.fr, legifrance.gouv.fr…)
- **Feedback loop** — Les résultats web de qualité enrichissent automatiquement le RAG
- **Analyse PDF** — Extraction de texte depuis des documents administratifs
- **Historique** — Conversations sauvegardées localement (SQLite)
- **Streaming** — Réponses en temps réel token par token
- **API HTTP** — Serveur Axum avec streaming SSE pour intégrations tiers

## Stack technique

| Composant | Technologie |
|---|---|
| Core métier | **Rust** (`marianne-core`) |
| App desktop | **Tauri 2** (`marianne-tauri`) |
| Serveur HTTP | **Axum 0.7** + SSE (`marianne-server`) |
| LLM | **llama-cpp-2** (GGUF — compatible tout modèle) |
| GPU | **Vulkan** (principal) · CUDA · Metal (optionnel) |
| RAG | **GraphRAG hybride** (LanceDB vectoriel + petgraph) |
| Embeddings | fastembed (multilingual-e5-small, 384 dims) |
| Frontend desktop | HTML/CSS/JS vanilla + marked.js |
| Historique | SQLite (sqlx) |
| Recherche web | Sources officielles uniquement |

## Architecture

```
marianne/                          ← Workspace Cargo
├── marianne-core/                 ← 🧠 Logique métier (sans Tauri ni HTTP)
│   └── src/
│       ├── chat.rs                ←  Pipeline chat → Sender<ChatEvent>
│       ├── state.rs               ←  AppState (Clone, Arc partout)
│       ├── llm/                   ←  Moteur llama.cpp
│       ├── rag/                   ←  GraphRAG (LanceDB + petgraph)
│       ├── web/                   ←  Recherche web + cache
│       ├── documents/             ←  Extraction PDF
│       ├── prompts/               ←  Système de prompt
│       ├── history/               ←  SQLite
│       ├── profile/               ←  Profil utilisateur
│       ├── corpus/                ←  Corpus légal
│       ├── network/               ←  Connectivité
│       └── models.rs              ←  Registre modèles GGUF
│
├── src-tauri/                     ← 🖥️ App desktop (thin layer)
│   └── src/
│       ├── main.rs                ←  marianne_lib::run()
│       ├── lib.rs                 ←  Bootstrap Tauri + re-exports core
│       └── commands/              ←  IPC → core (chat, setup, profile…)
│
├── marianne-server/               ← ⚙️ Serveur HTTP binaire
│   └── src/
│       ├── main.rs                ←  Axum bootstrap (--bind, --data-dir)
│       ├── state.rs               ←  Arc<AppState>
│       └── routes/
│           ├── chat.rs            ←  POST /api/v1/chat → SSE
│           ├── history.rs         ←  GET /api/v1/history/:id
│           ├── profile.rs         ←  GET/PUT /api/v1/profile
│           └── documents.rs       ←  POST /api/v1/documents/extract
│
├── marianne-web/                  ← 🌐 Frontend Svelte (à créer)
└── frontend/                      ← Interface WebView Tauri actuelle
```

**Principe clé** : `marianne_core::chat::process_chat(state, request, tx: Sender<ChatEvent>)` est le seul point d'entrée du pipeline. Tauri mappe `ChatEvent` → `window.emit()`, Axum le mappe → SSE. Le core ne sait pas qui écoute.

## Prérequis

- **Rust** ≥ 1.75 (`rustup`)
- **Tauri CLI** v2 (`cargo install tauri-cli --version "^2.0"`)
- **CMake** ≥ 3.21
- Windows : Visual Studio Build Tools (MSVC) + WebView2
- *Optionnel* : **CUDA Toolkit** ≥ 12.0 + GPU NVIDIA

## Démarrage rapide

```bash
cd marianne

# App desktop — CPU
cargo tauri dev

# App desktop — GPU (Vulkan, compatible toute carte récente)
cargo tauri dev --features vulkan

# Serveur HTTP — CPU
cargo run -p marianne-server -- --bind 0.0.0.0:3000

# Serveur HTTP — GPU (Vulkan)
cargo run -p marianne-server --features vulkan -- --bind 0.0.0.0:3000

# GPU NVIDIA avec CUDA (accélération maximale)
cargo tauri dev --features cuda
```

Au premier lancement, configurez et téléchargez un modèle GGUF depuis l'interface (HuggingFace ou chemin local).

## API HTTP (`marianne-server`)

| Méthode | Route | Description |
|---|---|---|
| `POST` | `/api/v1/chat` | Chat en streaming SSE |
| `GET` | `/api/v1/history/:id` | Historique d'une conversation |
| `GET` | `/api/v1/profile` | Profil utilisateur |
| `PUT` | `/api/v1/profile` | Mettre à jour le profil |
| `POST` | `/api/v1/documents/extract` | Extraction de texte PDF |
| `GET` | `/health` | Healthcheck |

Exemple de consommation SSE :
```typescript
const es = new EventSource('/api/v1/chat');
es.addEventListener('stream-token', e => append(JSON.parse(e.data).token));
es.addEventListener('generation-done', e => finalize(JSON.parse(e.data)));
```

## Features Cargo

| Feature | Description |
|---|---|
| `default` | `custom-protocol` (Tauri) |
| `vulkan` | Accélération GPU Vulkan (principal — AMD, NVIDIA, Intel) |
| `cuda` | Accélération GPU NVIDIA CUDA (alternative) |
| `metal` | Accélération Apple Silicon |
| `vectordb` | Base vectorielle LanceDB |
| `fastembed` | Embeddings locaux multilingual-e5-small |

## Vérification

```bash
# Sans CUDA/MSVC (rapide)
cargo check -p marianne-server
cargo check -p marianne --no-default-features

# Avec CUDA (nécessite l'environnement MSVC + NVCC)
cargo check -p marianne --features cuda
```

## Phases de développement

- [x] **Phase 1** — Squelette Tauri + architecture modules
- [x] **Phase 2** — Moteur LLM (llama-cpp-2, GPU CUDA)
- [x] **Phase 3** — Pipeline GraphRAG complet (LanceDB + petgraph)
- [x] **Phase 4** — Interface utilisateur (streaming, markdown)
- [x] **Phase 5** — Fonctionnalités métier (corpus juridique, profils)
- [x] **Phase 6** — Optimisations performances (GPU, sampling)
- [x] **Phase 7** — Workspace multi-crates (core / tauri / server)
- [x] **Phase 8** — Recherche web souveraine + feedback loop RAG
- [ ] **Phase 9** — Frontend Svelte (`marianne-web`) + distribution

## Contribuer

```bash
cd marianne
cargo check -p marianne-server        # Vérifie core + server
cargo check -p marianne --no-default-features  # Vérifie la couche Tauri
cargo test
```

## Licence

MIT — Projet souverain français, données locales uniquement.


## Présentation

**Marianne AI** est une intelligence artificielle locale qui aide les citoyens français à :

- 📄 Comprendre un courrier administratif en langage clair
- ⚖️ Connaître leurs droits (travail, CAF, URSSAF, logement, retraite, santé)
- ✍️ Rédiger des lettres de réclamation / contestation
- 🧭 Naviguer dans les démarches administratives
- 📑 Analyser des documents PDF (relevés, courriers officiels)

Le tout **sans internet obligatoire**, sans compte, sans données envoyées nulle part.

## Fonctionnalités v0.0.1

- **LLM local** — Inférence Phi-3-Mini 3.8B via llama.cpp (accélération GPU CUDA)
- **RAG hybride** — Base vectorielle LanceDB + graphe de connaissances petgraph
- **Corpus juridique** — 13 fiches thématiques (CAF, travail, impôts, santé, retraite…)
- **Recherche web** — Sources officielles uniquement (service-public.fr, legifrance.gouv.fr…)
- **Feedback loop** — Les résultats web de qualité enrichissent automatiquement le RAG
- **Analyse PDF** — Extraction de texte depuis des documents administratifs
- **Historique** — Conversations sauvegardées localement (SQLite)
- **Streaming** — Réponses en temps réel token par token

## Stack technique

| Composant | Technologie |
|---|---|
| Backend | **Rust** (Tauri 2) |
| LLM | **Phi-3-Mini 3.8B** (llama-cpp-2, GGUF Q4_K_M) |
| GPU | **CUDA** (optionnel — NVIDIA, chargement ~1.2s) |
| RAG | **GraphRAG hybride** (LanceDB vectoriel + petgraph) |
| Embeddings | fastembed (multilingual-e5-small, 384 dims) |
| Frontend | HTML/CSS/JS vanilla + marked.js |
| Historique | SQLite (sqlx) |
| Recherche web | Sources officielles uniquement (filet de secours) |

## Architecture

```
┌─────────────────────────────────────────┐
│       Frontend Tauri (WebView)          │
└────────────────┬────────────────────────┘
                 │ IPC streaming
┌────────────────▼────────────────────────┐
│         Backend Rust                     │
│  Commands · LLM · GraphRAG · Web        │
└───┬──────────────┬──────────────┬───────┘
    │              │              │
┌───▼──────┐ ┌────▼────────┐ ┌──▼──────────┐
│ Phi-3    │ │ LanceDB +   │ │ Recherche   │
│ llama.cpp│ │ petgraph    │ │ web officiel│
│ GPU/CPU  │ │ Corpus légal│ │ → feedback  │
└──────────┘ └─────────────┘ └─────────────┘
```

## Prérequis

- **Rust** ≥ 1.75 (`rustup`)
- **Tauri CLI** v2 (`cargo install tauri-cli --version "^2.0"`)
- **Protobuf compiler** (`protoc`)
- **CMake** ≥ 3.21
- Windows : Visual Studio Build Tools (MSVC) + WebView2
- *Optionnel* : **CUDA Toolkit** ≥ 12.0 + GPU NVIDIA (pour accélération GPU)

## Démarrage rapide

```bash
cd marianne

# Build CPU (par défaut — inclut fastembed + vectordb)
cargo tauri dev

# Build avec accélération GPU NVIDIA
cargo tauri dev --features cuda
```

Au premier lancement, Marianne télécharge le modèle Phi-3-Mini (~2.2 Go) automatiquement avec reprise en cas d'interruption.

## Structure du projet

```
marianne/
├── src-tauri/src/
│   ├── main.rs            # Point d'entrée Tauri
│   ├── state.rs           # État global partagé
│   ├── commands/          # Commandes IPC (chat, setup, corpus, documents, profile)
│   ├── llm/               # Moteur LLM llama-cpp-2 (engine, sampler, streamer)
│   ├── rag/               # RAG hybride (embedder, retriever, store, graph, feedback)
│   ├── web/               # Recherche web (searcher, cache, sources officielles)
│   ├── documents/         # Extraction PDF
│   ├── prompts/           # Prompt système Marianne
│   ├── network/           # Détection connectivité
│   ├── profile/           # Profil utilisateur
│   └── history/           # Historique SQLite
├── frontend/              # Interface WebView
│   ├── index.html
│   ├── scripts/app.js
│   └── styles/main.css
└── corpus/                # 13 fiches thématiques (CAF, travail, impôts…)
```

## Features Cargo

| Feature | Description |
|---|---|
| `default` | `custom-protocol` + `fastembed` + `vectordb` |
| `cuda` | Accélération GPU NVIDIA (llama.cpp CUDA + fastembed + vectordb) |
| `vectordb` | Base vectorielle LanceDB (RAG persistant) |
| `fastembed` | Embeddings locaux multilingual-e5-small |

## Phases de développement

- [x] **Phase 1** — Squelette Tauri + architecture modules
- [x] **Phase 2** — Moteur LLM (llama-cpp-2, GPU CUDA)
- [x] **Phase 3** — Pipeline GraphRAG complet (LanceDB + petgraph)
- [x] **Phase 4** — Interface utilisateur (streaming, markdown)
- [x] **Phase 5** — Fonctionnalités métier (corpus juridique, profils)
- [x] **Phase 6** — Optimisations performances (GPU, sampling)
- [ ] **Phase 7** — Distribution & packaging (en cours)
- [x] **Phase 8** — Recherche web souveraine + feedback loop RAG

## Documentation


## Contribuer

```bash
# Vérifier la compilation
cd marianne
cargo check --features cuda

# Lancer les tests
cargo test
```

## Licence

MIT — Projet souverain français, données locales uniquement.