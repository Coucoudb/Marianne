# 🎉 Frontend Marianne - Système de Routing Complet

## ✅ Implémentation Terminée

Le frontend Svelte a été entièrement restructuré avec **svelte-spa-router** et 5 nouvelles pages fonctionnelles exploitant toutes les API REST du backend.

---

## 🚀 Installation Rapide

```bash
cd marianne/marianne-web
npm install  # Installe svelte-spa-router
npm run dev  # Lance le serveur de développement
```

---

## 🗺️ Navigation

### Pages Disponibles

| Route | Page | Fonctionnalités |
|-------|------|-----------------|
| **`/`** | ChatPage | Chat streaming, RAG, recherche web, documents |
| **`/history`** | HistoryPage | Liste et détail des conversations |
| **`/profile`** | ProfilePage | Édition complète du profil utilisateur |
| **`/documents`** | DocumentsPage | Analyse de documents PDF/TXT/MD |
| **`/models`** | ModelsPage | Gestion et téléchargement de modèles |
| **`/settings`** | WebSettingsPage | Configuration API (mode web) |

Le **Header** inclut maintenant des boutons de navigation entre toutes les pages avec indication de la page active.

---

## 📦 Fichiers Créés/Modifiés

### ✨ Nouveaux Fichiers

```
src/
├── routes.ts                    # Configuration du router
├── pages/
│   ├── ChatPage.svelte         # Page de chat (logique extraite de App.svelte)
│   ├── HistoryPage.svelte      # Historique des conversations
│   ├── ProfilePage.svelte      # Édition du profil
│   ├── DocumentsPage.svelte    # Analyse de documents
│   └── ModelsPage.svelte       # Gestion des modèles
└── ROUTING.md                   # Documentation architecture
```

### 🔧 Fichiers Modifiés

```
package.json          # Ajout de svelte-spa-router
src/lib/types.ts      # Types étendus (UserProfile, ModelsStatus, etc.)
src/lib/backend.ts    # Nouvelles fonctions API (getProfile, getHistory, etc.)
src/components/Header.svelte  # Navigation avec boutons de route
src/App.svelte        # Restructuration avec Router
```

---

## 🎯 Fonctionnalités par Page

### 1️⃣ **ChatPage** (`/`) - Page Principale

**Ce qui fonctionne :**
- ✅ Chat streaming en temps réel via SSE
- ✅ RAG (recherche dans le corpus local)
- ✅ Recherche web automatique si confiance faible
- ✅ Analyse de documents en pièce jointe (Tauri)
- ✅ Affichage des sources juridiques
- ✅ Statistiques (tokens, temps de génération)
- ✅ Gestion des contradictions

**API utilisées :**
- `POST /api/v1/chat` (SSE)
- `POST /api/v1/documents/extract` (si fichiers attachés)

---

### 2️⃣ **HistoryPage** (`/history`) - Historique

**Ce qui fonctionne :**
- ✅ Liste de toutes les conversations (stockées dans localStorage)
- ✅ Clic sur une conversation pour voir les détails
- ✅ Récupération de l'historique complet via API
- ✅ Affichage des messages user/assistant avec timestamps
- ✅ Bouton de suppression de conversation
- ✅ Compteur de messages par conversation

**API utilisées :**
- `GET /api/v1/history/:conversation_id`

**Note :** Actuellement, la liste des conversations utilise localStorage. Pour une solution complète, un endpoint backend `/api/v1/history` (sans ID) serait nécessaire pour lister toutes les conversations.

---

### 3️⃣ **ProfilePage** (`/profile`) - Profil Utilisateur

**Ce qui fonctionne :**
- ✅ Chargement automatique du profil existant
- ✅ Formulaire complet avec tous les champs :
  - Prénom, âge
  - Statut professionnel (9 options)
  - Situation familiale (avec nombre d'enfants si applicable)
  - Département (code postal)
  - Sujets d'intérêt (multi-sélection de 8 thèmes)
  - Niveau de langue (Simple/Standard/Technique)
  - Préférence device (GPU/CPU)
- ✅ Sauvegarde avec feedback succès/erreur
- ✅ Validation côté client

**API utilisées :**
- `GET /api/v1/profile` (au chargement)
- `PUT /api/v1/profile` (à la soumission)

---

### 4️⃣ **DocumentsPage** (`/documents`) - Analyse

**Ce qui fonctionne :**
- ✅ Sélection de fichier via bouton "Parcourir" (Tauri)
- ✅ Saisie manuelle du chemin
- ✅ Question personnalisée sur le document
- ✅ Extraction du contenu avec prévisualisation
- ✅ Génération automatique du prompt
- ✅ Formats supportés : PDF, TXT, MD, JSON
- ⚠️ Mode Tauri uniquement (désactivé en mode web)

**API utilisées :**
- `POST /api/v1/documents/extract`

**Pourquoi Tauri seulement ?**
En mode web, le navigateur ne peut pas accéder directement aux fichiers du système de fichiers local de l'utilisateur pour des raisons de sécurité. En mode Tauri, l'application desktop a accès aux fichiers locaux via l'API native.

---

### 5️⃣ **ModelsPage** (`/models`) - Gestion Avancée

**Ce qui fonctionne :**
- ✅ Affichage du modèle actuellement chargé :
  - Nom, device (GPU/CPU), label device, VRAM
- ✅ Liste de tous les modèles téléchargés :
  - ID, nom, repo HuggingFace, fichier GGUF, taille
- ✅ Formulaire de téléchargement de nouveau modèle :
  - Repo ID HuggingFace
  - Nom du fichier GGUF
  - Nom lisible
- ✅ Bouton "Charger ce modèle" pour switcher
- ✅ Badge "Actif" sur le modèle en cours
- ✅ Bouton "Actualiser" pour rafraîchir l'état

**API utilisées :**
- `GET /api/v1/models/status`
- `POST /api/v1/models/download`
- `POST /api/v1/models/load`

**Exemple d'utilisation :**
1. Cliquer sur "+ Télécharger un modèle"
2. Remplir :
   - Repo: `microsoft/Phi-3.5-mini-instruct-gguf`
   - Fichier: `Phi-3.5-mini-instruct-Q4_K_M.gguf`
   - Nom: `Phi-3.5 Mini`
3. Cliquer sur "Démarrer le téléchargement"
4. Une fois téléchargé, cliquer sur "Charger ce modèle"

---

## 🎨 Design

### Charte Graphique
- **Tricolore** : Bleu/Blanc/Rouge dans le header
- **Palette chaude** : Beige (#faf8f5), blanc cassé
- **Accent** : Bleu France (#000091)
- **Typographie** : Marianne (fallback system fonts)
- **États interactifs** : Hover, active, focus avec transitions douces

### Navigation Active
Le bouton de la page courante dans le header est mis en évidence :
- Fond bleu France
- Texte blanc
- Font weight augmenté

---

## 🔧 Mode Tauri vs Web

### Mode Tauri (Application Desktop)
✅ **Toutes les fonctionnalités disponibles**
- IPC natif via `invoke()`
- Accès aux fichiers locaux
- Pas de configuration API nécessaire
- Performance optimale

### Mode Web (Navigateur)
✅ **Chat, Historique, Profil, Modèles** via HTTP/REST
❌ **Documents désactivé** (pas d'accès aux fichiers locaux du navigateur)
⚙️ **Configuration URL API** dans `/settings` pour pointer vers `marianne-server`

**Configurer l'URL API en mode web :**
1. Aller sur `/settings`
2. Saisir l'URL du serveur (ex: `http://localhost:3000`)
3. Tester la connexion
4. Enregistrer

---

## 📝 Types TypeScript

Tous les types API sont définis dans `src/lib/types.ts` :

```typescript
// Profil utilisateur
UserProfile
ProfessionalStatus
FamilyStatus
LanguageLevel
DevicePreference
GpuSelection

// Documents
ExtractRequest
ExtractedDocument

// Modèles
ModelInfo
LoadedModelInfo
ModelsStatus
DownloadModelRequest

// Historique
ConversationTurn

// Système
SystemInfo
GpuDevice
```

---

## 🧪 Test de l'Implémentation

### 1. Tester le Chat
```
1. Aller sur /
2. Envoyer : "Comment demander le RSA ?"
3. Vérifier le streaming
4. Vérifier les sources affichées
```

### 2. Tester l'Historique
```
1. Aller sur /history
2. Voir la liste des conversations
3. Cliquer sur une conversation
4. Voir les détails
5. Revenir à la liste
```

### 3. Tester le Profil
```
1. Aller sur /profile
2. Remplir le formulaire
3. Cliquer sur "Enregistrer"
4. Vérifier le message de succès
5. Recharger la page → les données doivent être persistées
```

### 4. Tester les Documents (Tauri)
```
1. Aller sur /documents
2. Cliquer sur "Parcourir..."
3. Sélectionner un PDF
4. Saisir une question
5. Cliquer sur "Extraire et analyser"
6. Voir le contenu extrait
```

### 5. Tester les Modèles
```
1. Aller sur /models
2. Voir le modèle actif
3. Voir la liste des modèles téléchargés
4. Cliquer sur "Charger ce modèle" sur un autre modèle
5. Vérifier le changement
```

---

## 🐛 Troubleshooting

### Erreur : "Cannot find module 'svelte-spa-router'"
```bash
npm install svelte-spa-router
```

### Page blanche au démarrage
1. Ouvrir la console (F12)
2. Vérifier les erreurs
3. Vérifier que le serveur backend tourne (`marianne-server`)

### Mode web : "Serveur inaccessible"
1. Lancer `marianne-server` sur le port 3000
2. Aller sur `/settings`
3. Configurer l'URL : `http://localhost:3000`
4. Tester la connexion

### Les événements de streaming ne s'affichent pas
Vérifier que :
1. Le backend envoie les événements SSE correctement
2. La console réseau (F12 > Network) montre un flux actif
3. Le `Content-Type` de la réponse est `text/event-stream`

---

## 📚 Documentation Complète

Voir [ROUTING.md](./ROUTING.md) pour l'architecture détaillée.

---

## 🎯 Prochaines Étapes Possibles

- [ ] Ajouter pagination dans HistoryPage
- [ ] Implémenter recherche/filtrage dans l'historique
- [ ] Export de conversations (JSON, TXT, Markdown)
- [ ] Upload de documents en mode web (multipart/form-data)
- [ ] Notifications temps réel pour téléchargements de modèles
- [ ] Mode sombre / clair
- [ ] Paramètres avancés (température, top_p, max_tokens)
- [ ] Graphique d'utilisation GPU/CPU
- [ ] Cache des profils récents

---

## ✨ Résumé des Changements

✅ **5 pages fonctionnelles** créées de toutes pièces
✅ **Routing complet** avec svelte-spa-router
✅ **Navigation Header** avec indication de page active
✅ **9 nouveaux endpoints API** exploités
✅ **Types TypeScript complets** pour toutes les API
✅ **Design cohérent** avec la charte Marianne
✅ **Mode Tauri ET Web** supportés
✅ **Documentation complète** (ce fichier + ROUTING.md)

**L'interface utilisateur est maintenant pleinement fonctionnelle !** 🚀
