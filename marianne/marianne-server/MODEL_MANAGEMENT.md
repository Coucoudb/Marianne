# Implémentation : Installation automatique et gestion des modèles

## Résumé

Le serveur `marianne-server` installe et charge désormais **automatiquement** Phi-3 Mini au premier lancement, et expose des endpoints REST pour gérer les modèles LLM.

---

## 1. Installation automatique au démarrage ✅

### Fichiers modifiés

- **`marianne-server/src/main.rs`** : Appel à `ensure_model_ready()` après création de l'état
- **`marianne-core/src/setup.rs`** : **CRÉÉ** — Logique d'installation automatique
- **`marianne-core/src/lib.rs`** : Exposition du module `setup`

### Comportement

Au démarrage du serveur, la séquence suivante s'exécute automatiquement :

1. **Vérification du modèle** — Si Phi-3 Mini n'est pas téléchargé :
   - Téléchargement depuis HuggingFace (~2.2 Go)
   - Reprise automatique en cas d'interruption (HTTP Range)
   - Enregistrement dans `models/registry.json`

2. **Chargement en mémoire** — Si le modèle n'est pas chargé :
   - Chargement du fichier GGUF en RAM/VRAM
   - Configuration GPU/CPU selon le profil utilisateur

3. **Initialisation du RAG** — Si le corpus est vide :
   - Copie des fiches légales depuis `marianne/corpus/`
   - Indexation dans LanceDB (embeddings + FTS)

En cas d'échec, le serveur démarre quand même, et l'utilisateur peut réessayer avec `POST /api/v1/models/setup`.

**Logs typiques :**
```
📥 Premier lancement : téléchargement de Phi-3 Mini (~2.2 Go)...
📥 Phi-3-mini-4k-instruct-q4.gguf : 500 / 2200 Mo (22%)
✅ Phi-3-mini-4k-instruct-q4.gguf téléchargé et enregistré
🔄 Chargement du modèle en mémoire...
✅ Modèle phi-3-mini-q4 chargé en mémoire
📚 Indexation du corpus légal...
✅ RAG initialisé : 248 chunks
✅ Marianne prête !
```

---

## 2. Nouveaux endpoints REST ✅

### Fichiers modifiés

- **`marianne-server/src/routes/models.rs`** : **CRÉÉ** — Handlers pour les modèles
- **`marianne-server/src/routes/mod.rs`** : Enregistrement des routes

### Endpoints disponibles

#### **GET `/api/v1/models/status`**
Retourne la liste des modèles téléchargés et le modèle chargé en mémoire.

**Réponse :**
```json
{
  "downloaded_models": [
    {
      "id": "phi-3-mini-q4",
      "name": "Phi-3 Mini (Q4)",
      "filename": "Phi-3-mini-4k-instruct-q4.gguf",
      "size_mb": 2200,
      "repo_id": "microsoft/Phi-3-mini-4k-instruct-gguf"
    }
  ],
  "loaded_model": {
    "id": "phi-3-mini-q4",
    "name": "Phi-3 Mini (Q4)",
    "device": "gpu"
  }
}
```

---

#### **POST `/api/v1/models/download`**
Télécharge un modèle depuis HuggingFace (en arrière-plan).

**Requête :**
```json
{
  "repo_id": "microsoft/Phi-3.5-mini-instruct-gguf",
  "filename": "Phi-3.5-mini-instruct-Q4_K_M.gguf",
  "name": "Phi-3.5 Mini (Q4_K_M)"
}
```

**Réponse immédiate :**
```json
{
  "status": "downloading",
  "model_id": "microsoft_Phi-3.5-mini-instruct-gguf_Phi-3.5-mini-instruct-Q4_K_M"
}
```

Le téléchargement s'effectue en arrière-plan avec logs dans la console serveur.

---

#### **POST `/api/v1/models/load`**
Charge un modèle téléchargé en mémoire.

**Requête :**
```json
{
  "model_id": "phi-3-mini-q4"
}
```

**Réponse :**
```json
{
  "status": "loaded",
  "model_name": "Phi-3 Mini (Q4)"
}
```

**Effets secondaires :**
- Décharge l'ancien modèle (libère VRAM/RAM)
- Met à jour le profil utilisateur (`selected_model`)
- Charge le nouveau modèle avec les paramètres GPU/CPU du profil

---

#### **POST `/api/v1/models/setup`**
Réexécute l'initialisation complète (download + load + RAG).

Utile si :
- Le téléchargement initial a échoué
- Le modèle a été supprimé manuellement
- Le RAG doit être réindexé

**Réponse :**
```json
{
  "status": "ready",
  "message": "Marianne est prête"
}
```

---

## 3. Logique partagée dans `marianne-core` ✅

### Nouveau module : `marianne-core/src/setup.rs`

Expose les fonctions suivantes (réutilisables par `src-tauri`) :

| Fonction | Description |
|----------|-------------|
| `ensure_model_ready(state)` | Séquence complète : download + load + RAG |
| `download_default_model(data_dir)` | Télécharge Phi-3 Mini depuis HuggingFace |
| `load_model_into_memory(state)` | Charge le modèle sélectionné en RAM |
| `initialize_rag_from_corpus(state)` | Indexe le corpus légal dans LanceDB |
| `download_model_from_huggingface(...)` | Télécharge un modèle arbitraire avec callback |

**Constantes publiques :**
```rust
pub const DEFAULT_MODEL_REPO: &str = "microsoft/Phi-3-mini-4k-instruct-gguf";
pub const DEFAULT_MODEL_FILE: &str = "Phi-3-mini-4k-instruct-q4.gguf";
pub const DEFAULT_MODEL_ID: &str = "phi-3-mini-q4";
```

**Avantages :**
- Code partagé entre `marianne-server` et `src-tauri`
- Pas de duplication de logique
- Testable indépendamment

---

## 4. Gestion d'erreur robuste ✅

### Stratégie

- **Au démarrage** : Échec non bloquant → le serveur démarre quand même
- **Dans les handlers** : Erreurs HTTP structurées (`AppError`)

### Exemple d'erreur HTTP

**Requête** : `POST /api/v1/models/load` avec un modèle inexistant

**Réponse** : `404 Not Found`
```json
{
  "error": "Modèle introuvable"
}
```

---

## 5. Compatibilité avec l'app Tauri ✅

### Code existant préservé

- **`src-tauri/src/commands/setup.rs`** : Non modifié, toujours fonctionnel
- Les commandes Tauri (`download_model`, `load_model`, `initialize_rag`) utilisent toujours leur logique actuelle
- Pas de casse de l'app desktop

### Migration future (optionnelle)

Pour éviter la duplication, les commandes Tauri pourraient être réécrites pour appeler `marianne_core::setup::*` directement, mais ce n'est pas urgent.

---

## 6. Tests suggérés

### Test 1 : Premier lancement
```bash
# Supprimer les données existantes
rm -rf ~/.local/share/marianne/models

# Démarrer le serveur
cargo run --bin marianne-server

# Logs attendus :
# 📥 Premier lancement : téléchargement de Phi-3 Mini (~2.2 Go)...
# ✅ Modèle phi-3-mini-q4 chargé en mémoire
# ✅ Marianne prête !
```

### Test 2 : Status des modèles
```bash
curl http://localhost:3000/api/v1/models/status | jq
```

### Test 3 : Charger un autre modèle
```bash
# 1. Télécharger un modèle (en arrière-plan)
curl -X POST http://localhost:3000/api/v1/models/download \
  -H "Content-Type: application/json" \
  -d '{
    "repo_id": "microsoft/Phi-3.5-mini-instruct-gguf",
    "filename": "Phi-3.5-mini-instruct-Q4_K_M.gguf",
    "name": "Phi-3.5 Mini"
  }'

# 2. Attendre la fin du download (voir logs serveur)

# 3. Charger le modèle en mémoire
curl -X POST http://localhost:3000/api/v1/models/load \
  -H "Content-Type: application/json" \
  -d '{
    "model_id": "microsoft_Phi-3.5-mini-instruct-gguf_Phi-3.5-mini-instruct-Q4_K_M"
  }'
```

### Test 4 : Réinitialisation complète
```bash
curl -X POST http://localhost:3000/api/v1/models/setup
```

---

## 7. Configuration GPU / Troubleshooting

### ⚠️ Problème : "Mode CPU" malgré un GPU présent

**Symptôme :**
```
💻 Aucun backend GPU détecté par llama.cpp
⚠️ Matériel GPU détecté (GPU NVIDIA détecté: NVIDIA GeForce RTX 4070) mais llama.cpp ne le voit pas.
   Cause probable : llama.cpp compilé sans support GPU (cuda/vulkan).
```

**Cause :**  
Par défaut, `llama.cpp` est compilé **sans support GPU**. Les features `cuda` et `vulkan` existent mais ne sont pas dans `default`.

**Solution :**

#### GPU NVIDIA (CUDA) — Performances optimales
```bash
# Prérequis : CUDA Toolkit ≥ 12.0 + pilote NVIDIA récent
cargo build -p marianne-server --release --features cuda

# Lancer le serveur
./target/release/marianne-server --bind 0.0.0.0:3000
```

#### GPU Universel (Vulkan) — Compatible AMD, NVIDIA, Intel
```bash
# Prérequis : Vulkan SDK + pilotes à jour
cargo build -p marianne-server --release --features vulkan

# Lancer le serveur
./target/release/marianne-server --bind 0.0.0.0:3000
```

### Vérification du support GPU

```bash
# Logs au démarrage :
🎮 GPU détecté : NVIDIA GeForce RTX 4070 (Gpu, 7826 Mo VRAM)
🚀 GPU disponible — offloading 999 couches
✅ Modèle phi-3-mini-q4 chargé (2145 Mo, GPU) — vocab: 32064 tokens
```

### Recommandations par matériel

| Matériel | Feature recommandée | Commande |
|----------|---------------------|----------|
| **RTX 3060/4070/4090** | `cuda` | `cargo build --features cuda` |
| **AMD RX 6000/7000** | `vulkan` | `cargo build --features vulkan` |
| **Intel Arc A770** | `vulkan` | `cargo build --features vulkan` |
| **GPU intégré (iGPU)** | `vulkan` | `cargo build --features vulkan` |
| **CPU uniquement** | aucune | `cargo build` (par défaut) |

### Diagnostic automatique

Le moteur LLM détecte automatiquement la présence de GPU au niveau système (via `wmic` sur Windows, `lspci` sur Linux) et affiche un avertissement si :
- Un GPU est présent dans le système
- Mais llama.cpp ne le voit pas (liste vide)

**Message d'avertissement typique :**
```
⚠️ Matériel GPU détecté (GPU NVIDIA détecté: NVIDIA GeForce RTX 4070) mais llama.cpp ne le voit pas.
   Cause probable : llama.cpp compilé sans support GPU (cuda/vulkan).
   Solution :
   • GPU NVIDIA RTX → Recompilez avec: cargo build --release --features cuda
   • GPU AMD/Intel/Autre → Recompilez avec: cargo build --release --features vulkan
```

---

## 8. Améliorations futures (hors scope)

### Progression du téléchargement en temps réel
Actuellement, `POST /models/download` retourne immédiatement et le téléchargement se fait en arrière-plan. Pour afficher la progression :

**Option A : Server-Sent Events (SSE)**
```rust
// GET /api/v1/models/download/progress?model_id=...
pub async fn download_progress_stream() -> Sse<...> {
    // Stream de DownloadProgress
}
```

**Option B : Polling**
```rust
// GET /api/v1/models/download/status?model_id=...
pub async fn download_status() -> Json<DownloadStatus> {
    // Retourne { percent, downloaded_mb, total_mb }
}
```

### Suppression de modèle
```rust
// DELETE /api/v1/models/:model_id
pub async fn delete_model(Path(id): Path<String>) -> Result<...> {
    // Supprimer le fichier GGUF et l'entrée du registre
}
```

---

## Fichiers créés

- `marianne-core/src/setup.rs` (465 lignes)
- `marianne-server/src/routes/models.rs` (200 lignes)

## Fichiers modifiés

- `marianne-core/src/lib.rs` (+1 ligne)
- `marianne-server/src/main.rs` (+7 lignes)
- `marianne-server/src/routes/mod.rs` (+5 lignes)

## Total

**~670 lignes de code ajoutées**, zéro casse de code existant.

---

## Résultat final

✅ **Installation automatique** : Phi-3 Mini téléchargé et chargé au premier lancement  
✅ **Chargement automatique** : Modèle en mémoire dès le démarrage  
✅ **Endpoints REST** : Gestion complète des modèles via HTTP  
✅ **Logique réutilisable** : Code partagé dans `marianne-core`  
✅ **Gestion d'erreur** : Robuste, non bloquante  
✅ **Compatible Tauri** : Aucune casse de l'app desktop  

Le serveur est maintenant **autonome** et **prêt à l'emploi** dès le premier lancement ! 🚀
