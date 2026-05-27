# marianne-web — Frontend Svelte (Tauri)

Frontend Svelte 4 + Vite de l'application desktop Tauri **Marianne**. Il remplace l'ancien frontend vanilla JS (`frontend/`) et communique avec le backend Rust via l'IPC Tauri.

---

## Commandes

```powershell
# Installer les dépendances (première fois ou après changement de package.json)
cd marianne/marianne-web
npm install

# Build de production (génère marianne-web/dist/)
npm run build

# Serveur de développement (http://localhost:1420)
npm run dev

# Vérification TypeScript sans build
npm run check
```

> **Note Tauri :** lancer `cargo tauri dev` depuis `marianne/src-tauri/` exécute automatiquement `npm run dev` (via `beforeDevCommand`) et charge l'UI depuis `http://localhost:1420`.
> De même, `cargo tauri build` lance `npm run build` avant de packager.

---

## Structure des fichiers

```
marianne-web/
├── index.html                  ← point d'entrée HTML (chargé par Vite)
├── package.json                ← dépendances npm
├── vite.config.ts              ← config Vite (port 1420, target chrome105)
├── svelte.config.js            ← config Svelte (vitePreprocess pour TypeScript)
├── tsconfig.json               ← config TypeScript (mode bundler, strict: false)
└── src/
    ├── main.ts                 ← bootstrap : mount App.svelte sur #app
    ├── app.css                 ← styles globaux (variables CSS, tous les composants)
    ├── App.svelte              ← racine : état global + listeners Tauri + logique métier
    ├── lib/
    │   ├── types.ts            ← interfaces TypeScript partagées
    │   ├── markdown.ts         ← parseMarkdown() avec sanitization XSS
    │   └── sources.ts          ← formatSourceLabel() pour les chips de sources
    └── components/
        ├── Header.svelte       ← barre en-tête, indicateur de statut, bouton paramètres
        ├── SettingsPanel.svelte ← panneau paramètres (auto-géré, monté à l'ouverture)
        ├── ChatMessages.svelte  ← liste des messages, markdown, sources, drag & drop
        ├── InputArea.svelte     ← textarea, bouton envoi/stop, gestion des fichiers stagés
        └── SetupModal.svelte    ← modal de premier démarrage (téléchargement du modèle)
```

---

## Architecture des composants

### Flux de données

```
App.svelte  (état global, listeners Tauri)
│
├─ props ──► Header.svelte
│               └─ SettingsPanel.svelte  (self-contained, invoke direct)
│
├─ props/events ──► ChatMessages.svelte
├─ props/events ──► InputArea.svelte
└─ props/events ──► SetupModal.svelte    (conditionnel)
```

### Responsabilités par composant

| Composant | Responsabilité |
|---|---|
| **App.svelte** | État global, setup des listeners `listen()`, pipeline chat, gestion documents, `invoke` critiques |
| **Header.svelte** | En-tête, toggle paramètres, fermeture au clic extérieur |
| **SettingsPanel.svelte** | Device/GPU/modèles/HF search — appels `invoke` internes, monté frais à chaque ouverture |
| **ChatMessages.svelte** | Rendu messages, markdown, indicateurs web, chips sources, drag & drop |
| **InputArea.svelte** | Textarea auto-resize, send/stop, staging de fichiers |
| **SetupModal.svelte** | Modal premier lancement, barre de progression téléchargement |

### Cycle d'un message

```
[InputArea] dispatch('send')
    │
    ▼
[App.svelte] sendMessage() ou sendWithDocuments()
    │  invoke('send_message')
    ▼
[Rust/marianne-core] process_chat()
    │  émet ChatEvent::StreamToken, ChatEvent::GenerationDone, ...
    ▼
[App.svelte] listen('stream-token') → updateMsg(streamingId, { content: tokenBuffer })
             listen('generation-done') → updateMsg(id, { sources, stats, streaming: false })
    │
    ▼
[ChatMessages.svelte] rendu réactif via {#each msgs}
```

---

## Types partagés (`src/lib/types.ts`)

```ts
type StatusType = 'loading' | 'ready' | 'error'

interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  thinking?: boolean    // spinner "Marianne réfléchit..."
  analyzing?: boolean   // spinner "analyse le(s) document(s)..."
  streaming?: boolean   // token stream en cours
  webBadge?: WebBadge
  contradictionWarning?: string
  sources?: string[]
  stats?: { time_ms: number; tokens_generated: number }
}

interface WebBadge {
  text: string
  kind: 'searching' | 'done' | 'empty' | 'offline'
}

interface DownloadProgress {
  percent: number
  downloaded_mb: number
  total_mb: number
}
```

---

## Événements Tauri écoutés

| Événement | Payload | Traitement |
|---|---|---|
| `stream-token` | `{ token, conversation_id }` | Accumule dans `tokenBuffer`, met à jour le message en cours |
| `generation-done` | `{ full_response, sources, tokens_generated, time_ms }` | Finalise le message, affiche sources et stats |
| `download-progress` | `{ percent, downloaded_mb, total_mb }` | Met à jour `downloadPct` (passé à SetupModal et SettingsPanel) |
| `model-ready` | — | Passe `modelLoaded = true`, masque la modal, incrémente `refreshTick` |
| `confidence-info` | `{ score, web_search_triggered }` | Affiche le badge de recherche web si déclenchée |
| `web-search-status` | `{ status, sources_count }` | Met à jour le badge (done / empty) |
| `offline-mode` | `{ message }` | Badge "hors-ligne" |
| `contradiction-warning` | `{ message }` | Badge d'avertissement de contradiction web/corpus |
| `corpus-update-status` | `{ status, updated }` | Toast de mise à jour si `updated > 0` |

---

## Commandes Tauri invoquées

### App.svelte (logique critique)

| Commande | Arguments | Retour |
|---|---|---|
| `check_model_status` | — | `{ model_downloaded, model_loaded }` |
| `load_model` | — | — |
| `initialize_rag` | — | — |
| `send_message` | `{ request: { message, conversation_id?, max_tokens } }` | `string` (conversation_id) |
| `stop_generation` | — | — |
| `download_model` | — | — |
| `set_device_preference` | `{ preference: 'Gpu' \| 'Cpu' }` | — |
| `extract_document` | `{ request: { file_path, question } }` | `{ file_name, text }` |
| `check_corpus_update` | — | `boolean` |
| `update_corpus` | — | — |

### SettingsPanel.svelte (paramètres)

| Commande | Arguments | Retour |
|---|---|---|
| `get_device_info` | — | `{ label }` |
| `get_device_preference` | — | `{ preference, gpu_available }` |
| `set_device_preference` | `{ preference }` | — |
| `list_gpu_devices` | — | `{ devices: [...], selection }` |
| `set_gpu_selection` | `{ selection }` | — |
| `list_installed_models` | — | `InstalledEntry[]` |
| `select_model` | `{ modelId }` | — |
| `delete_model` | `{ modelId }` | — |
| `search_huggingface` | `{ query }` | `HfResult[]` |
| `get_model_gguf_files` | `{ repoId }` | `GgufFile[]` |
| `download_hf_model` | `{ repoId, filename, name }` | — |

---

## Plugins Tauri utilisés

| Plugin npm | Plugin Rust | Usage |
|---|---|---|
| `@tauri-apps/api` | intégré | `invoke`, `listen` |
| `@tauri-apps/plugin-dialog` | `tauri-plugin-dialog` | Sélecteur de fichiers (PDF/TXT/MD) |
| `@tauri-apps/plugin-shell` | `tauri-plugin-shell` | Ouvrir les sources URL dans le navigateur système |

Les permissions sont déclarées dans `src-tauri/capabilities/default.json` :
```json
"shell:allow-open", "dialog:default", "fs:default"
```

---

## Règles et conventions

### Sécurité

- Tout le contenu Markdown affiché passe par `parseMarkdown()` qui appelle `sanitizeHtml()` : supprime `<script>`, `<iframe>`, attributs `on*`, liens `javascript:`.
- Ne jamais afficher `{@html content}` sans passer par `parseMarkdown()`.
- Les messages utilisateur sont affichés en texte brut (`{msg.content}`, non parsé).

### État global

- Tout l'état réactif qui traverse plusieurs composants vit dans `App.svelte` (variables `let`).
- `SettingsPanel.svelte` est **self-contained** : il gère son propre état interne et appelle directement `invoke`. Il est remonté à chaque ouverture du panneau (`{#if showSettings}` dans Header), garantissant des données fraîches.
- Ne pas utiliser de store Svelte externe — la réactivité Svelte native (`let` + `$:`) suffit.

### Événements entre composants

- Les composants enfants remontent les actions via `createEventDispatcher`.
- `App.svelte` ne passe jamais de callbacks directs comme props ; il écoute les events Svelte (`on:send`, `on:drop`, etc.).

### CSS

- Tous les styles sont dans `src/app.css` (global). Les composants `.svelte` n'ont **pas** de bloc `<style>` local.
- Variables CSS déclarées dans `:root` (préfixe `--bleu-france`, `--rouge-marianne`, etc.).
- Classes nommées selon le composant auquel elles appartiennent : `.message`, `.input-area`, `.settings-panel`, etc.

### TypeScript

- `strict: false` dans `tsconfig.json` — pas d'erreurs bloquantes sur les types implicites.
- Les types partagés entre composants sont dans `src/lib/types.ts`.
- Les imports dynamiques (`import('@tauri-apps/plugin-dialog')`) sont utilisés pour les plugins qui ne sont pas toujours nécessaires.

### Markdown (bibliothèque `marked`)

- Version `^12` — `marked.parse(text)` retourne `string` de façon synchrone.
- Ne pas utiliser le mode async.
- Toujours sanitizer après le parse (voir `src/lib/markdown.ts`).

---

## Dépendances

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "marked": "^12"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3",
    "svelte": "^4",
    "svelte-check": "^3",
    "typescript": "^5",
    "vite": "^5"
  }
}
```

---

## Intégration Tauri

Le fichier `src-tauri/tauri.conf.json` est configuré ainsi :

```json
"build": {
  "frontendDist": "../marianne-web/dist",
  "devUrl": "http://localhost:1420",
  "beforeDevCommand": "cd ../marianne-web && npm run dev",
  "beforeBuildCommand": "cd ../marianne-web && npm run build"
}
```

`withGlobalTauri: true` reste actif dans `app` — `window.__TAURI__` est disponible mais non utilisé par le code Svelte (les imports npm sont préférés).
