<script lang="ts">
  import { onMount, createEventDispatcher } from 'svelte';
  import { apiClient, type Agent, type Skill, type SaveLevel } from '../lib/api';
  import { slide, fade } from 'svelte/transition';

  let agents: Agent[] = [];
  let availableSkills: Skill[] = [];
  let loading = true;
  let editingAgent: Agent | null = null;
  let saveLevel: SaveLevel = 'server';
  
  const dispatch = createEventDispatcher();

  onMount(async () => {
    await loadAgents();
  });

  async function loadAgents() {
    loading = true;
    try {
      const [agentsData, skillsData] = await Promise.all([
        apiClient.listAgents(),
        apiClient.listSkills()
      ]);
      agents = agentsData;
      availableSkills = skillsData;
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  function toggleSkill(skillId: string) {
    if (!editingAgent) return;
    if (editingAgent.skills.includes(skillId)) {
      editingAgent.skills = editingAgent.skills.filter(id => id !== skillId);
    } else {
      editingAgent.skills = [...editingAgent.skills, skillId];
    }
  }

  function editAgent(agent: Agent) {
    editingAgent = { ...agent };
  }

  function createAgent() {
    editingAgent = {
      id: crypto.randomUUID(),
      name: 'Nouvel Agent',
      description: 'Agent spécialisé...',
      system_prompt: 'Tu es un expert en...',
      skills: [],
      tools: [],
      working_directory: 'C:\\'
    };
  }

  async function saveAgent() {
    if (!editingAgent) return;
    try {
      await apiClient.saveAgent(editingAgent, saveLevel);
      await loadAgents();
      editingAgent = null;
    } catch (e) {
      console.error(e);
    }
  }

  async function deleteAgent(id: string) {
    if (!confirm('Supprimer cet agent ?')) return;
    try {
      await apiClient.deleteAgent(id);
      await loadAgents();
    } catch (e) {
      console.error(e);
    }
  }

  function selectAgent(agent: Agent) {
    dispatch('select', agent);
  }
</script>

<div class="agents-manager">
  {#if editingAgent}
    <div class="editor" in:slide>
      <div class="header">
        <h3>Éditer l'Agent</h3>
        <button class="btn-icon" on:click={() => editingAgent = null}>✕</button>
      </div>
      <div class="form-group">
        <label>Nom</label>
        <input bind:value={editingAgent.name} type="text" placeholder="Nom de l'agent" />
      </div>
      <div class="form-group">
        <label>Description</label>
        <input bind:value={editingAgent.description} type="text" placeholder="Courte description" />
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
        <label>Prompt Système</label>
        <textarea bind:value={editingAgent.system_prompt} placeholder="Instructions de l'agent..." rows="5"></textarea>
      </div>
      <div class="form-group">
        <label>Dossier de travail autorisé</label>
        <input bind:value={editingAgent.working_directory} type="text" placeholder="Ex: C:\ (C:\ pour accès total)" />
      </div>
      <div class="form-group">
        <label>Compétences (Skills)</label>
        <div class="skills-selector">
          {#if availableSkills.length === 0}
            <div class="empty-skills">Aucun skill disponible. Créez-en d'abord dans l'onglet Skills.</div>
          {/if}
          {#each availableSkills as skill}
            <label class="skill-checkbox">
              <input type="checkbox" checked={editingAgent.skills.includes(skill.id)} on:change={() => toggleSkill(skill.id)} />
              <span class="skill-name">{skill.name}</span>
            </label>
          {/each}
        </div>
      </div>
      <div class="form-group">
        <label>Outils Actifs (séparés par virgule)</label>
        <input value={editingAgent.tools.join(', ')} on:change={(e) => editingAgent.tools = e.currentTarget.value.split(',').map(s=>s.trim()).filter(Boolean)} type="text" placeholder="read_file, write_file, replace_file_content, run_command, grep_search" />
      </div>
      <div class="actions">
        <button class="btn primary" on:click={saveAgent}>Enregistrer</button>
        <button class="btn" on:click={() => editingAgent = null}>Annuler</button>
      </div>
    </div>
  {:else}
    <div class="list" in:fade>
      <div class="header">
        <h3>Agents Spécialisés</h3>
        <button class="btn primary" on:click={createAgent}>+ Nouvel Agent</button>
      </div>
      
      {#if loading}
        <div class="loading">Chargement des agents...</div>
      {:else if agents.length === 0}
        <div class="empty">Aucun agent configuré.</div>
      {:else}
        <div class="grid">
          {#each agents as agent}
            <div class="agent-card">
              <div class="agent-info">
                <div class="agent-header-row">
                  <h4>{agent.name}</h4>
                  {#if agent.level}
                    <span class="level-badge level-{agent.level}">
                      {agent.level === 'global' ? '🌐 Global' : agent.level === 'project' ? '📁 Projet' : '🖥️ Serveur'}
                    </span>
                  {/if}
                </div>
                <p>{agent.description}</p>
                <div class="tags">
                  {#each agent.tools as tool}
                    <span class="tag tool">{tool}</span>
                  {/each}
                </div>
              </div>
              <div class="agent-actions">
                <button class="btn-icon" on:click={() => selectAgent(agent)} title="Discuter avec cet agent">💬</button>
                <button class="btn-icon" on:click={() => editAgent(agent)} title="Éditer">✏️</button>
                <button class="btn-icon danger" on:click={() => deleteAgent(agent.id)} title="Supprimer">🗑️</button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .agents-manager {
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

  .agent-card {
    background: var(--surface-3);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    transition: transform 0.2s, box-shadow 0.2s;
  }
  
  .agent-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  }

  .agent-info h4 {
    margin: 0 0 0.5rem 0;
    font-size: 1.1rem;
  }

  .agent-info p {
    font-size: 0.9rem;
    color: var(--text-muted);
    margin: 0 0 1rem 0;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 1rem;
  }

  .tag {
    font-size: 0.75rem;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    background: var(--surface-1);
    border: 1px solid var(--border-color);
  }
  
  .tag.tool {
    background: rgba(var(--primary-color-rgb, 100, 100, 255), 0.1);
    color: var(--primary-color, #88f);
    border-color: rgba(var(--primary-color-rgb, 100, 100, 255), 0.3);
  }

  .agent-actions {
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
  
  .skills-selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    background: var(--surface-1);
    padding: 0.8rem;
    border-radius: 6px;
    border: 1px solid var(--border-color);
  }
  
  .skill-checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
  }
  
  .skill-name {
    font-size: 0.9rem;
    color: var(--text-color);
  }
  
  .empty-skills {
    font-size: 0.85rem;
    color: var(--text-muted);
    font-style: italic;
  }

  .agent-header-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .agent-header-row h4 {
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
</style>
