# 🇫🇷 Marianne — Assistant Administratif Français

> Application desktop souveraine, 100% locale, sans cloud, sans serveur, avec LLM embarqué.
> Vos données ne quittent jamais votre ordinateur.

## Présentation

**Marianne** est une IA locale qui aide les citoyens français à :
- Comprendre un courrier administratif en langage clair
- Connaître leurs droits (travail, CAF, URSSAF, logement, retraite)
- Rédiger des lettres de réclamation / contestation
- Naviguer dans les démarches administratives

Le tout **sans internet obligatoire**, sans compte, sans données envoyées nulle part.

## Stack technique

| Composant | Technologie |
|---|---|
| Backend | **Rust** (Tauri 2) |
| LLM | **Phi-3-Mini 3.8B** (Candle, GGUF Q4_K_M) |
| RAG | **GraphRAG hybride** (LanceDB vectoriel + petgraph) |
| Embeddings | fastembed (multilingual-e5-small) |
| Frontend | HTML/CSS/JS vanilla + marked.js |
| Recherche web | Filet de secours — sources officielles uniquement |

## Architecture

```
┌─────────────────────────────────────────┐
│       Frontend Tauri (WebView)          │
└────────────────┬────────────────────────┘
                 │ IPC batché (50ms)
┌────────────────▼────────────────────────┐
│         Backend Rust                     │
│  Commands · LLM Candle · GraphRAG       │
└───┬────────────────────────────┬────────┘
    │                            │
┌───▼──────────┐  ┌─────────────▼────────┐
│ Phi-3-Mini   │  │ LanceDB + petgraph   │
│ GPU/CPU auto │  │ Corpus légal français│
└──────────────┘  └──────────────────────┘
```

## Prérequis

- **Rust** ≥ 1.75 (`rustup`)
- **Tauri CLI** v2 (`cargo install tauri-cli`)
- **Protobuf compiler** (`protoc`)
- Windows : Visual Studio Build Tools + WebView2

## Démarrage rapide

```bash
cd marianne
cargo tauri dev
```

Au premier lancement, Marianne télécharge le modèle Phi-3-Mini (~2.2 Go) automatiquement avec reprise en cas d'interruption.

## Structure du projet

```
marianne/
├── src-tauri/src/
│   ├── main.rs            # Point d'entrée Tauri
│   ├── state.rs           # État global partagé
│   ├── commands/          # Commandes IPC (chat, setup)
│   ├── llm/               # Moteur LLM Candle (Phi-3)
│   ├── rag/               # GraphRAG (LanceDB + petgraph)
│   ├── prompts/           # Prompt système Marianne
│   └── history/           # Historique SQLite
├── frontend/              # Interface WebView
└── corpus/                # Données légales à ingérer
```

## Phases de développement

- [x] **Phase 1** — Squelette Tauri + architecture modules
- [ ] **Phase 2** — Moteur LLM (inférence Candle)
- [ ] **Phase 3** — Pipeline GraphRAG complet
- [ ] **Phase 4** — Interface utilisateur
- [ ] **Phase 5** — Fonctionnalités métier
- [ ] **Phase 6** — Optimisations performances
- [ ] **Phase 7** — Distribution & packaging
- [ ] **Phase 8** — Recherche web de secours

## Documentation

- [`guide_assistant_administratif_rust-3.md`](guide_assistant_administratif_rust-3.md) — Guide complet de développement
- [`marianne_phase8_recherche_web.md`](marianne_phase8_recherche_web.md) — Recherche web souveraine
- [`marianne_upgrades.md`](marianne_upgrades.md) — Améliorations V1 → V2

## Licence

Projet souverain français — données locales uniquement.