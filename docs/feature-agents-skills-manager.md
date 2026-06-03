# Feature : Gestionnaire d'Agents et Skills

**Status** : 📋 Spécification  
**Priorité** : Moyenne  
**Version cible** : 2.0  
**Date** : 1er juin 2026

---

## 🎯 Objectif

Permettre aux utilisateurs de créer et gérer des agents personnalisés et leurs skills associées directement depuis l'interface web de Marianne, transformant l'assistant d'un outil juridique spécialisé vers un assistant généraliste configurable. Rendre marianne utilisable pour des recherches générales (juridique, IT, sécurité, finance, ...)

## 🔄 Changements de paradigme

### 1. Suppression du prompt système juridique

**Actuel** :
```markdown
Tu es Marianne, une assistante administrative française spécialisée dans 
les démarches administratives et juridiques. Tu ne réponds QUE sur des 
sujets liés à l'administration française...
```

**Nouveau** :
```markdown
Tu es Marianne, une assistante virtuelle polyvalente. Tu adaptes tes réponses 
au contexte et aux compétences définies par l'utilisateur via les agents et 
skills activés.
```

**Fichiers à modifier** :
- `marianne-core/src/prompts/system.rs` : fonction `build_system_prompt()`
- Le prompt système devient **dynamique** et construit depuis :
  - Les instructions de l'agent actif
  - Les skills associées à l'agent
  - Le profil utilisateur (conservé pour personnalisation)

### 2. Suppression des restrictions d'usage

**Restrictions actuelles à retirer** :
- ❌ "Ne réponds QUE sur des sujets administratifs"
- ❌ "Refuse toute question hors cadre"
- ❌ "Redirige vers un conseiller si hors domaine"

**Nouveau comportement** :
- ✅ L'agent actif définit son propre périmètre
- ✅ Plusieurs agents peuvent coexister (coding, juridique, médical, etc.)
- ✅ L'utilisateur choisit quel agent activer

**Bonus** : 
L'utilisateur peut créer un agent qui permet de faire dialoguer divers agents spécialisé ce qui permet d'affiner la réponse avec différents spécialistes et avoir une réponse beaucoup plus fiable.

### 3. Recherche web non restreinte

**Actuel** :
```rust
// marianne-core/src/web/sources.rs
const OFFICIAL_SOURCES: &[&str] = &[
    "service-public.fr",
    "legifrance.gouv.fr",
    "impots.gouv.fr",
    // ... uniquement sites officiels
];

// Filtrage strict dans le code
if !is_official_source(&url) {
    return None; // Source rejetée
}
```

**Nouveau** :
```rust
// Suppression du filtrage par domaine
// Les sources sont évaluées par pertinence, pas par origine
// Configuration par agent :
pub struct WebSearchConfig {
    pub allowed_domains: Option<Vec<String>>, // None = tous domaines
    pub blocked_domains: Vec<String>,         // Liste noire optionnelle
    pub min_sources_for_confidence: usize,    // Augmenté à 5
}
```

**Fichiers à modifier** :
- `marianne-core/src/web/sources.rs` : retirer la liste blanche
- `marianne-core/src/web/searcher.rs` : désactiver le filtrage strict
- `marianne-core/src/rag/retriever.rs` : configuration par agent

### 4. Augmentation du seuil de confiance web

**Actuel** :
```rust
const MIN_SOURCES_FOR_HIGH_CONFIDENCE: usize = 2;
const MIN_SOURCES_FOR_MEDIUM_CONFIDENCE: usize = 1;
```

**Nouveau** :
```rust
const MIN_SOURCES_FOR_HIGH_CONFIDENCE: usize = 5;  // +150%
const MIN_SOURCES_FOR_MEDIUM_CONFIDENCE: usize = 3; // +200%
const MIN_SOURCES_FOR_LOW_CONFIDENCE: usize = 1;    // Nouveau palier
```

**Fichiers à modifier** :
- `marianne-core/src/llm/confidence.rs`
- `marianne-core/src/web/rag_updater.rs`

---

## 📐 Architecture de la feature

### Structure de données

```
workspace/
├── .github/
│   ├── agents/                    # Agents personnalisés
│   │   ├── coding-assistant.agent.md
│   │   ├── medical-advisor.agent.md
│   │   └── general-assistant.agent.md
│   └── skills/                    # Skills personnalisées
│       ├── rust-expert/
│       │   ├── SKILL.md
│       │   └── resources/
│       ├── medical-knowledge/
│       └── web-research/
└── marianne/
    └── user-data/
        ├── agents.json            # Registre local
        ├── skills.json
        └── active-agent.json      # Agent actif
```

### Modèle de données

#### Agent

```rust
// marianne-core/src/workspace/agent.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Identifiant unique (slug)
    pub id: String,
    
    /// Nom affiché
    pub name: String,
    
    /// Description courte (triggers pour activation)
    pub description: String,
    
    /// Instructions markdown complètes
    pub instructions: String,
    
    /// Outils autorisés (file_search, grep_search, etc.)
    pub tools: Vec<String>,
    
    /// Skills associées (IDs)
    pub skills: Vec<String>,
    
    /// Configuration de déclenchement
    pub apply_to: ApplyTo,
    
    /// Configuration de recherche web
    pub web_search_config: WebSearchConfig,
    
    /// Métadonnées
    pub created_at: i64,
    pub updated_at: i64,
    pub author: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyTo {
    /// Langages de programmation
    pub languages: Vec<String>,
    
    /// Patterns de fichiers (glob)
    pub file_patterns: Vec<String>,
    
    /// Activation automatique
    pub auto_activation: bool,
    
    /// Keywords de déclenchement
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    /// Domaines autorisés (None = tous)
    pub allowed_domains: Option<Vec<String>>,
    
    /// Domaines bloqués
    pub blocked_domains: Vec<String>,
    
    /// Nombre minimum de sources pour haute confiance
    pub min_sources_for_confidence: usize,
    
    /// Activer la recherche web
    pub enable_web_search: bool,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            allowed_domains: None, // Tous domaines autorisés
            blocked_domains: vec![],
            min_sources_for_confidence: 5,
            enable_web_search: true,
        }
    }
}
```

#### Skill

```rust
// marianne-core/src/workspace/skill.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Identifiant unique
    pub id: String,
    
    /// Nom affiché
    pub name: String,
    
    /// Description avec triggers
    pub description: String,
    
    /// Contenu markdown
    pub content: String,
    
    /// Fichiers de ressources
    pub resources: Vec<SkillResource>,
    
    /// Métadonnées
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResource {
    pub name: String,
    pub path: String,
    pub content: String,
    pub mime_type: String,
}
```

---

## 🔧 Backend API

### Endpoints

```
GET    /api/v1/workspace/agents              Liste tous les agents
POST   /api/v1/workspace/agents              Crée un nouvel agent
GET    /api/v1/workspace/agents/:id          Détails d'un agent
PUT    /api/v1/workspace/agents/:id          Met à jour un agent
DELETE /api/v1/workspace/agents/:id          Supprime un agent
POST   /api/v1/workspace/agents/:id/activate Active un agent

GET    /api/v1/workspace/skills              Liste toutes les skills
POST   /api/v1/workspace/skills              Crée une nouvelle skill
GET    /api/v1/workspace/skills/:id          Détails d'une skill
PUT    /api/v1/workspace/skills/:id          Met à jour une skill
DELETE /api/v1/workspace/skills/:id          Supprime une skill

POST   /api/v1/workspace/validate-agent      Valide la syntaxe d'un agent
POST   /api/v1/workspace/validate-skill      Valide la syntaxe d'une skill

GET    /api/v1/workspace/templates/agents    Templates d'agents prédéfinis
GET    /api/v1/workspace/templates/skills    Templates de skills prédéfinis
```

### Exemples de requêtes

#### Créer un agent

```bash
curl -X POST http://localhost:3000/api/v1/workspace/agents \
  -H "Content-Type: application/json" \
  -d '{
    "id": "coding-assistant",
    "name": "Coding Assistant",
    "description": "Expert en développement Rust et TypeScript",
    "instructions": "Tu es un expert en développement...",
    "tools": ["file_search", "grep_search", "vscode_listCodeUsages"],
    "skills": ["rust-expert", "typescript-expert"],
    "apply_to": {
      "languages": ["rust", "typescript"],
      "file_patterns": ["**/*.rs", "**/*.ts"],
      "auto_activation": true,
      "keywords": ["code", "bug", "refactor"]
    },
    "web_search_config": {
      "allowed_domains": null,
      "blocked_domains": [],
      "min_sources_for_confidence": 5,
      "enable_web_search": true
    }
  }'
```

#### Activer un agent

```bash
curl -X POST http://localhost:3000/api/v1/workspace/agents/coding-assistant/activate
```

---

## 🎨 Frontend UI

### Page principale : AgentsManager.svelte

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  
  let activeTab: 'agents' | 'skills' = 'agents';
  let agents: Agent[] = [];
  let skills: Skill[] = [];
  let selectedAgent: Agent | null = null;
  let activeAgentId: string | null = null;

  onMount(async () => {
    agents = await fetchAgents();
    skills = await fetchSkills();
    activeAgentId = await fetchActiveAgent();
  });

  async function activateAgent(agentId: string) {
    await fetch(`/api/v1/workspace/agents/${agentId}/activate`, {
      method: 'POST'
    });
    activeAgentId = agentId;
  }
</script>

<div class="agents-manager">
  <header>
    <h1>Gestion des Agents et Skills</h1>
    <button on:click={createNewAgent}>+ Nouvel Agent</button>
  </header>

  <div class="tabs">
    <button class:active={activeTab === 'agents'} 
            on:click={() => activeTab = 'agents'}>
      🤖 Agents ({agents.length})
    </button>
    <button class:active={activeTab === 'skills'} 
            on:click={() => activeTab = 'skills'}>
      🎓 Skills ({skills.length})
    </button>
  </div>

  {#if activeTab === 'agents'}
    <div class="agents-grid">
      {#each agents as agent}
        <div class="agent-card" class:active={agent.id === activeAgentId}>
          <h3>{agent.name}</h3>
          <p>{agent.description}</p>
          <div class="agent-meta">
            <span>{agent.skills.length} skills</span>
            <span>{agent.tools.length} outils</span>
          </div>
          <div class="actions">
            {#if agent.id === activeAgentId}
              <span class="badge active">Actif</span>
            {:else}
              <button on:click={() => activateAgent(agent.id)}>
                Activer
              </button>
            {/if}
            <button on:click={() => editAgent(agent)}>Éditer</button>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <!-- Skills grid -->
  {/if}
</div>
```

### Composant : AgentEditor.svelte

```svelte
<script lang="ts">
  import MarkdownEditor from './MarkdownEditor.svelte';
  import SkillPicker from './SkillPicker.svelte';
  import WebSearchConfig from './WebSearchConfig.svelte';

  export let agent: Agent;
  export let skills: Skill[];

  let formData = { ...agent };

  async function save() {
    const response = await fetch(`/api/v1/workspace/agents/${agent.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(formData)
    });

    if (response.ok) {
      alert('Agent sauvegardé !');
    }
  }
</script>

<div class="agent-editor">
  <h2>Éditer : {agent.name}</h2>

  <section>
    <h3>Informations générales</h3>
    <label>
      Nom
      <input type="text" bind:value={formData.name} />
    </label>
    
    <label>
      Description (déclencheurs)
      <textarea bind:value={formData.description} rows="3" />
    </label>
  </section>

  <section>
    <h3>Instructions</h3>
    <MarkdownEditor bind:value={formData.instructions} />
  </section>

  <section>
    <h3>Skills associées</h3>
    <SkillPicker {skills} bind:selected={formData.skills} />
  </section>

  <section>
    <h3>Outils autorisés</h3>
    <div class="tools-grid">
      {#each AVAILABLE_TOOLS as tool}
        <label>
          <input type="checkbox" 
                 value={tool.id}
                 checked={formData.tools.includes(tool.id)} />
          {tool.name}
        </label>
      {/each}
    </div>
  </section>

  <section>
    <h3>Configuration de recherche web</h3>
    <WebSearchConfig bind:config={formData.web_search_config} />
  </section>

  <section>
    <h3>Activation automatique</h3>
    <label>
      Langages
      <input type="text" 
             bind:value={formData.apply_to.languages}
             placeholder="rust, typescript, python" />
    </label>
    
    <label>
      Patterns de fichiers (glob)
      <input type="text" 
             bind:value={formData.apply_to.file_patterns}
             placeholder="**/*.rs, src/**/*.ts" />
    </label>
    
    <label>
      <input type="checkbox" bind:checked={formData.apply_to.auto_activation} />
      Activation automatique selon le contexte
    </label>
  </section>

  <div class="actions">
    <button on:click={cancel}>Annuler</button>
    <button class="primary" on:click={save}>Sauvegarder</button>
  </div>
</div>
```

### Composant : WebSearchConfig.svelte

```svelte
<script lang="ts">
  export let config: WebSearchConfig;
</script>

<div class="web-search-config">
  <label>
    <input type="checkbox" bind:checked={config.enable_web_search} />
    Activer la recherche web
  </label>

  {#if config.enable_web_search}
    <label>
      Domaines autorisés (vide = tous)
      <input type="text" 
             bind:value={config.allowed_domains}
             placeholder="wikipedia.org, stackoverflow.com" />
      <small>Séparez par des virgules. Laissez vide pour autoriser tous les domaines.</small>
    </label>

    <label>
      Domaines bloqués
      <input type="text" 
             bind:value={config.blocked_domains}
             placeholder="spam.com, ads.example.com" />
    </label>

    <label>
      Nombre de sources minimum pour haute confiance
      <input type="number" 
             bind:value={config.min_sources_for_confidence}
             min="1" max="10" />
      <small>
        Recommandé : 5 sources. Plus le nombre est élevé, plus la réponse est fiable.
      </small>
    </label>
  {/if}
</div>
```

---

## 🚀 Plan d'implémentation

### Phase 1 : Modifications du prompt système (1-2 jours)

**Fichiers à modifier** :
```
marianne-core/src/prompts/system.rs
marianne-core/src/state.rs (ajouter active_agent)
```

**Tâches** :
- [ ] Rendre le prompt système dynamique
- [ ] Charger les instructions depuis l'agent actif
- [ ] Supprimer les restrictions juridiques
- [ ] Tests avec différents types d'agents

### Phase 2 : Recherche web non restreinte (1-2 jours)

**Fichiers à modifier** :
```
marianne-core/src/web/sources.rs
marianne-core/src/web/searcher.rs
marianne-core/src/rag/retriever.rs
marianne-core/src/llm/confidence.rs
```

**Tâches** :
- [ ] Supprimer la liste blanche de domaines
- [ ] Implémenter WebSearchConfig
- [ ] Augmenter les seuils de confiance (2→5, 1→3)
- [ ] Tester avec sources variées

### Phase 3 : Backend - Gestion d'agents (3-4 jours)

**Nouveaux fichiers** :
```
marianne-core/src/workspace/mod.rs
marianne-core/src/workspace/agent.rs
marianne-core/src/workspace/agent_manager.rs
marianne-core/src/workspace/skill_manager.rs
marianne-server/src/routes/workspace.rs
```

**Tâches** :
- [ ] Parser les fichiers .agent.md (YAML frontmatter)
- [ ] CRUD agents (list, create, update, delete)
- [ ] Validation syntaxique
- [ ] Activation d'agents
- [ ] Tests unitaires

### Phase 4 : Backend - Gestion de skills (2-3 jours)

**Tâches** :
- [ ] Parser les skills (SKILL.md)
- [ ] CRUD skills
- [ ] Gestion des ressources associées
- [ ] Tests

### Phase 5 : Frontend - UI de gestion (4-5 jours)

**Nouveaux fichiers** :
```
marianne-web/src/components/AgentsManager.svelte
marianne-web/src/components/AgentList.svelte
marianne-web/src/components/AgentEditor.svelte
marianne-web/src/components/SkillList.svelte
marianne-web/src/components/SkillEditor.svelte
marianne-web/src/components/MarkdownEditor.svelte
marianne-web/src/components/SkillPicker.svelte
marianne-web/src/components/WebSearchConfig.svelte
```

**Tâches** :
- [ ] Page de gestion des agents
- [ ] Éditeur d'agent avec preview markdown
- [ ] Sélecteur de skills
- [ ] Configuration de recherche web
- [ ] Validation côté client

### Phase 6 : Templates prédéfinis (1-2 jours)

**Tâches** :
- [ ] Créer template "Assistant généraliste"
- [ ] Créer template "Coding Assistant"
- [ ] Créer template "Research Assistant"
- [ ] Créer template "Medical Advisor"
- [ ] API d'importation de templates

### Phase 7 : Tests et documentation (2-3 jours)

**Tâches** :
- [ ] Tests end-to-end
- [ ] Tests d'intégration backend/frontend
- [ ] Documentation utilisateur
- [ ] Guide de migration depuis l'assistant juridique
- [ ] Exemples d'agents personnalisés

---

## 📊 Estimation totale

**Temps de développement** : 16-22 jours (3-4 semaines)

**Répartition** :
- Backend : 8-11 jours
- Frontend : 4-5 jours
- Tests & Doc : 2-3 jours
- Migration : 2-3 jours

---

## 🎯 Critères de succès

### Fonctionnels

- [ ] L'utilisateur peut créer un agent depuis l'interface web
- [ ] L'utilisateur peut éditer et supprimer des agents
- [ ] L'utilisateur peut activer/désactiver des agents
- [ ] Le prompt système s'adapte à l'agent actif
- [ ] La recherche web fonctionne sans restriction de domaine
- [ ] Le seuil de confiance est augmenté (5 sources minimum)
- [ ] Les skills peuvent être associées aux agents

### Non-fonctionnels

- [ ] Temps de chargement < 500ms pour la liste des agents
- [ ] Sauvegarde d'un agent < 200ms
- [ ] Interface intuitive (pas de formation nécessaire)
- [ ] Migration transparente depuis l'ancien système
- [ ] Compatibilité avec les agents existants en `.github/agents/`

---

## 🔒 Sécurité

### Validation

- ✅ Validation YAML du frontmatter
- ✅ Sanitisation du markdown (pas d'injection de code)
- ✅ Validation des patterns glob
- ✅ Limite de taille des instructions (100 Ko max)
- ✅ Limite du nombre d'agents (50 max par utilisateur)

### Permissions

- ✅ L'utilisateur ne peut modifier que ses propres agents
- ✅ Les agents système (prédéfinis) sont en lecture seule
- ✅ Les outils dangereux nécessitent une confirmation explicite

---

## 🗺️ Migration depuis l'assistant juridique

### Étape 1 : Créer l'agent "Assistant Juridique"

```yaml
---
name: Assistant Juridique
description: Expert en démarches administratives et juridiques françaises
applyTo:
  keywords: [juridique, administratif, CAF, impôts, RSA]
  auto_activation: false
webSearchConfig:
  allowed_domains:
    - service-public.fr
    - legifrance.gouv.fr
    - impots.gouv.fr
    - caf.fr
  min_sources_for_confidence: 3
---

Tu es Marianne, une assistante administrative française spécialisée...
[ancien prompt système]
```

### Étape 2 : Créer l'agent "Assistant Généraliste"

```yaml
---
name: Assistant Généraliste
description: Assistant polyvalent pour toutes questions
applyTo:
  auto_activation: true
webSearchConfig:
  allowed_domains: null
  min_sources_for_confidence: 5
---

Tu es Marianne, une assistante virtuelle polyvalente...
```

### Étape 3 : Migration des utilisateurs

- Les utilisateurs existants gardent l'agent juridique par défaut
- Un message les informe de la nouvelle fonctionnalité
- Ils peuvent créer leurs propres agents

---

## 📚 Ressources

### Templates d'agents prédéfinis

Créer dans `.github/agents/templates/` :
- `general-assistant.agent.md`
- `coding-assistant.agent.md`
- `research-assistant.agent.md`
- `medical-advisor.agent.md`
- `legal-assistant.agent.md` (ancien comportement)

### Documentation utilisateur

Créer dans `docs/user-guide/` :
- `agents-getting-started.md`
- `skills-creation-guide.md`
- `advanced-agent-configuration.md`
- `migration-from-legal-assistant.md`

---

## ✅ Checklist d'implémentation

### Backend Core

- [ ] `marianne-core/src/workspace/mod.rs`
- [ ] `marianne-core/src/workspace/agent.rs`
- [ ] `marianne-core/src/workspace/agent_manager.rs`
- [ ] `marianne-core/src/workspace/skill_manager.rs`
- [ ] Modifier `marianne-core/src/prompts/system.rs` (prompt dynamique)
- [ ] Modifier `marianne-core/src/web/sources.rs` (retirer liste blanche)
- [ ] Modifier `marianne-core/src/llm/confidence.rs` (seuils augmentés)
- [ ] Tests unitaires complets

### Backend Server

- [ ] `marianne-server/src/routes/workspace.rs`
- [ ] Ajouter routes dans `marianne-server/src/routes/mod.rs`
- [ ] Tests d'intégration API

### Frontend

- [ ] `marianne-web/src/components/AgentsManager.svelte`
- [ ] `marianne-web/src/components/AgentList.svelte`
- [ ] `marianne-web/src/components/AgentEditor.svelte`
- [ ] `marianne-web/src/components/SkillList.svelte`
- [ ] `marianne-web/src/components/SkillEditor.svelte`
- [ ] `marianne-web/src/components/MarkdownEditor.svelte`
- [ ] `marianne-web/src/components/SkillPicker.svelte`
- [ ] `marianne-web/src/components/WebSearchConfig.svelte`
- [ ] Ajouter route dans le router
- [ ] Tests E2E

### Documentation

- [ ] Mettre à jour `docs/marianne-server-api.md`
- [ ] Créer `docs/user-guide/agents-getting-started.md`
- [ ] Créer `docs/user-guide/skills-creation-guide.md`
- [ ] Créer `docs/developer/agents-architecture.md`

### Templates

- [ ] `.github/agents/templates/general-assistant.agent.md`
- [ ] `.github/agents/templates/coding-assistant.agent.md`
- [ ] `.github/agents/templates/research-assistant.agent.md`
- [ ] `.github/agents/templates/legal-assistant.agent.md`

---

**Auteur** : Équipe Marianne  
**Dernière mise à jour** : 1er juin 2026  
**Status** : En attente d'implémentation
