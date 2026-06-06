# Marianne Client (`marianne-client`)

Le `marianne-client` est l'application frontend Desktop. C'est l'interface visuelle avec laquelle l'utilisateur final interagit. Elle a été construite pour être rapide, réactive, et facilement extensible, communiquant directement avec `marianne-server` via API REST et flux SSE (Server-Sent Events).

## Technologie

- **Framework Web** : [Svelte 5](https://svelte.dev/)
- **Langage** : TypeScript
- **Build Tool** : Vite
- **Embarquement** : Tauri (optionnel) ou Navigateur Web Classique (Electron possible)
- **Styling** : CSS Vanilla (pour garder le contrôle absolu sur les tokens et le design "Aesthetics" premium)

## Structure des Composants (`src/renderer/`)

Le layout principal se trouve dans `App.svelte` qui pilote la vue entre la page de Chat et la fenêtre des Paramètres (modale ou onglets).

### `components/ChatMessages.svelte`
Gère l'affichage du flux de conversation. Il traite de manière transparente les réponses textuelles du LLM ainsi que l'exécution des outils agentiques (`<tool_call>`). Le parsing du Markdown se fait via `marked.js` couplé à `DOMPurify` pour la sécurité.

### `components/InputArea.svelte`
Composant pour la zone de texte intelligente. Gère les envois via "Entrée", les sauts de ligne ("Shift+Entrée") et intègre un système d'upload de pièces jointes (fichiers PDF) qui seront extraites et ajoutées au contexte.

### `components/AgentsManager.svelte`
Interface d'administration pour la flotte d'agents spécialisés.
Permet de :
- Créer un nouvel agent.
- Éditer son **Prompt Système**.
- Restreindre son périmètre d'action (`working_directory`).
- Activer ou désactiver ses outils dynamiques (ex: `read_file`, `write_file`, `run_command`).
- Lui assigner des "Skills" (bases de connaissances).

### `components/SkillsManager.svelte`
Interface permettant d'ajouter et de modifier des blocs de connaissances statiques (Skills). Ces Skills peuvent ensuite être assignés à divers agents pour enrichir leur prompt d'instructions métier spécifiques.

### Onglet "Modèles" (`App.svelte`)
Interface intégrée dans les paramètres pour télécharger et remplacer le modèle d'IA "à chaud" depuis le hub d'HuggingFace (ex: `Qwen/Qwen2.5-0.5B-Instruct-GGUF`), avec gestion du polling et suppression automatique de l'ancien modèle.

## Communication avec le Serveur

La librairie `src/renderer/lib/api.ts` regroupe l'intégralité des appels REST.
La librairie `src/renderer/lib/chat.ts` gère la connexion asynchrone SSE pour afficher les tokens d'IA au fur et à mesure de leur génération sans bloquer l'UI.
