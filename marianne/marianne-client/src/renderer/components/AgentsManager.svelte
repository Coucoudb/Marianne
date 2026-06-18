<script lang="ts">
  import { onMount } from 'svelte';
  import { apiClient, type Agent, type Skill, type SaveLevel } from '../lib/api';
  import { slide, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  
  import { Button } from "$lib/components/ui/button";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import * as Select from "$lib/components/ui/select";
  import { Switch } from "$lib/components/ui/switch";
  import * as Tooltip from "$lib/components/ui/tooltip";
  import { Label } from "$lib/components/ui/label";

  let { onselect, onsubroute } = $props<{
    onselect?: (agent: Agent) => void;
    onsubroute?: (label: string) => void;
  }>();

  let agents: Agent[] = $state([]);
  let availableSkills: Skill[] = $state([]);
  let loading = $state(true);
  let view: 'list' | 'form' = $state('list');
  
  let editingAgent: Agent | null = $state(null);
  let saveLevel: string = $state("server");
  let pendingDeleteId: string | null = $state(null);
  let isCreating = $state(false);
  let errorMsg: string | null = $state(null);

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
      errorMsg = e instanceof Error ? e.message : 'Une erreur est survenue.';
    } finally {
      loading = false;
    }
  }

  function toggleSkill(skillId: string, checked: boolean) {
    if (!editingAgent) return;
    if (checked) {
      if (!editingAgent.skills.includes(skillId)) editingAgent.skills.push(skillId);
    } else {
      editingAgent.skills = editingAgent.skills.filter(id => id !== skillId);
    }
  }

  function editAgent(agent: Agent) {
    isCreating = false;
    editingAgent = { ...agent };
    saveLevel = agent.level || "server";
    view = 'form';
    onsubroute?.('Éditer');
  }

  function createAgent() {
    isCreating = true;
    editingAgent = {
      id: crypto.randomUUID(),
      name: 'Nouvel Agent',
      description: 'Agent spécialisé...',
      system_prompt: 'Tu es un expert en...',
      skills: [],
      tools: [],
      working_directory: 'C:\\'
    };
    saveLevel = "server";
    view = 'form';
    onsubroute?.('Nouveau');
  }

  function cancelForm() {
    view = 'list';
    editingAgent = null;
    onsubroute?.('');
  }
  
  function getSaveLevelLabel(val: string) {
    if (val === 'project') return 'Projet (Dossier .marianne, pour Git)';
    if (val === 'global') return 'Global (Préférences utilisateur)';
    return 'Serveur (Défaut, stockage global)';
  }

  async function saveAgent() {
    if (!editingAgent) return;
    try {
      await apiClient.saveAgent(editingAgent, saveLevel as SaveLevel);
      await loadAgents();
      errorMsg = null;
      view = 'list';
      editingAgent = null;
      onsubroute?.('');
    } catch (e) {
      console.error(e);
      errorMsg = e instanceof Error ? e.message : 'Une erreur est survenue.';
    }
  }

  function deleteAgent(id: string) {
    pendingDeleteId = id;
  }

  async function confirmDelete() {
    if (!pendingDeleteId) return;
    try {
      await apiClient.deleteAgent(pendingDeleteId);
      await loadAgents();
      errorMsg = null;
    } catch (e) {
      console.error(e);
      errorMsg = e instanceof Error ? e.message : 'Une erreur est survenue.';
    } finally {
      pendingDeleteId = null;
    }
  }

  function doSelectAgent(agent: Agent) {
    if (onselect) onselect(agent);
  }
</script>

<div class="bg-card text-card-foreground border rounded-xl p-6 mb-6 shadow-md">
  {#if view === 'list'}
  {#if errorMsg}
    <div class="mb-4 p-3 rounded-md bg-destructive/10 text-destructive text-sm flex justify-between items-center" role="alert">
      <span>{errorMsg}</span>
      <button class="ml-2 text-destructive/70 hover:text-destructive" aria-label="Fermer l'erreur" onclick={() => errorMsg = null}>✕</button>
    </div>
  {/if}
  <div class="flex justify-between items-center mb-6" transition:fade={{ duration: 200, easing: cubicOut }}>
    <h3 class="m-0 font-medium text-lg">Agents Spécialisés</h3>
    <Button onclick={createAgent}>+ Nouvel Agent</Button>
  </div>
  
  {#if loading}
    <div class="py-8 text-center text-muted-foreground animate-pulse">Chargement des agents...</div>
  {:else if agents.length === 0}
    <div class="py-12 text-center text-muted-foreground italic flex flex-col items-center gap-2" transition:fade>
      <span class="text-4xl opacity-30" aria-hidden="true">🤖</span>
      <span>Aucun agent configuré.</span>
      <span class="text-xs">Cliquez sur <strong>+ Nouvel Agent</strong> pour commencer.</span>
    </div>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4" transition:fade={{ duration: 300 }}>
      {#each agents as agent}
        <div class="bg-muted/30 border rounded-lg p-4 flex flex-col justify-between transition-all hover:-translate-y-1 hover:shadow-lg">
          <div class="mb-4">
            <div class="flex items-center gap-2 mb-2">
              <h4 class="m-0 font-semibold text-lg">{agent.name}</h4>
              {#if agent.level}
                <span class="text-xs px-2 py-0.5 rounded-full font-medium whitespace-nowrap {agent.level === 'global' ? 'bg-blue-100 text-blue-600 border border-blue-200' : agent.level === 'project' ? 'bg-green-100 text-green-600 border border-green-200' : 'bg-gray-100 text-gray-600 border border-gray-200'}">
                  {agent.level === 'global' ? '🌐 Global' : agent.level === 'project' ? '📁 Projet' : '🖥️ Serveur'}
                </span>
              {/if}
            </div>
            <p class="text-sm text-muted-foreground mb-4">{agent.description}</p>
            <div class="flex flex-wrap gap-1.5 mb-2">
              {#each agent.tools as tool}
                <span class="text-xs px-2 py-1 rounded bg-primary/10 text-primary border border-primary/20">{tool}</span>
              {/each}
            </div>
          </div>
          <div class="flex gap-2 justify-end">
            <Tooltip.Provider>
              <Tooltip.Root>
                <Tooltip.Trigger>
                  <Button variant="ghost" size="icon" aria-label="Discuter avec cet agent" onclick={() => doSelectAgent(agent)}>💬</Button>
                </Tooltip.Trigger>
                <Tooltip.Content>Discuter avec cet agent</Tooltip.Content>
              </Tooltip.Root>
            </Tooltip.Provider>

            <Tooltip.Provider>
              <Tooltip.Root>
                <Tooltip.Trigger>
                  <Button variant="ghost" size="icon" aria-label="Éditer l'agent" onclick={() => editAgent(agent)}>✏️</Button>
                </Tooltip.Trigger>
                <Tooltip.Content>Éditer</Tooltip.Content>
              </Tooltip.Root>
            </Tooltip.Provider>
            
            <Tooltip.Provider>
              <Tooltip.Root>
                <Tooltip.Trigger>
                  <Button variant="ghost" size="icon" aria-label="Supprimer l'agent" class="text-destructive hover:text-destructive hover:bg-destructive/10" onclick={() => deleteAgent(agent.id)}>🗑️</Button>
                </Tooltip.Trigger>
                <Tooltip.Content>Supprimer</Tooltip.Content>
              </Tooltip.Root>
            </Tooltip.Provider>
          </div>
        </div>
      {/each}
    </div>
  {/if}
  {:else}
  <!-- Full-page form -->
  <div class="max-w-2xl mx-auto py-4">
    {#if errorMsg}
      <div class="mb-4 p-3 rounded-md bg-destructive/10 text-destructive text-sm flex justify-between items-center" role="alert">
        <span>{errorMsg}</span>
        <button class="ml-2 text-destructive/70 hover:text-destructive" aria-label="Fermer l'erreur" onclick={() => errorMsg = null}>✕</button>
      </div>
    {/if}

    {#if editingAgent}
      <div class="grid gap-4" transition:slide={{ duration: 300, easing: cubicOut }}>
        <!-- Nom -->
        <div class="grid gap-2">
          <Label for="agent-name">Nom</Label>
          <Input id="agent-name" bind:value={editingAgent.name} placeholder="Nom de l'agent" />
        </div>
        <!-- Description -->
        <div class="grid gap-2">
          <Label for="agent-desc">Description</Label>
          <Input id="agent-desc" bind:value={editingAgent.description} placeholder="Courte description" />
        </div>
        <!-- Sauvegarder dans -->
        <div class="grid gap-2">
          <Label for="save-level-agent">Sauvegarder dans :</Label>
          <Select.Root type="single" bind:value={saveLevel}>
            <Select.Trigger id="save-level-agent" class="w-full">
              {getSaveLevelLabel(saveLevel)}
            </Select.Trigger>
            <Select.Content>
              <Select.Item value="server">Serveur (Défaut, stockage global)</Select.Item>
              <Select.Item value="project">Projet (Dossier .marianne, pour Git)</Select.Item>
              <Select.Item value="global">Global (Préférences utilisateur)</Select.Item>
            </Select.Content>
          </Select.Root>
        </div>
        <!-- Prompt Système -->
        <div class="grid gap-2">
          <Label for="agent-prompt">Prompt Système</Label>
          <textarea id="agent-prompt" class="flex min-h-[100px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50" bind:value={editingAgent.system_prompt} placeholder="Instructions de l'agent..." rows="5"></textarea>
        </div>
        <!-- Dossier de travail -->
        <div class="grid gap-2">
          <Label for="agent-workdir">Dossier de travail autorisé</Label>
          <Input id="agent-workdir" bind:value={editingAgent.working_directory} placeholder="Ex: C:\ (C:\ pour accès total)" />
        </div>
        <!-- Skills -->
        <div class="grid gap-2">
          <Label>Compétences (Skills)</Label>
          <div class="flex flex-col gap-3 bg-muted/50 p-4 rounded-md border">
            {#if availableSkills.length === 0}
              <div class="text-sm text-muted-foreground italic">Aucun skill disponible. Créez-en d'abord dans l'onglet Skills.</div>
            {/if}
            {#each availableSkills as skill}
              <div class="flex items-center space-x-2">
                <Switch
                  id="skill-{skill.id}"
                  checked={editingAgent.skills.includes(skill.id)}
                  onCheckedChange={(v) => toggleSkill(skill.id, v)}
                />
                <Label for="skill-{skill.id}" class="font-normal cursor-pointer">{skill.name}</Label>
              </div>
            {/each}
          </div>
        </div>
        <!-- Outils -->
        <div class="grid gap-2">
          <Label for="agent-tools">Outils Actifs (séparés par virgule)</Label>
          <Input
            id="agent-tools"
            value={editingAgent.tools.join(', ')}
            oninput={(e) => editingAgent.tools = e.currentTarget.value.split(',').map(s=>s.trim()).filter(Boolean)}
            placeholder="read_file, write_file, replace_file_content..."
          />
        </div>
        <!-- Actions -->
        <div class="flex justify-end gap-2 pt-2">
          <Button variant="outline" onclick={cancelForm}>Annuler</Button>
          <Button onclick={saveAgent}>Enregistrer</Button>
        </div>
      </div>
    {/if}
  </div>
  {/if}
</div>

<Dialog.Root open={pendingDeleteId !== null} onOpenChange={(open) => { if (!open) pendingDeleteId = null; }}>
  <Dialog.Content class="sm:max-w-[400px]">
    <Dialog.Header>
      <Dialog.Title>Confirmer la suppression</Dialog.Title>
      <Dialog.Description>
        Êtes-vous sûr de vouloir supprimer cet agent ? Cette action est irréversible.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => pendingDeleteId = null}>Annuler</Button>
      <Button variant="destructive" onclick={confirmDelete}>Supprimer</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
