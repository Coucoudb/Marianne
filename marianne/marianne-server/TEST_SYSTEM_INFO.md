# Test de l'endpoint /api/v1/system/info

## Description
Cet endpoint expose les informations système et du modèle LLM pour le panneau des paramètres en mode web.

## Endpoint
```
GET /api/v1/system/info
```

## Exemple de réponse

### Cas 1: GPU disponible, modèle chargé
```json
{
  "device": {
    "backend": "gpu",
    "label": "GPU (NVIDIA GeForce RTX 3060)",
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

### Cas 2: Mode CPU, pas de GPU
```json
{
  "device": {
    "backend": "cpu",
    "label": "CPU (11 threads)",
    "gpu_available": false
  },
  "model": {
    "name": "Phi-3 Mini (Q4)",
    "active": true
  },
  "preference": {
    "device": "Cpu",
    "gpu_selection": "Auto"
  },
  "gpu_devices": []
}
```

### Cas 3: Multi-GPU avec sélection spécifique
```json
{
  "device": {
    "backend": "gpu",
    "label": "GPU (NVIDIA RTX 4090)",
    "gpu_available": true
  },
  "model": {
    "name": "Phi-3.5 Mini (Q4)",
    "active": true
  },
  "preference": {
    "device": "Gpu",
    "gpu_selection": {
      "Specific": 1
    }
  },
  "gpu_devices": [
    {
      "index": 0,
      "name": "NVIDIA RTX 3060",
      "device_type": "gpu",
      "vram_free_mb": 12288
    },
    {
      "index": 1,
      "name": "NVIDIA RTX 4090",
      "device_type": "gpu",
      "vram_free_mb": 24576
    }
  ]
}
```

## Test manuel

### 1. Démarrer le serveur
```bash
cd marianne/marianne-server
cargo run --release
```

Le serveur démarre par défaut sur `http://127.0.0.1:3000`

### 2. Tester l'endpoint
```bash
curl http://127.0.0.1:3000/api/v1/system/info
```

Ou avec jq pour un affichage formaté :
```bash
curl -s http://127.0.0.1:3000/api/v1/system/info | jq .
```

### 3. Test avec le frontend
Dans `marianne-web`, modifier le code pour appeler cet endpoint :
```typescript
const response = await fetch('http://localhost:3000/api/v1/system/info');
const systemInfo = await response.json();
console.log(systemInfo);
```

## Structure des données

### DevicePreference (enum)
- `"Gpu"` : Utiliser le GPU si disponible (défaut)
- `"Cpu"` : Forcer le mode CPU

### GpuSelection (enum)
- `"Auto"` : Sélection automatique du premier GPU (défaut)
- `{"Specific": n}` : Utiliser le GPU à l'index n
- `"AllGpus"` : Répartir le modèle sur tous les GPU disponibles

### device_type
- `"gpu"` : GPU dédié
- `"integrated_gpu"` : GPU intégré (iGPU)
- `"accelerator"` : Accélérateur (TPU, NPU, etc.)

## Notes d'implémentation

### Réutilisation du code Tauri
Les fonctions suivantes de `src-tauri/src/commands/setup.rs` ont été adaptées :
- `is_gpu_available()` : Détection GPU via llama_cpp_2
- `list_gpu_devices()` : Énumération des GPU avec VRAM
- `get_device_info()` : Backend actuel et label
- `resolve_model_name()` : Nom lisible du modèle depuis le registre

### Dépendances
- `llama_cpp_2` : Déjà présent dans marianne-core
- `std::thread::available_parallelism()` : Stdlib (pas besoin de num_cpus)

### Modèle non chargé
Si le modèle n'est pas chargé en mémoire (`model.active = false`), le serveur retourne les informations basées sur les préférences utilisateur, mais le backend peut ne pas refléter la réalité tant que le modèle n'est pas initialisé.
