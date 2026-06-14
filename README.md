# 🇫🇷 Marianne AI — Assistant Administratif Français

[![Release](https://img.shields.io/github/v/release/Coucoudb/Marianne?style=flat-square)](https://github.com/Coucoudb/Marianne/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> Assistant IA souverain, 100% local.
> Vos données ne quittent jamais votre contrôle.

## Présentation

**Marianne AI** est une intelligence artificielle locale qui aide les citoyens français à :

- 📄 Comprendre un courrier administratif en langage clair
- ⚖️ Connaître leurs droits (travail, CAF, URSSAF, logement, retraite, santé)
- ✍️ Rédiger des lettres de réclamation / contestation
- 🧭 Naviguer dans les démarches administratives
- 📑 Analyser des documents PDF (relevés, courriers officiels)

Le tout **sans cloud obligatoire**, sans compte distant, sans que vos données personnelles ne soient envoyées à des tiers.

## Fonctionnalités

- **LLM 100% Local** — Inférence rapide via llama.cpp (accélération GPU CUDA/Vulkan/Metal). Gestion dynamique des modèles GGUF (téléchargement depuis HuggingFace ou import local).
- **Système Multi-Agents** — Création d'agents spécialisés avec gestion autonome de prompts, d'outils et de délégation de tâches.
- **Outils Agentiques (Function Calling)** — Exécution de commandes terminal, recherche locale, et actions sur les fichiers système.
- **RAG Hybride** — Base vectorielle LanceDB couplée à un graphe de connaissances (petgraph) pour des réponses sourcées sur la loi française.
- **Corpus Juridique Intégré** — Fiches pratiques et textes de loi injectés dynamiquement ("Skills").
- **Recherche Web Souveraine** — Interrogation de sources officielles (service-public.fr, etc.) ou recherche générale avec enrichissement automatique du RAG.
- **Analyse de Fichiers** — Extraction de texte depuis des documents locaux (PDF, TXT, MD).
- **Historique & Profil** — Sauvegarde locale des conversations (SQLite) et profil utilisateur persistant.
- **Architecture Client-Serveur** — Interface Electron légère et backend Rust puissant (séparables sur le réseau local).

## Architecture

Marianne AI a migré vers une architecture moderne **Client-Serveur** :

1. **`marianne-server` (Backend Rust)**
   - Serveur HTTP (Axum) exposant une API REST et du streaming SSE.
   - Héberge le modèle LLM, la base vectorielle, et orchestre le système multi-agents.
2. **`marianne-client` (Frontend Electron)**
   - Application de bureau légère (Svelte 5 + Vite + TypeScript).
   - Se connecte au serveur local ou distant.
   - Fournit l'interface utilisateur, l'accès au système de fichiers local et un terminal intégré.
3. **`marianne-core` (Bibliothèque Rust)**
   - Coeur logique partagé contenant l'intégration llama.cpp, le GraphRAG et les agents.

Cette séparation permet d'héberger le serveur gourmand en ressources sur une machine puissante (NAS, PC Fixe avec GPU) tout en utilisant l'application depuis un ordinateur plus modeste.

## Prérequis

- **Rust** ≥ 1.75 (`rustup`)
- **Node.js** ≥ 18 + npm (pour le client Electron)
- **CMake** ≥ 3.21
- Windows : Visual Studio Build Tools (MSVC)
- *Optionnel* : **CUDA Toolkit** ≥ 12.0 + GPU NVIDIA (pour accélération CUDA)
- *Optionnel* : **Vulkan SDK** (pour accélération GPU universelle)

## Démarrage rapide

L'application nécessite de lancer à la fois le serveur et le client.

### 1. Démarrer le Serveur (Backend)

Ouvrez un terminal et placez-vous à la racine du projet :

```bash
cd marianne/marianne-server

# Mode CPU (par défaut)
cargo run --release

# Ou avec accélération GPU NVIDIA (recommandé si disponible)
cargo run --release --features cuda

# Le serveur écoute par défaut sur http://0.0.0.0:3000
```

### 2. Démarrer le Client (Frontend)

Ouvrez un second terminal à la racine du projet :

```bash
cd marianne/marianne-client

# Installation des dépendances (la première fois)
npm install

# Lancement de l'application de bureau
npm run dev
```

Au premier lancement, le client vous demandera de configurer la connexion au serveur (par défaut `http://127.0.0.1:3000`).

## ⚡ Compilation du serveur avec support GPU

**Par défaut, le serveur est compilé en mode CPU uniquement**. Pour des performances optimales, compilez le serveur avec le support matériel approprié :

### GPU NVIDIA (CUDA) — Performances maximales
```bash
# Prérequis : CUDA Toolkit ≥ 12.0
cd marianne/marianne-server
cargo build --release --features cuda
```

### GPU Universel (Vulkan) — AMD, Intel, NVIDIA
```bash
# Prérequis : Vulkan SDK
cd marianne/marianne-server
cargo build --release --features vulkan
```

### Apple Silicon (Metal) — macOS (M1/M2/M3)
```bash
cd marianne/marianne-server
cargo build --release --features metal
```

## Structure du projet

```text
marianne/
├── marianne-core/       # 🧠 Moteur Rust (LLM, RAG, Agents, Prompts)
├── marianne-server/     # ⚙️ Serveur HTTP (Axum, SSE, API REST)
├── marianne-client/     # 🖥️ App Desktop (Electron, Svelte, Vite)
├── corpus/              # 📚 Base de connaissances juridique par défaut
├── ARCHITECTURE.md      # Détails architecturaux
└── MIGRATION.md         # Guide de migration depuis l'ancienne version Tauri
```

## Contribuer

Pour contribuer au code source, assurez-vous que les tests et la compilation passent correctement sur le core et le server :

```bash
cd marianne
cargo check -p marianne-server
cargo check -p marianne-server --features cuda # Si vous avez CUDA
cargo test --all
```

## Licence

MIT — Projet souverain français, données locales uniquement.