# 🇫🇷 Marianne AI — Assistant Administratif Français

[![Release](https://img.shields.io/github/v/release/Coucoudb/Marianne?style=flat-square)](https://github.com/Coucoudb/Marianne/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> **v0.0.1** — Application desktop souveraine, 100% locale, sans cloud, sans serveur, avec LLM embarqué.
> Vos données ne quittent jamais votre ordinateur.

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