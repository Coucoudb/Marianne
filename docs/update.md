Section Upgrade (Propositions d'Évolutions)

Voici des recommandations pour faire évoluer l'architecture et les fonctionnalités à moyen et long terme.

### A. Sécurité et Mode Multi-Utilisateurs ✅ IMPLÉMENTÉ

*   ✅ **Authentification multi-utilisateurs (API Keys en base)** : Table `api_keys` SQLite avec hash SHA-256, middleware Axum `auth_middleware` protégeant `/chat`, `/history`, `/profile`, `/documents`, `/workspace`. Bootstrap via `--bootstrap-admin-key` / `MARIANNE_BOOTSTRAP_ADMIN_KEY`.
*   ✅ **Système de rôles (`user` / `admin`)** : Guard `require_admin` protégeant la gestion des modèles et les routes `/api/v1/admin/keys`. Les routes admin permettent de créer, lister et révoquer des clés.
*   ✅ **Isolation des Contextes** : Historique SQLite scopé par `user_id` (dérivé du hash de la clé). Chaque utilisateur ne voit que ses propres conversations.
*   ✅ **Chiffrement au Repos** : Messages (`user_message`, `assistant_message`) chiffrés en AES-256-GCM dans SQLite. Clé dérivée de `MARIANNE_DB_KEY` (ou du hostname en fallback). Rétro-compatible avec les données existantes.

### B. Expérience Utilisateur et Frontend (Moyen Terme)
*   **Gestion de Modèles (HuggingFace Hub)** : Intégrer directement dans l'UI un explorateur pour télécharger des modèles GGUF depuis HuggingFace (avec indicateur de compatibilité RAM/VRAM), avec reprise sur erreur.

### C. Capacités de l'IA (Long Terme)
*   **Mémoire à Long Terme (Long-Term Memory)** : Ajouter un agent qui résume les conversations passées et stocke les préférences ou entités utilisateur dans la base RAG pour personnaliser les futures interactions.
*   **Support Multi-Modal** : Intégrer LLaVA ou un équivalent dans le pipeline Rust pour permettre à Marianne de "voir" des images envoyées depuis le client.
*   **Système de Plugins / Tools (Function Calling)** : Standardiser l'exécution de code ou l'appel d'APIs externes via le framework *Tool Calling* supporté par de nombreux modèles récents, permettant de créer une vraie "Marketplace" d'extensions.
