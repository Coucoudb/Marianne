# Frontend Marianne - Architecture avec Routing

## 📋 Vue d'ensemble

Le frontend Marianne a été restructuré avec un système de routing complet basé sur **svelte-spa-router**, offrant une navigation fluide entre plusieurs pages fonctionnelles.

## 🗺️ Structure des Routes

```
/ (ChatPage)           - Interface de chat principal avec streaming
/history              - Historique des conversations passées
/profile              - Édition du profil utilisateur
/documents            - Analyse de documents (PDF, TXT, MD)
/models               - Gestion avancée des modèles LLM
/settings             - Configuration API (mode web uniquement)
```

## 📁 Architecture des Fichiers

```
marianne-web/
├── src/
│   ├── App.svelte           # App racine avec router
│   ├── routes.ts            # Définition des routes
│   ├── components/          # Composants réutilisables
│   │   ├── Header.svelte    # Header avec navigation
│   │   ├── ChatMessages.svelte
│   │   ├── InputArea.svelte
│   │   ├── SetupModal.svelte
│   │   ├── SettingsPanel.svelte
│   │   └── WebSettingsPage.svelte
│   ├── pages/               # Pages routées
│   │   ├── ChatPage.svelte
│   │   ├── HistoryPage.svelte
│   │   ├── ProfilePage.svelte
│   │   ├── DocumentsPage.svelte
│   │   └── ModelsPage.svelte
│   └── lib/
│       ├── api.ts           # Détection Tauri + config URL
│       ├── backend.ts       # Abstraction IPC/HTTP
│       ├── types.ts         # Types TypeScript
│       ├── markdown.ts      # Rendu Markdown
│       └── sources.ts       # Formattage des sources
```

## 🎯 Fonctionnalités par Page

### 1. **ChatPage** (`/`)
✅ Chat streaming en temps réel
✅ Support RAG (Retrieval-Augmented Generation)
✅ Recherche web optionnelle
✅ Analyse de documents en pièce jointe (Tauri)
✅ Affichage des sources et statistiques
✅ Badge de confiance avec recherche web

### 2. **HistoryPage** (`/history`)
✅ Liste des conversations sauvegardées (localStorage)
✅ Récupération de l'historique via API (`GET /api/v1/history/:id`)
✅ Affichage détaillé de chaque conversation
✅ Suppression de conversations
✅ Navigation vers le chat

### 3. **ProfilePage** (`/profile`)
✅ Formulaire complet d'édition du profil
✅ Récupération du profil existant (`GET /api/v1/profile`)
✅ Mise à jour du profil (`PUT /api/v1/profile`)
✅ Gestion des champs complexes (famille, sujets d'intérêt)
✅ Validation côté client
✅ Feedback de succès/erreur

### 4. **DocumentsPage** (`/documents`)
✅ Sélection de fichiers (bouton ou chemin manuel)
✅ Extraction de contenu (`POST /api/v1/documents/extract`)
✅ Affichage du texte extrait
✅ Question personnalisée sur le document
✅ Prompt généré automatiquement
⚠️ Mode Tauri uniquement (le web ne peut pas accéder aux fichiers locaux)

### 5. **ModelsPage** (`/models`)
✅ Liste des modèles téléchargés (`GET /api/v1/models/status`)
✅ Affichage du modèle actif avec détails (device, VRAM)
✅ Téléchargement de nouveaux modèles (`POST /api/v1/models/download`)
✅ Chargement d'un modèle différent (`POST /api/v1/models/load`)
✅ Informations détaillées (taille, repo, fichier)
✅ Badge "Actif" sur le modèle chargé

## 🔌 API Backend Utilisées

| Endpoint | Méthode | Utilisé par | Description |
|----------|---------|-------------|-------------|
| `/health` | GET | Tous | Vérification serveur |
| `/api/v1/chat` | POST (SSE) | ChatPage | Streaming de réponse |
| `/api/v1/history/:id` | GET | HistoryPage | Historique d'une conversation |
| `/api/v1/profile` | GET | ProfilePage | Récupération du profil |
| `/api/v1/profile` | PUT | ProfilePage | Mise à jour du profil |
| `/api/v1/documents/extract` | POST | DocumentsPage | Extraction de document |
| `/api/v1/system/info` | GET | SettingsPanel | Info système (GPU, modèle) |
| `/api/v1/models/status` | GET | ModelsPage | Liste des modèles |
| `/api/v1/models/download` | POST | ModelsPage | Télécharger un modèle |
| `/api/v1/models/load` | POST | ModelsPage | Charger un modèle |

## 🎨 Style CSS

Le design respecte la charte graphique "Marianne" avec :
- Tricolore français dans le header
- Palette chaude (beige, blanc cassé)
- Ombres douces pour la profondeur
- Typographie Marianne
- States interactifs (hover, active, focus)

## 🚀 Développement

### Installer les dépendances
```bash
cd marianne-web
npm install
```

### Démarrer en mode développement
```bash
npm run dev
```

### Build pour production
```bash
npm run build
```

### Vérifier les types TypeScript
```bash
npm run check
```

## 🔧 Mode Tauri vs Web

### Mode Tauri (Desktop)
- Toutes les fonctionnalités disponibles
- IPC natif via `invoke()`
- Accès aux fichiers locaux
- Pas besoin de configurer l'URL API

### Mode Web (Browser)
- Chat fonctionnel via HTTP/SSE
- Historique, profil, modèles via API REST
- ⚠️ Documents désactivé (pas d'accès fichiers locaux)
- Configuration URL API dans `/settings`

## 📝 Types TypeScript

Tous les types de l'API sont définis dans `lib/types.ts` :
- `UserProfile` : Profil utilisateur complet
- `ConversationTurn` : Un tour de conversation
- `ExtractedDocument` : Résultat d'extraction
- `ModelsStatus` : Statut des modèles
- `SystemInfo` : Informations système
- etc.

## 🎯 Prochaines Étapes

- [ ] Ajouter pagination dans HistoryPage pour les grandes listes
- [ ] Implémenter recherche dans l'historique
- [ ] Ajouter export de conversations (JSON, TXT)
- [ ] Upload de documents en mode web (multipart/form-data)
- [ ] Notifications push pour les téléchargements de modèles
- [ ] Mode sombre / clair

## 🐛 Debugging

### Router ne fonctionne pas ?
Vérifiez que `svelte-spa-router` est installé :
```bash
npm install svelte-spa-router
```

### Page blanche ?
Ouvrez la console développeur (F12) pour voir les erreurs. Vérifiez que le serveur backend est accessible.

### Mode web : "Serveur inaccessible" ?
Configurez l'URL API dans `/settings` avec l'adresse correcte du serveur `marianne-server`.
