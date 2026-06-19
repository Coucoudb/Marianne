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
  let pendingDeleteId: string | null = $state(null);
  let isCreating = $state(false);
  let errorMsg: string | null = $state(null);
  let saveLevel = $state<SaveLevel>('server');
  let toolsDropdownOpen = $state(false);

  const AVAILABLE_TOOLS = [
    { id: 'read_file', name: 'read_file', desc: 'Lire le contenu d\'un fichier' },
    { id: 'write_file', name: 'write_file', desc: 'Créer ou écrire dans un fichier' },
    { id: 'list_dir', name: 'list_dir', desc: 'Lister le contenu d\'un répertoire' },
    { id: 'run_command', name: 'run_command', desc: 'Exécuter une commande système' },
    { id: 'replace_file_content', name: 'replace_file_content', desc: 'Remplacer du texte dans un fichier' },
    { id: 'grep_search', name: 'grep_search', desc: 'Rechercher un motif dans des fichiers' },
  ];

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
  <div class="flex justify-between items-center mb-8 pb-4 border-b border-gray-100" transition:fade={{ duration: 200, easing: cubicOut }}>
    <div>
      <h3 class="m-0 font-bold text-2xl text-gray-900">Agents Spécialisés</h3>
      <p class="text-sm text-gray-500 mt-1">Gérez vos agents d'intelligence artificielle personnalisés</p>
    </div>
    <Button onclick={createAgent} class="text-white hover:opacity-90 shadow-md transition-all hover:scale-[1.02] font-medium" style="background-color: var(--color-bleu-france)">+ Nouvel Agent</Button>
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
  <div class="max-w-3xl mx-auto p-8 mt-4 bg-white/80 backdrop-blur-xl border border-gray-100 rounded-2xl shadow-xl transition-all">
    {#if errorMsg}
      <div class="mb-6 p-4 rounded-xl bg-red-50 text-red-700 border border-red-200 text-sm flex justify-between items-center shadow-sm" role="alert">
        <span class="font-medium">{errorMsg}</span>
        <button class="ml-2 text-red-400 hover:text-red-700 hover:bg-red-100 p-1 rounded-md transition-colors" aria-label="Fermer l'erreur" onclick={() => errorMsg = null}>✕</button>
      </div>
    {/if}

    <div class="mb-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-2">{isCreating ? 'Créer un Nouvel Agent' : 'Éditer l\'Agent'}</h2>
      <p class="text-sm text-gray-500">Configurez les paramètres, les compétences et les accès de votre agent.</p>
    </div>

    {#if editingAgent}
      <div class="grid gap-6" transition:slide={{ duration: 300, easing: cubicOut }}>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <!-- Nom -->
          <div class="grid gap-2">
            <Label for="agent-name" class="font-semibold text-gray-700">Nom de l'agent</Label>
            <Input id="agent-name" class="rounded-lg px-4 bg-gray-50/50 focus:bg-white transition-colors" bind:value={editingAgent.name} placeholder="Nom de l'agent" />
          </div>
          <!-- Sauvegarder dans -->
          <div class="grid gap-2">
            <Label for="save-level-agent" class="font-semibold text-gray-700">Emplacement de sauvegarde</Label>
            <Select.Root type="single" bind:value={saveLevel}>
              <Select.Trigger id="save-level-agent" class="w-full rounded-lg px-4 bg-gray-50/50 focus:bg-white transition-colors">
                <span class="truncate">{getSaveLevelLabel(saveLevel)}</span>
              </Select.Trigger>
              <Select.Content class="rounded-lg">
                <Select.Item value="server" class="pl-6">Serveur (Défaut, stockage global)</Select.Item>
                <Select.Item value="project" class="pl-6">Projet (Dossier .marianne)</Select.Item>
                <Select.Item value="global" class="pl-6">Global (Préférences)</Select.Item>
              </Select.Content>
            </Select.Root>
          </div>
        </div>

        <!-- Description -->
        <div class="grid gap-2">
          <Label for="agent-desc" class="font-semibold text-gray-700">Description courte</Label>
          <Input id="agent-desc" class="rounded-lg px-4 bg-gray-50/50 focus:bg-white transition-colors" bind:value={editingAgent.description} placeholder="Ex: Spécialiste en analyse de données..." />
        </div>

        <!-- Prompt Système -->
        <div class="grid gap-2">
          <Label for="agent-prompt" class="font-semibold text-gray-700">Prompt Système (Instructions)</Label>
          <textarea id="agent-prompt" class="flex min-h-[140px] w-full rounded-lg border border-input bg-gray-50/50 hover:bg-gray-50 focus:bg-white px-5 py-3 text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#000091]/30 resize-y" bind:value={editingAgent.system_prompt} placeholder="Tu es un expert en..." rows="5"></textarea>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <!-- Dossier de travail -->
          <div class="grid gap-2">
            <Label for="agent-workdir" class="font-semibold text-gray-700">Dossier de travail autorisé</Label>
            <Input id="agent-workdir" class="rounded-lg px-4 bg-gray-50/50 focus:bg-white font-mono text-sm" bind:value={editingAgent.working_directory} placeholder="Ex: C:\ (C:\ pour accès total)" />
          </div>
          <!-- Outils -->
          <div class="grid gap-2 relative">
            <Label for="agent-tools" class="font-semibold text-gray-700">Outils activés</Label>
            
            <button 
              type="button" 
              class="flex w-full items-center justify-between rounded-lg border border-input bg-gray-50/50 px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[#000091]/30 transition-colors hover:bg-white" 
              onclick={() => toolsDropdownOpen = !toolsDropdownOpen}
            >
              <span class="truncate font-mono text-gray-700">
                {editingAgent.tools.length > 0 ? editingAgent.tools.join(', ') : 'Aucun outil sélectionné'}
              </span>
              <span class="text-gray-500 text-xs ml-2">▼</span>
            </button>

            {#if toolsDropdownOpen}
              <!-- Click away overlay -->
              <div class="fixed inset-0 z-10" role="presentation" onclick={() => toolsDropdownOpen = false}></div>
              
              <!-- Dropdown content -->
              <div class="absolute top-[100%] left-0 mt-1 z-20 w-full rounded-lg border border-gray-200 bg-white p-2 shadow-xl shadow-gray-200/50">
                <div class="max-h-60 overflow-y-auto pr-1">
                  {#each AVAILABLE_TOOLS as tool}
                    <label class="flex items-center space-x-3 rounded-md px-3 py-2.5 hover:bg-gray-50 cursor-pointer transition-colors border border-transparent hover:border-gray-100 mb-1">
                      <input 
                        type="checkbox" 
                        checked={editingAgent.tools.includes(tool.id)} 
                        onchange={(e) => {
                          if (e.currentTarget.checked) {
                            editingAgent.tools = [...editingAgent.tools, tool.id];
                          } else {
                            editingAgent.tools = editingAgent.tools.filter(t => t !== tool.id);
                          }
                        }} 
                        class="h-4 w-4 rounded border-gray-300 text-[#000091] focus:ring-[#000091]" 
                      />
                      <div class="flex flex-col">
                        <span class="text-sm font-medium text-gray-900 font-mono">{tool.name}</span>
                        <span class="text-xs text-gray-500">{tool.desc}</span>
                      </div>
                    </label>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        </div>

        <!-- Skills -->
        <div class="grid gap-3">
          <Label class="font-semibold text-gray-700">Compétences (Skills)</Label>
          <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3 bg-gray-50/80 p-5 rounded-xl border border-gray-100 shadow-inner">
            {#if availableSkills.length === 0}
              <div class="text-sm text-gray-500 italic col-span-full py-4 text-center">Aucune compétence disponible. Créez-en d'abord dans l'onglet Skills.</div>
            {/if}
            {#each availableSkills as skill}
              <div class="flex items-center space-x-3 p-2 hover:bg-white rounded-lg transition-colors border border-transparent hover:border-gray-200 hover:shadow-sm">
                <Switch
                  id="skill-{skill.id}"
                  checked={editingAgent.skills.includes(skill.id)}
                  onCheckedChange={(v) => toggleSkill(skill.id, v)}
                  class="data-[state=checked]:bg-[#000091]"
                />
                <Label for="skill-{skill.id}" class="font-medium cursor-pointer text-sm truncate">{skill.name}</Label>
              </div>
            {/each}
          </div>
        </div>

        <!-- Actions -->
        <div class="flex justify-end gap-3 pt-6 mt-2 border-t border-gray-100">
          <Button variant="outline" class="hover:bg-gray-100" onclick={cancelForm}>Annuler</Button>
          <Button class="text-white hover:opacity-90 shadow-md hover:shadow-lg transition-all" style="background-color: var(--color-bleu-france)" onclick={saveAgent}>Enregistrer l'agent</Button>
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
