# Build Marianne Server

Guide pour compiler le serveur HTTP Rust avec accélération GPU.

## Pré-requis

- Rust 1.75+ (`rustup`)
- Protobuf compiler
- CMake 3.20+
- **Optionnel** : CUDA Toolkit 12.6 (GPU NVIDIA)
- **Optionnel** : Vulkan SDK (GPU universel)

### Installation des dépendances

#### Windows

```powershell
# Protobuf
choco install protoc

# CMake
choco install cmake

# CUDA Toolkit (optionnel, pour GPU NVIDIA)
# Télécharger depuis https://developer.nvidia.com/cuda-downloads

# Vulkan SDK (optionnel)
# Télécharger depuis https://vulkan.lunarg.com/
```

#### Linux (Ubuntu/Debian)

```bash
sudo apt-get update
sudo apt-get install -y \
  protobuf-compiler \
  libprotobuf-dev \
  cmake \
  libssl-dev

# CUDA (optionnel, pour GPU NVIDIA)
# Suivre https://developer.nvidia.com/cuda-downloads

# Vulkan (optionnel)
sudo apt-get install -y libvulkan-dev vulkan-tools glslc
```

#### macOS

```bash
brew install protobuf cmake

# Metal est intégré dans macOS (pas d'installation requise)
```

## Build

### Variantes disponibles

| Variante | Features Cargo | Accélération | Usage |
|----------|---------------|--------------|-------|
| **CPU** | `fastembed,vectordb` | ❌ CPU uniquement | Pas de GPU ou tests |
| **CUDA** | `fastembed,vectordb,cuda` | ✅ GPU NVIDIA | RTX/GTX (meilleure perf) |
| **Vulkan** | `fastembed,vectordb,vulkan` | ✅ GPU universel | AMD, Intel, NVIDIA |
| **Metal** | `fastembed,vectordb,metal` | ✅ GPU Apple | macOS ARM64 |

### Build CPU uniquement

```bash
cd marianne
cargo build --release -p marianne-server \
  --no-default-features \
  --features fastembed,vectordb
```

### Build CUDA (NVIDIA)

Pré-requis : CUDA Toolkit 12.6 installé

```bash
cd marianne
cargo build --release -p marianne-server \
  --no-default-features \
  --features fastembed,vectordb,cuda
```

### Build Vulkan (universel)

Pré-requis : Vulkan SDK installé

```bash
cd marianne
cargo build --release -p marianne-server \
  --no-default-features \
  --features fastembed,vectordb,vulkan
```

### Build Metal (macOS)

```bash
cd marianne
cargo build --release -p marianne-server \
  --no-default-features \
  --features fastembed,vectordb,metal \
  --target aarch64-apple-darwin
```

## Binaires compilés

Le binaire est généré dans :
- **Sans target spécifique** : `marianne/target/release/marianne-server` (ou `.exe`)
- **Avec target** : `marianne/target/<target>/release/marianne-server`

## Lancement

```bash
# Serveur local (127.0.0.1:3000)
./marianne-server

# Exposer sur le réseau
./marianne-server --bind 0.0.0.0:3000

# Spécifier le répertoire de données
./marianne-server --data-dir /var/lib/marianne

# Aide complète
./marianne-server --help
```

## Structure du projet

```
marianne/
├── marianne-server/     # Binary crate (serveur HTTP)
│   ├── src/
│   │   ├── main.rs      # Entry point
│   │   ├── routes/      # Axum routes (chat, documents, history...)
│   │   └── state.rs     # Shared app state
│   └── Cargo.toml
├── marianne-core/       # Library crate (logique métier)
│   ├── src/
│   │   ├── llm/         # LLM engine (llama.cpp)
│   │   ├── rag/         # RAG (embeddings, retrieval, graph)
│   │   ├── web/         # Web scraping sources officielles
│   │   ├── history/     # SQLite conversation history
│   │   └── documents/   # PDF/TXT extraction
│   └── Cargo.toml
└── Cargo.toml           # Workspace
```

## Features Cargo

### Principales features

- `fastembed` : Embeddings avec FastEmbed
- `vectordb` : LanceDB pour recherche vectorielle
- `cuda` : Accélération GPU NVIDIA
- `vulkan` : Accélération GPU universelle (Vulkan)
- `metal` : Accélération GPU Apple (macOS)

### Features dépendantes

- **CUDA** implique CUDA Toolkit 12.6+
- **Vulkan** implique Vulkan SDK 1.3+
- **Metal** uniquement sur macOS ARM64

## Optimisations

### Profile release optimisé

Le `Cargo.toml` racine contient déjà :

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
```

### Build avec optimisations natives

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release ...
```

## Dépannage

### Erreur "CUDA Toolkit not found"

Vérifiez que `CUDA_PATH` est défini :

```bash
# Linux/macOS
export CUDA_PATH=/usr/local/cuda

# Windows
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6"
```

### Erreur "protobuf compiler not found"

Installez `protoc` :
- Windows: `choco install protoc`
- Linux: `sudo apt-get install protobuf-compiler`
- macOS: `brew install protobuf`

### Build très lent sur Windows

Utilisez un chemin court pour éviter les limites de 260 caractères :

```powershell
$env:CARGO_TARGET_DIR = "D:\cb"
cargo build --release ...
```

### Erreur de linking Vulkan

Vérifiez que le Vulkan SDK est dans le PATH :

```bash
# Linux
export VULKAN_SDK=/usr
export LD_LIBRARY_PATH=$VULKAN_SDK/lib:$LD_LIBRARY_PATH

# Windows
$env:VULKAN_SDK = "C:\VulkanSDK\1.4.309.0"
```

## CI/CD

Le workflow GitHub Actions `.github/workflows/build-server.yml` :
- Build multi-plateforme (Windows, Linux, macOS)
- Build multi-variante (CPU, CUDA, Vulkan, Metal)
- Installation automatique CUDA Toolkit et Vulkan SDK
- Upload des artéfacts

Pour déclencher un build manuellement :

```bash
gh workflow run build-server.yml
```

## Tests

```bash
# Tests unitaires
cargo test -p marianne-core

# Tests d'intégration serveur
cargo test -p marianne-server

# Tests avec features spécifiques
cargo test --features cuda
```

## Benchmarks

```bash
cargo bench -p marianne-core
```
