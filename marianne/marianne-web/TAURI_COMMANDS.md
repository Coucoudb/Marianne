# Commandes Tauri Requises pour le Frontend

Ce document liste toutes les commandes Tauri (`invoke()`) que le backend `src-tauri/` doit implémenter pour supporter pleinement toutes les fonctionnalités du frontend.

---

## ✅ Commandes Déjà Implémentées

Ces commandes sont appelées par le frontend et doivent déjà exister :

| Commande | Fichier Frontend | Description |
|----------|-----------------|-------------|
| `check_model_status` | `backend.ts` | Vérifie si le modèle est téléchargé/chargé |
| `load_model` | `backend.ts` | Charge le modèle en mémoire |
| `initialize_rag` | `backend.ts` | Initialise le système RAG |
| `download_model` | `backend.ts` | Télécharge le modèle par défaut |
| `check_corpus_update` | `backend.ts` | Vérifie si le corpus nécessite une mise à jour |
| `update_corpus` | `backend.ts` | Met à jour le corpus légal |
| `set_device_preference` | `backend.ts` | Change la préférence GPU/CPU |
| `send_message` | `backend.ts` | Envoie un message au LLM (streaming) |
| `stop_generation` | `backend.ts` | Arrête la génération en cours |
| `extract_document` | `backend.ts` | Extrait le texte d'un document local |

---

## 🆕 Commandes à Implémenter

Ces commandes sont appelées par les nouvelles pages mais doivent être ajoutées au backend Tauri :

### 1. **Historique** - `HistoryPage.svelte`

#### `get_history`
```rust
#[tauri::command]
async fn get_history(conversation_id: String) -> Result<Vec<ConversationTurn>, String> {
    // Récupère l'historique d'une conversation depuis la DB SQLite
    // Retourne un Vec de ConversationTurn (role, content, timestamp)
}
```

**Type Rust :**
```rust
#[derive(Serialize, Deserialize)]
struct ConversationTurn {
    role: String,        // "user" ou "assistant"
    content: String,     // Contenu du message
    timestamp: i64,      // Unix timestamp
}
```

**Où implémenter :** `src-tauri/src/history/mod.rs` ou `src-tauri/src/commands/history.rs`

---

### 2. **Profil** - `ProfilePage.svelte`

#### `get_profile`
```rust
#[tauri::command]
async fn get_profile(state: State<'_, AppState>) -> Result<UserProfile, String> {
    // Charge le profil utilisateur depuis le fichier ou la DB
    // Retourne l'objet UserProfile complet
}
```

#### `update_profile`
```rust
#[tauri::command]
async fn update_profile(
    profile: UserProfile, 
    state: State<'_, AppState>
) -> Result<(), String> {
    // Sauvegarde le profil mis à jour
    // Met à jour updated_at avec timestamp actuel
}
```

**Type Rust :**
```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub first_name: String,
    pub age: Option<u8>,
    pub professional_status: Option<ProfessionalStatus>,
    pub family_status: Option<FamilyStatus>,
    pub department: Option<String>,
    pub topics_of_interest: Vec<String>,
    pub language_level: LanguageLevel,
    pub device_preference: DevicePreference,
    pub gpu_selection: GpuSelection,
    pub selected_model: Option<String>,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ProfessionalStatus {
    Salarie,
    ChomeurIndemise,
    ChomeurNonIndemise,
    EtudiantApprentis,
    Retraite,
    Independant,
    FonctionPublique,
    Autre,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum FamilyStatus {
    Simple(String),  // "Celibataire" ou "EnCouple"
    WithChildren {
        #[serde(rename = "Parent")]
        parent: Option<ChildrenCount>,
        #[serde(rename = "ParentIsolé")]
        parent_isole: Option<ChildrenCount>,
        #[serde(rename = "CoupleAvecEnfants")]
        couple_avec_enfants: Option<ChildrenCount>,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChildrenCount {
    pub children_count: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum LanguageLevel {
    Simple,
    Standard,
    Technique,
}
```

**Où implémenter :** `src-tauri/src/profile/mod.rs` ou `src-tauri/src/commands/profile.rs`

---

### 3. **Système** - `SettingsPanel.svelte`

#### `get_system_info`
```rust
#[tauri::command]
async fn get_system_info(state: State<'_, AppState>) -> Result<SystemInfo, String> {
    // Retourne les infos système : device actif, modèle, GPU disponibles
}
```

**Type Rust :**
```rust
#[derive(Serialize, Deserialize)]
pub struct SystemInfo {
    pub device: DeviceInfo,
    pub model: ModelInfo,
    pub preference: UserPreference,
    pub gpu_devices: Vec<GpuDevice>,
}

#[derive(Serialize, Deserialize)]
pub struct DeviceInfo {
    pub backend: String,       // "gpu" ou "cpu"
    pub label: String,          // Ex: "GPU (NVIDIA RTX 3060)"
    pub gpu_available: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,           // Ex: "Phi-3 Mini (Q4)"
    pub active: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserPreference {
    pub device: String,         // "Gpu" ou "Cpu"
    pub gpu_selection: String,  // "Auto", "AllGpus", ou {"Specific": 0}
}

#[derive(Serialize, Deserialize)]
pub struct GpuDevice {
    pub index: usize,
    pub name: String,
    pub device_type: String,    // "gpu", "integrated_gpu", "accelerator"
    pub vram_free_mb: u64,
}
```

**Où implémenter :** `src-tauri/src/llm/device.rs` ou `src-tauri/src/commands/system.rs`

---

### 4. **Modèles** - `ModelsPage.svelte`

#### `get_models_status`
```rust
#[tauri::command]
async fn get_models_status(state: State<'_, AppState>) -> Result<ModelsStatus, String> {
    // Liste tous les modèles téléchargés et le modèle actuellement chargé
}
```

#### `download_new_model`
```rust
#[tauri::command]
async fn download_new_model(
    request: DownloadModelRequest,
    state: State<'_, AppState>
) -> Result<DownloadResponse, String> {
    // Lance le téléchargement d'un nouveau modèle depuis HuggingFace
    // Émet des événements 'download-progress' pendant le téléchargement
}
```

#### `load_model_by_id`
```rust
#[tauri::command]
async fn load_model_by_id(
    model_id: String,
    state: State<'_, AppState>
) -> Result<(), String> {
    // Décharge le modèle actuel et charge le modèle avec l'ID spécifié
    // Émet l'événement 'model-ready' une fois chargé
}
```

**Types Rust :**
```rust
#[derive(Serialize, Deserialize)]
pub struct ModelsStatus {
    pub downloaded_models: Vec<ModelEntry>,
    pub loaded_model: Option<LoadedModel>,
}

#[derive(Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub repo_id: String,
    pub filename: String,
    pub size_mb: u64,
}

#[derive(Serialize, Deserialize)]
pub struct LoadedModel {
    pub id: String,
    pub name: String,
    pub device: String,          // "gpu" ou "cpu"
    pub device_label: String,    // Ex: "GPU (NVIDIA RTX 3060)"
}

#[derive(Serialize, Deserialize)]
pub struct DownloadModelRequest {
    pub repo_id: String,
    pub filename: String,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct DownloadResponse {
    pub status: String,          // "started"
    pub model_id: String,
}
```

**Où implémenter :** `src-tauri/src/llm/model.rs` ou `src-tauri/src/commands/models.rs`

---

## 📋 Checklist d'Implémentation

### Historique
- [ ] Implémenter `get_history(conversation_id: String)`
- [ ] Tester avec une conversation existante
- [ ] Vérifier le format des timestamps (Unix time en secondes)

### Profil
- [ ] Implémenter `get_profile()`
- [ ] Implémenter `update_profile(profile: UserProfile)`
- [ ] Créer le fichier de stockage (`~/.marianne/profile.json`)
- [ ] Gérer la migration si le format change
- [ ] Valider les données avant sauvegarde

### Système
- [ ] Implémenter `get_system_info()`
- [ ] Détecter les GPU disponibles (via `llama_cpp_2` ou système)
- [ ] Calculer la VRAM libre pour chaque GPU
- [ ] Retourner le device actuellement utilisé

### Modèles
- [ ] Implémenter `get_models_status()`
- [ ] Scanner le dossier des modèles (`~/.cache/huggingface/hub/`)
- [ ] Parser les métadonnées des modèles
- [ ] Implémenter `download_new_model(request)`
- [ ] Gérer la progression du téléchargement (événements)
- [ ] Implémenter `load_model_by_id(model_id)`
- [ ] Décharger proprement le modèle actuel avant de charger le nouveau

---

## 🔌 Événements Tauri

Ces événements sont **déjà gérés** par le frontend et doivent être émis par le backend :

| Événement | Quand l'émettre | Payload |
|-----------|-----------------|---------|
| `stream-token` | Pendant la génération | `{ token: String, conversation_id: String }` |
| `generation-done` | Fin de génération | `{ full_response: String, conversation_id: String, time_ms: u64, tokens_generated: u32, sources: Vec<String> }` |
| `download-progress` | Téléchargement modèle | `{ percent: f32, downloaded_mb: f32, total_mb: f32 }` |
| `model-ready` | Modèle chargé | `{}` |
| `confidence-info` | Score de confiance | `{ score: f32, level: String, explanation: String, web_search_triggered: bool }` |
| `web-search-status` | Recherche web terminée | `{ status: String, sources_count: usize }` |
| `offline-mode` | Pas d'Internet | `{ message: String }` |
| `contradiction-warning` | Contradiction détectée | `{ message: String }` |
| `corpus-update-status` | Corpus mis à jour | `{ status: String, updated: usize }` |

---

## 🧪 Tests Recommandés

### Pour chaque commande :
1. **Test unitaire** : Vérifier le fonctionnement isolé
2. **Test d'intégration** : Appeler depuis le frontend
3. **Test d'erreur** : Vérifier la gestion des erreurs
4. **Test de sérialisation** : Vérifier que les types Rust → JSON → TypeScript fonctionnent

### Exemple de test :
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_profile() {
        let state = create_test_state();
        let profile = get_profile(state).await.unwrap();
        assert_eq!(profile.first_name, "Test");
    }
}
```

---

## 📚 Documentation Associée

- [Frontend IMPLEMENTATION.md](./IMPLEMENTATION.md) - Guide d'utilisation du frontend
- [Frontend ROUTING.md](./ROUTING.md) - Architecture détaillée
- [Backend marianne-server-api.md](../docs/marianne-server-api.md) - API REST complète

---

## 🎯 Priorisation

### Priorité Haute (fonctionnalités critiques)
1. ✅ `extract_document` (déjà fait)
2. 🆕 `get_profile` / `update_profile` (profil utilisateur)
3. 🆕 `get_models_status` (affichage modèle actif)

### Priorité Moyenne
4. 🆕 `get_history` (historique conversations)
5. 🆕 `get_system_info` (infos GPU dans settings)

### Priorité Basse (fonctionnalités avancées)
6. 🆕 `download_new_model` (téléchargement modèles additionnels)
7. 🆕 `load_model_by_id` (switch entre modèles)

---

## ✅ Résumé

**7 nouvelles commandes Tauri** à implémenter pour support complet du frontend :

1. `get_history` - Historique d'une conversation
2. `get_profile` - Récupération du profil
3. `update_profile` - Mise à jour du profil
4. `get_system_info` - Infos système (GPU, device)
5. `get_models_status` - Liste des modèles
6. `download_new_model` - Télécharger un modèle
7. `load_model_by_id` - Charger un modèle différent

Une fois implémentées, **toutes les pages du frontend seront pleinement fonctionnelles** en mode Tauri ! 🚀
