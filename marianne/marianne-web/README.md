# Marianne Web Frontend

Interface utilisateur Svelte pour l'assistant administratif Marianne, avec support Tauri (desktop) et Web (navigateur).

## 🎯 Vue d'ensemble

Frontend moderne basé sur **Svelte 4** et **TypeScript**, utilisant **svelte-spa-router** pour la navigation entre plusieurs pages fonctionnelles. Exploite toutes les API REST du backend `marianne-server`.

## ✨ Fonctionnalités

### Pages Disponibles

- **`/`** - Chat streaming avec RAG, recherche web, et analyse de documents
- **`/history`** - Historique complet des conversations
- **`/profile`** - Édition du profil utilisateur (âge, statut, préférences)
- **`/documents`** - Extraction et analyse de documents PDF/TXT
- **`/models`** - Gestion et téléchargement de modèles LLM
- **`/settings`** - Configuration URL API (mode web)

### Caractéristiques Techniques

✅ **Dual-mode** : Fonctionne en application Tauri (desktop) ET en SPA web  
✅ **Streaming temps réel** : Affichage progressif des réponses via SSE  
✅ **RAG intégré** : Recherche dans le corpus légal français  
✅ **Recherche web** : Trigger automatique si confiance faible  
✅ **Multi-documents** : Analyse simultanée de plusieurs fichiers  
✅ **Type-safe** : TypeScript strict avec types complets de l'API  
✅ **Responsive** : Interface adaptative desktop/mobile  

## 🚀 Démarrage Rapide

### Prérequis
- Node.js 18+
- npm ou pnpm
- Backend `marianne-server` lancé (pour mode web)

### Installation
```bash
npm install
```

### Développement
```bash
npm run dev
# Ouvre http://localhost:5173
```

### Build Production
```bash
npm run build
# Output dans dist/
```

### Vérification TypeScript
```bash
npm run check
```

## 📁 Structure du Projet

```
marianne-web/
├── src/
│   ├── App.svelte              # App racine avec router
│   ├── routes.ts               # Configuration des routes
│   ├── main.ts                 # Point d'entrée
│   ├── app.css                 # Styles globaux
│   │
│   ├── components/             # Composants réutilisables
│   │   ├── Header.svelte       # Header avec navigation
│   │   ├── ChatMessages.svelte # Affichage des messages
│   │   ├── InputArea.svelte    # Zone de saisie
│   │   ├── SetupModal.svelte   # Modal téléchargement modèle
│   │   ├── SettingsPanel.svelte # Panneau settings (Tauri)
│   │   └── WebSettingsPage.svelte # Config API (web)
│   │
│   ├── pages/                  # Pages routées
│   │   ├── ChatPage.svelte     # Chat principal
│   │   ├── HistoryPage.svelte  # Historique
│   │   ├── ProfilePage.svelte  # Profil utilisateur
│   │   ├── DocumentsPage.svelte # Analyse documents
│   │   └── ModelsPage.svelte   # Gestion modèles
│   │
│   └── lib/                    # Utilitaires
│       ├── api.ts              # Config API + détection Tauri
│       ├── backend.ts          # Abstraction IPC/HTTP
│       ├── types.ts            # Types TypeScript
│       ├── markdown.ts         # Rendu Markdown
│       └── sources.ts          # Formattage sources
│
├── IMPLEMENTATION.md           # 📘 Guide d'implémentation complet
├── ROUTING.md                  # 🗺️ Architecture détaillée
├── TAURI_COMMANDS.md           # 🔌 Commandes Tauri à implémenter
├── package.json
├── tsconfig.json
├── vite.config.ts
└── svelte.config.js
```

## 📚 Documentation

### Guides

| Fichier | Description |
|---------|-------------|
| **[IMPLEMENTATION.md](./IMPLEMENTATION.md)** | Guide complet d'utilisation et de test |
| **[ROUTING.md](./ROUTING.md)** | Architecture du système de routing |
| **[TAURI_COMMANDS.md](./TAURI_COMMANDS.md)** | Commandes Tauri backend à implémenter |

### Liens API

- [API REST complète](../../docs/marianne-server-api.md) - Documentation backend

## 🎨 Design

### Charte Graphique Marianne
- **Tricolore** français dans le header (Bleu/Blanc/Rouge)
- **Palette chaude** : Beige (#faf8f5), blanc cassé
- **Accent** : Bleu France (#000091)
- **Typographie** : Marianne (avec fallbacks)
- **Ombres douces** pour la profondeur
- **Transitions fluides** sur les interactions

### Navigation Active
Le bouton de la page courante est mis en évidence dans le header avec un fond bleu et texte blanc.

## 🔧 Mode Tauri vs Web

| Fonctionnalité | Tauri | Web |
|----------------|-------|-----|
| Chat streaming | ✅ | ✅ |
| Historique | ✅ | ✅ |
| Profil utilisateur | ✅ | ✅ |
| Analyse documents | ✅ | ❌* |
| Gestion modèles | ✅ | ✅ |
| Settings | ✅ | ✅ |

*En mode web, le navigateur ne peut pas accéder directement aux fichiers locaux du système de fichiers pour des raisons de sécurité. L'analyse de documents n'est disponible qu'en mode Tauri.

### Configuration Mode Web

1. Lancer le backend : `marianne-server` sur port 3000
2. Ouvrir l'app web : http://localhost:5173
3. Aller sur `/settings`
4. Configurer l'URL : `http://localhost:3000`
5. Tester la connexion et enregistrer

## 🐛 Troubleshooting

### Erreur : Cannot find module 'svelte-spa-router'
```bash
npm install svelte-spa-router
```

### Page blanche au démarrage
1. Ouvrir console (F12)
2. Vérifier erreurs JavaScript
3. Vérifier que le backend est accessible

### Mode web : "Serveur inaccessible"
1. Vérifier que `marianne-server` tourne
2. Configurer l'URL dans `/settings`
3. Tester avec `curl http://localhost:3000/health`

---

**Pour toute question, consulter [IMPLEMENTATION.md](./IMPLEMENTATION.md) ou [ROUTING.md](./ROUTING.md)**
