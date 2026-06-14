<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { apiClient, type Skill, type SaveLevel } from '../lib/api';
  import { slide, fade } from 'svelte/transition';

  let skills: Skill[] = [];
  let loading = true;
  let editingSkill: Skill | null = null;
  let saveLevel: SaveLevel = 'server';
  
  const dispatch = createEventDispatcher();

  onMount(async () => {
    await loadSkills();
  });

  async function loadSkills() {
    loading = true;
    try {
      skills = await apiClient.listSkills();
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  function editSkill(skill: Skill) {
    editingSkill = { ...skill };
  }

  function createSkill() {
    editingSkill = {
      id: crypto.randomUUID(),
      name: 'Nouveau Skill',
      description: 'Courte description du domaine',
      content: 'Contenu détaillé des connaissances (par ex. procédures, faits, règles)...'
    };
  }

  async function saveSkill() {
    if (!editingSkill) return;
    try {
      await apiClient.saveSkill(editingSkill, saveLevel);
      await loadSkills();
      editingSkill = null;
    } catch (e) {
      console.error(e);
    }
  }

  async function deleteSkill(id: string) {
    if (!confirm('Supprimer cette compétence ? Les agents qui l\'utilisent ne l\'auront plus.')) return;
    try {
      await apiClient.deleteSkill(id);
      await loadSkills();
    } catch (e) {
      console.error(e);
    }
  }

  function selectSkill(skill: Skill) {
    dispatch('select', skill);
  }
</script>

<div class="skills-manager">
  {#if editingSkill}
    <div class="editor" in:slide>
      <div class="header">
        <h3>Éditer la Compétence (Skill)</h3>
        <button class="btn-icon" on:click={() => editingSkill = null}>✕</button>
      </div>
      <div class="form-group">
        <label>Nom</label>
        <input bind:value={editingSkill.name} type="text" placeholder="Nom du skill" />
      </div>
      <div class="form-group">
        <label>Description</label>
        <input bind:value={editingSkill.description} type="text" placeholder="Courte description" />
      </div>
      <div class="form-group">
        <label>Scope (Chargement Contextuel)</label>
        <input bind:value={editingSkill.scope} type="text" placeholder="Ex: **/*.rs (laisser vide pour toujours charger)" />
      </div>
      <div class="form-group">
        <label>Sauvegarder dans :</label>
        <select bind:value={saveLevel}>
          <option value="server">Serveur (Défaut, stockage global Marianne)</option>
          <option value="project">Projet (Dossier .marianne du projet actuel, idéal pour Git)</option>
          <option value="global">Global (Préférences utilisateur, ~/.marianne)</option>
        </select>
      </div>
      <div class="form-group">
        <label>Contenu de Connaissances</label>
        <textarea bind:value={editingSkill.content} placeholder="Texte de connaissances que l'agent lira..." rows="10"></textarea>
      </div>
      <div class="actions">
        <button class="btn primary" on:click={saveSkill}>Enregistrer</button>
        <button class="btn" on:click={() => editingSkill = null}>Annuler</button>
      </div>
    </div>
  {:else}
    <div class="list" in:fade>
      <div class="header">
        <h3>Base de Connaissances (Skills)</h3>
        <button class="btn primary" on:click={createSkill}>+ Nouveau Skill</button>
      </div>
      
      {#if loading}
        <div class="loading">Chargement des skills...</div>
      {:else if skills.length === 0}
        <div class="empty">Aucune compétence configurée.</div>
      {:else}
        <div class="grid">
          {#each skills as skill}
            <div class="skill-card">
              <div class="skill-info">
                <div class="skill-header-row">
                  <h4>{skill.name}</h4>
                  {#if skill.level}
                    <span class="level-badge level-{skill.level}">
                      {skill.level === 'global' ? '🌐 Global' : skill.level === 'project' ? '📁 Projet' : '🖥️ Serveur'}
                    </span>
                  {/if}
                </div>
                <p>{skill.description}</p>
                {#if skill.scope}
                  <div class="scope-tag">🎯 {skill.scope}</div>
                {/if}
                <div class="preview">{skill.content.slice(0, 80)}...</div>
              </div>
              <div class="skill-actions">
                <button class="btn-icon" on:click={() => editSkill(skill)} title="Éditer">✏️</button>
                <button class="btn-icon danger" on:click={() => deleteSkill(skill.id)} title="Supprimer">🗑️</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .skills-manager {
    background: var(--surface-2);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 1.5rem;
    margin-bottom: 1.5rem;
    box-shadow: 0 4px 20px rgba(0,0,0,0.2);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }
  
  .header h3 {
    margin: 0;
    font-weight: 500;
    color: var(--text-color);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }

  .skill-card {
    background: var(--surface-3);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    transition: transform 0.2s, box-shadow 0.2s;
  }
  
  .skill-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  }

  .skill-info h4 {
    margin: 0 0 0.5rem 0;
    font-size: 1.1rem;
  }

  .skill-info p {
    font-size: 0.9rem;
    color: var(--text-muted);
    margin: 0 0 0.5rem 0;
  }
  
  .preview {
    font-size: 0.8rem;
    color: var(--text-tertiary);
    font-family: monospace;
    background: var(--surface-1);
    padding: 0.5rem;
    border-radius: 4px;
    margin-bottom: 1rem;
  }

  .skill-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.4rem;
    font-size: 0.9rem;
    color: var(--text-muted);
  }

  .form-group input, .form-group textarea, .form-group select {
    width: 100%;
    padding: 0.7rem;
    background: var(--surface-1);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-color);
    font-family: inherit;
  }
  
  .form-group input:focus, .form-group textarea:focus, .form-group select:focus {
    border-color: var(--primary-color);
    outline: none;
  }

  .actions {
    display: flex;
    gap: 1rem;
    margin-top: 1.5rem;
  }

  .btn {
    padding: 0.6rem 1.2rem;
    border-radius: 6px;
    border: 1px solid var(--border-color);
    background: var(--surface-3);
    color: var(--text-color);
    cursor: pointer;
    transition: all 0.2s;
  }
  
  .btn:hover {
    background: var(--surface-hover);
  }
  
  .btn.primary {
    background: var(--primary-color, #4a90e2);
    color: white;
    border: none;
  }
  
  .btn.primary:hover {
    background: var(--primary-color-dark, #357abd);
  }

  .btn-icon {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0.4rem;
    border-radius: 4px;
    transition: all 0.2s;
  }
  
  .btn-icon:hover {
    background: var(--surface-hover);
    color: var(--text-color);
  }
  
  .btn-icon.danger:hover {
    background: rgba(255, 50, 50, 0.1);
    color: #ff4444;
  }

  .skill-header-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .skill-header-row h4 {
    margin: 0;
    font-size: 1.1rem;
  }

  .level-badge {
    font-size: 0.7rem;
    padding: 0.15rem 0.5rem;
    border-radius: 12px;
    font-weight: 500;
    white-space: nowrap;
    letter-spacing: 0.02em;
  }

  .level-global {
    background: rgba(100, 149, 237, 0.15);
    color: #6495ed;
    border: 1px solid rgba(100, 149, 237, 0.3);
  }

  .level-server {
    background: rgba(160, 160, 180, 0.12);
    color: var(--text-muted);
    border: 1px solid rgba(160, 160, 180, 0.25);
  }

  .level-project {
    background: rgba(80, 200, 120, 0.15);
    color: #50c878;
    border: 1px solid rgba(80, 200, 120, 0.3);
  }

  .scope-tag {
    font-size: 0.75rem;
    color: var(--text-tertiary);
    margin-bottom: 0.4rem;
  }
</style>
