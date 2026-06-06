# 🚀 Feuille de Route et Améliorations Futures : Marianne AI

Ce document détaille les perspectives d'évolution pour transformer **Marianne** en un véritable écosystème d'Agents Autonomes Généralistes. Il se base sur l'architecture actuelle (LLM local, RAG hybride, système de plugins/skills) et projette les prochaines grandes étapes techniques.

---

## 1. 🧠 Pipeline RAG (Retrieval-Augmented Generation)

Le RAG actuel combine recherche vectorielle sémantique et FTS (Full-Text Search) avec un début de Knowledge Graph (SurrealDB).

### A. Catégorisation Dynamique et Métadonnées
*   **Problème actuel :** La catégorisation était codée en dur ("sante", "caf") et limitait la recherche.
*   **Amélioration :** Intégrer un système de **tagging dynamique**. Lors de l'ingestion, un petit LLM extrait automatiquement les thèmes clés du document (ex: `code_source`, `finance`, `manuel_utilisateur`). Lors de la recherche, ces tags servent de filtre SQL (`WHERE tags CONTAINS 'finance'`) pour réduire drastiquement l'espace de recherche vectoriel.

### B. Semantic Chunking (Découpage Sémantique)
*   **Problème actuel :** Le texte est découpé de manière statique ou par nombre de mots/caractères, coupant parfois des idées en deux.
*   **Amélioration :** Utiliser des heuristiques NLP pour découper par paragraphes ou par cohérence sémantique, assurant que chaque chunk inséré dans le VectorStore a du sens de manière isolée.

### C. Évolution du Knowledge Graph (Graph-RAG)
*   **Amélioration :** L'extraction des entités (Nœuds) et de leurs relations (Edges) pour le graphe peut être grandement améliorée. L'objectif est de permettre à l'IA de faire du "Multi-hop reasoning" (raisonnement à plusieurs sauts) pour répondre à des questions complexes (ex: "Quel est le point commun entre le document A et B ?").

---

## 2. 🤖 Système d'Agents Autonomes

Les Agents sont le cœur de la nouvelle architecture de Marianne. Ils disposent d'un prompt système, d'outils et d'un espace de travail.

### A. Mémoire à Long Terme (Agent Memory)
*   **Concept :** Actuellement, les agents ont la mémoire de leur contexte immédiat.
*   **Amélioration :** Donner aux agents une mémoire persistante (via la base vectorielle) pour qu'ils se souviennent de leurs erreurs passées, des préférences de l'utilisateur, ou des conventions d'un projet de code d'une session à l'autre.

### B. Orchestration Multi-Agents
*   **Concept :** Faire collaborer plusieurs agents entre eux.
*   **Amélioration :** Créer un Agent "Manager" capable de diviser une tâche complexe et de la déléguer à des sous-agents spécialisés (un Agent "Chercheur", un Agent "Développeur", un Agent "Testeur"), puis de fusionner leurs travaux.

### C. Sécurité et Sandboxing des Outils (`run_command`)
*   **Problème actuel :** L'outil d'exécution de commande tourne directement sur le système hôte, ce qui est risqué si un agent décide de lancer une commande destructive (`rm -rf`).
*   **Amélioration :** Exécuter les outils système dans des conteneurs isolés (Docker ou WebAssembly) ou ajouter un système de **Human-in-the-loop** (demander l'approbation de l'utilisateur via une notification UI avant d'exécuter une commande dangereuse).

---

## 3. 🎓 Compétences (Skills System)

Les Skills permettent de donner des instructions ou de la connaissance experte à un agent.

### A. Skills Dynamiques (WASM / Lua)
*   **Problème actuel :** Les skills sont principalement du texte (contexte Markdown) injecté dans le prompt système.
*   **Amélioration :** Permettre aux skills de contenir du code exécutable léger (scripts Lua ou modules WebAssembly). Ainsi, un skill "Calcul Mathématique" pourrait intercepter une question, faire le calcul exact via un script de manière déterministe, et renvoyer la réponse sans hallucination du LLM.

### B. Place de Marché (Skill Hub)
*   **Amélioration :** Créer un écosystème où la communauté peut partager des Skills ou des Agents pré-configurés (ex: "Agent Réviseur de Code Rust", "Agent Assistant de Cuisine") importables en un clic depuis l'application client.

---

## 4. ⚙️ Moteur LLM et Chat (`marianne-core/llm`)

### A. Multimodalité (Vision & Voix)
*   **Amélioration :** Intégrer les modèles LLaVA ou Phi-3-Vision supportés par `llama.cpp` pour que Marianne puisse "voir" des images envoyées dans le chat ou lire des graphiques. Intégration de Whisper (Speech-to-Text) pour discuter vocalement.

### B. Gestion Dynamique de la VRAM (Offloading Intelligent)
*   **Amélioration :** Le système de détection GPU a été grandement amélioré, mais on peut aller plus loin en déchargeant dynamiquement le modèle de la VRAM si d'autres applications lourdes (ex: jeu vidéo) se lancent, et le recharger en mémoire RAM classique, ou vice-versa.

---

## 5. 💻 Client Application (Frontend Svelte / Electron)

### A. Interface d'Outils Riches (Rich UI)
*   **Problème actuel :** Les échanges sont textuels. Si l'agent utilise un outil, l'utilisateur ne le voit que via du texte.
*   **Amélioration :** Permettre aux agents de renvoyer du JSON rendu de manière visuelle par Svelte. (ex: Si l'agent fait une recherche RAG, afficher un graphe interactif des sources. Si l'agent analyse un dossier, afficher une arborescence cliquable).

### B. Éditeur de Workflow
*   **Amélioration :** Créer une interface "Nœuds et Liens" (façon Node-RED ou ComfyUI) permettant à l'utilisateur de dessiner visuellement comment les Agents, les Skills et le RAG interagissent entre eux pour créer des pipelines sur mesure.

---

## 6. 📊 Monitoring et Télémétrie Système

Pour assurer la stabilité et les performances d'un serveur IA qui tourne en permanence, une observabilité complète est cruciale.

### A. Tableau de Bord des Ressources (Dashboard)
*   **Concept :** Ajouter une vue "Monitoring" ou "Santé du Serveur" dans l'application client.
*   **Amélioration :** Afficher en temps réel des graphiques et des jauges sur les métriques critiques :
    *   **Utilisation RAM :** Quelle quantité de mémoire système est consommée par la base vectorielle (SurrealDB), le serveur Rust et le modèle LLM.
    *   **Utilisation VRAM :** L'espace pris par les couches (layers) du modèle sur la carte graphique, avec des alertes si on approche la limite (OOM).
    *   **Utilisation CPU/GPU :** Pourcentage de charge lors des pics d'inférence ou d'ingestion RAG.
    *   **Espace Disque :** Stockage consommé par les modèles GGUF, la base de données vectorielle et les logs.

### B. Alertes et Auto-Régulation
*   **Amélioration :** Mettre en place un système d'alertes préventives (ex: notification "Espace disque faible" ou "VRAM saturée"). À terme, le serveur pourrait s'auto-réguler (ex: décharger un agent inactif de la mémoire si la RAM devient critique).
