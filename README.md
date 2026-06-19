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

## Démarrage rapide

Téléchargez la [dernière release](https://github.com/Coucoudb/Marianne/releases/latest) et suivez les étapes ci-dessous.

### 1. Installer le serveur IA

Le serveur fait tourner l'IA localement. Choisissez l'archive adaptée à votre matériel :

> 🔑 **Premier démarrage** : définissez une clé admin pour sécuriser l'API :
> ```bash
> # Linux / macOS
> export MARIANNE_BOOTSTRAP_ADMIN_KEY=mk_$(cat /proc/sys/kernel/random/uuid | tr -d '-')
> # Windows (PowerShell)
> $env:MARIANNE_BOOTSTRAP_ADMIN_KEY = "mk_" + [guid]::NewGuid().ToString('N')
> ```
> La clé est insérée une seule fois au démarrage si aucune clé n'existe. Conservez-la pour configurer le client.

| Votre GPU | Windows | Linux | macOS |
|-----------|---------|-------|-------|
| **NVIDIA RTX/GTX** | `marianne-server-windows-x64-cuda.zip` | `marianne-server-linux-x64-cuda.tar.gz` | — |
| **AMD / Intel / Autre** | `marianne-server-windows-x64-vulkan.zip` | `marianne-server-linux-x64-vulkan.tar.gz` | — |
| **Apple Silicon** | — | — | `marianne-server-macos-arm64-metal.tar.gz` |
| **Pas de GPU** | `marianne-server-windows-x64-cpu.zip` | `marianne-server-linux-x64-cpu.tar.gz` | `marianne-server-macos-arm64-cpu.tar.gz` |

Décompressez l'archive puis lancez le serveur :

```bash
# Windows
marianne-server.exe

# Linux / macOS
./marianne-server
```

> 💡 Au premier lancement, le modèle IA (~2.2 Go) est téléchargé automatiquement.

### 2. Installer le client

Le client est l'application de bureau que vous utilisez au quotidien.

| Système | Fichier | Description |
|---------|---------|-------------|
| **Windows** | `Marianne AI Setup X.X.X.exe` | Installateur (recommandé) |
| **Windows** | `Marianne AI X.X.X.exe` | Portable (sans installation) |
| **macOS** | `Marianne AI-X.X.X-arm64.dmg` | Installateur |
| **Linux** | `Marianne AI-X.X.X.AppImage` | Portable (universel) |
| **Linux** | `marianne-client_X.X.X_amd64.deb` | Paquet Debian/Ubuntu |

Au premier lancement, configurez l'URL du serveur (par défaut `http://localhost:3000`).

### 3. C'est prêt !

Le serveur et le client peuvent tourner sur le même PC, ou séparément : le serveur sur une machine puissante (NAS, PC fixe avec GPU) et le client sur un ordinateur plus modeste.

---

## Développement

### Prérequis

- **Rust** ≥ 1.75 (`rustup`)
- **Node.js** ≥ 18 + npm (pour le client Electron)
- **CMake** ≥ 3.21
- Windows : Visual Studio Build Tools (MSVC)
- *Optionnel* : **CUDA Toolkit** ≥ 12.0 (accélération NVIDIA)
- *Optionnel* : **Vulkan SDK** (accélération GPU universelle)

### Lancer depuis les sources

```bash
# Terminal 1 — Serveur
cd marianne/marianne-server
cargo run --release                  # Mode CPU
cargo run --release --features cuda  # Avec GPU NVIDIA
cargo run --release --features vulkan # Avec GPU universel

# Terminal 2 — Client
cd marianne/marianne-client
npm install
npm run dev
```

### Compilation GPU du serveur

```bash
cd marianne/marianne-server

# NVIDIA (CUDA) — performances maximales
cargo build --release --features cuda

# AMD / Intel / NVIDIA (Vulkan) — universel
cargo build --release --features vulkan

# Apple Silicon (Metal)
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