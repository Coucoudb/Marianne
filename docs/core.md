# Marianne Core (`marianne-core`)

La librairie `marianne-core` est le cœur battant du projet Marianne AI. Elle contient toute la logique métier, totalement découplée de l'interface utilisateur ou des protocoles réseau, ce qui lui permet d'être embarquée dans une app Desktop (Tauri), un serveur (Axum) ou une CLI.

## Fonctionnalités Principales

1. **Moteur d'inférence LLM (`src/llm/`)** : 
   - Initialisation et gestion du contexte `llama.cpp`.
   - Support asynchrone des inférences en streaming.
   - Intégration transparente de l'accélération matérielle (CUDA, Vulkan, Metal).
   
2. **Système Multi-Agents (`src/workspace/`)** :
   - Gestion des agents spécialisés (profils, prompts, restrictions de dossiers).
   - Base de connaissances personnalisées ("Skills").
   - Exécution asynchrone d'outils (Function Calling).

3. **Boucle ReAct et Outils Agentiques (`src/chat.rs` & `src/llm/tools.rs`)** :
   - Pipeline de discussion complet avec interception automatique des balises `<tool_call>`.
   - Délégation récursive entre agents (`delegate_task`).
   - Interaction système avec des outils natifs sécurisés (`run_command`, `replace_file_content`, `grep_search`, `read_file`, `write_file`).

4. **Gestion des Modèles Dynamiques (`src/models.rs`)** :
   - Gestion du registre local (`models.json`).
   - Téléchargement, installation et suppression automatique de modèles GGUF depuis HuggingFace.

5. **Retrieval-Augmented Generation (RAG)** :
   - **Hybride** : Combine recherche sémantique (LanceDB) et graphe de relations (petgraph).
   - Injection dynamique du contexte dans les requêtes utilisateur.

## Architecture

Le point d'entrée principal pour toute discussion avec l'IA est `marianne_core::chat::process_chat`. 
Cette fonction prend en charge l'historique, charge le prompt système de l'agent actif, gère les boucles d'outils (ReAct) et envoie les jetons texte générés sur un canal de streaming (`Sender<ChatEvent>`).

Aucune connaissance de l'extérieur n'existe ici : `marianne-core` ne gère pas de HTTP ni de WebView Tauri.
