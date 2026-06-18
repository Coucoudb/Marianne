<script lang="ts">
  import { onMount } from 'svelte';
  import { apiClient, type Skill, type SaveLevel } from '../lib/api';
  import { slide, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  
  import { Button } from "$lib/components/ui/button";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import * as Select from "$lib/components/ui/select";
  import * as Tooltip from "$lib/components/ui/tooltip";
  import { Label } from "$lib/components/ui/label";

  let { onselect, onsubroute } = $props<{
    onselect?: (skill: Skill) => void;
    onsubroute?: (label: string) => void;
  }>();

  let skills: Skill[] = $state([]);
  let loading = $state(true);
  let view: 'list' | 'form' = $state('list');
  
  let editingSkill: Skill | null = $state(null);
  let saveLevel: string = $state("server");
  let pendingDeleteId: string | null = $state(null);
  let isCreating = $state(false);
  let errorMsg: string | null = $state(null);

  onMount(async () => {
    await loadSkills();
  });

  async function loadSkills() {
    loading = true;
    try {
      skills = await apiClient.listSkills();
    } catch (e) {
      console.error(e);
      errorMsg = e instanceof Error ? e.message : 'Une erreur est survenue.';
    } finally {
      loading = false;
    }
  }

  function editSkill(skill: Skill) {
    isCreating = false;
    editingSkill = { ...skill };
    saveLevel = skill.level || "server";
    view = 'form';
    onsubroute?.('Éditer');
  }

  function createSkill() {
    isCreating = true;
    editingSkill = {
      id: crypto.randomUUID(),
      name: 'Nouveau Skill',
      description: 'Courte description du domaine',
      content: 'Contenu détaillé des connaissances (par ex. procédures, faits, règles)...',
      scope: ''
    };
    saveLevel = "server";
    view = 'form';
    onsubroute?.('Nouveau');
  }

  function cancelForm() {
    view = 'list';
    editingSkill = null;
    onsubroute?.('');
  }
  
  function getSaveLevelLabel(val: string) {
    if (val === 'project') return 'Projet (Dossier .marianne, pour Git)';
    if (val === 'global') return 'Global (Préférences utilisateur)';
    return 'Serveur (Défaut, stockage global)';
  }

  async function saveSkill() {
    if (!editingSkill) return;
    try {
      await apiClient.saveSkill(editingSkill, saveLevel as SaveLevel);
      await loadSkills();
      errorMsg = null;
      view = 'list';
      editingSkill = null;
      onsubroute?.('');
    } catch (e) {
      console.error(e);
      errorMsg = e instanceof Error ? e.message : 'Une erreur est survenue.';
    }
  }

  function deleteSkill(id: string) {
    pendingDeleteId = id;
  }

  async function confirmDelete() {
    if (!pendingDeleteId) return;
    try {
      await apiClient.deleteSkill(pendingDeleteId);
      await loadSkills();
      errorMsg = null;
    } catch (e) {
      console.error(e);
      errorMsg = e instanceof Error ? e.message : 'Une erreur est survenue.';
    } finally {
      pendingDeleteId = null;
    }
  }

  function doSelectSkill(skill: Skill) {
    if (onselect) onselect(skill);
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
    <h3 class="m-0 font-medium text-lg">Base de Connaissances (Skills)</h3>
    <Button onclick={createSkill}>+ Nouveau Skill</Button>
  </div>
  
  {#if loading}
    <div class="py-8 text-center text-muted-foreground animate-pulse">Chargement des skills...</div>
  {:else if skills.length === 0}
    <div class="py-12 text-center text-muted-foreground italic flex flex-col items-center gap-2" transition:fade>
      <span class="text-4xl opacity-30" aria-hidden="true">📚</span>
      <span>Aucune compétence configurée.</span>
      <span class="text-xs">Cliquez sur <strong>+ Nouveau Skill</strong> pour commencer.</span>
    </div>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4" transition:fade={{ duration: 300 }}>
      {#each skills as skill}
        <div class="bg-muted/30 border rounded-lg p-4 flex flex-col justify-between transition-all hover:-translate-y-1 hover:shadow-lg">
          <div class="mb-4">
            <div class="flex items-center gap-2 mb-2">
              <h4 class="m-0 font-semibold text-lg">{skill.name}</h4>
              {#if skill.level}
                <span class="text-xs px-2 py-0.5 rounded-full font-medium whitespace-nowrap {skill.level === 'global' ? 'bg-blue-100 text-blue-600 border border-blue-200' : skill.level === 'project' ? 'bg-green-100 text-green-600 border border-green-200' : 'bg-gray-100 text-gray-600 border border-gray-200'}">
                  {skill.level === 'global' ? '🌐 Global' : skill.level === 'project' ? '📁 Projet' : '🖥️ Serveur'}
                </span>
              {/if}
            </div>
            <p class="text-sm text-muted-foreground mb-2">{skill.description}</p>
            {#if skill.scope}
              <div class="text-xs text-muted-foreground mb-2">🎯 {skill.scope}</div>
            {/if}
            <div class="text-xs text-muted-foreground font-mono bg-background p-2 rounded border mb-2 line-clamp-3">
              {skill.content}
            </div>
          </div>
          <div class="flex gap-2 justify-end">
            <Tooltip.Provider>
              <Tooltip.Root>
                <Tooltip.Trigger>
                  <Button variant="ghost" size="icon" aria-label="Éditer la compétence" onclick={() => editSkill(skill)}>✏️</Button>
                </Tooltip.Trigger>
                <Tooltip.Content>Éditer</Tooltip.Content>
              </Tooltip.Root>
            </Tooltip.Provider>
            
            <Tooltip.Provider>
              <Tooltip.Root>
                <Tooltip.Trigger>
                  <Button variant="ghost" size="icon" aria-label="Supprimer la compétence" class="text-destructive hover:text-destructive hover:bg-destructive/10" onclick={() => deleteSkill(skill.id)}>🗑️</Button>
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

    {#if editingSkill}
      <div class="grid gap-4" transition:slide={{ duration: 300, easing: cubicOut }}>
        <div class="grid gap-2">
          <Label for="skill-name">Nom</Label>
          <Input id="skill-name" bind:value={editingSkill.name} placeholder="Nom du skill" />
        </div>
        <div class="grid gap-2">
          <Label for="skill-desc">Description</Label>
          <Input id="skill-desc" bind:value={editingSkill.description} placeholder="Courte description" />
        </div>
        <div class="grid gap-2">
          <Label for="skill-scope">Scope (Chargement Contextuel)</Label>
          <Input id="skill-scope" bind:value={editingSkill.scope} placeholder="Ex: **/*.rs (laisser vide pour toujours charger)" />
        </div>
        <div class="grid gap-2">
          <Label for="save-level-skill">Sauvegarder dans :</Label>
          <Select.Root type="single" bind:value={saveLevel}>
            <Select.Trigger id="save-level-skill" class="w-full">
              {getSaveLevelLabel(saveLevel)}
            </Select.Trigger>
            <Select.Content>
              <Select.Item value="server">Serveur (Défaut, stockage global)</Select.Item>
              <Select.Item value="project">Projet (Dossier .marianne, pour Git)</Select.Item>
              <Select.Item value="global">Global (Préférences utilisateur)</Select.Item>
            </Select.Content>
          </Select.Root>
        </div>
        <div class="grid gap-2">
          <Label for="skill-content">Contenu de Connaissances</Label>
          <textarea id="skill-content" class="flex min-h-[150px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50" bind:value={editingSkill.content} placeholder="Texte de connaissances que l'agent lira..." rows="10"></textarea>
        </div>
        <div class="flex justify-end gap-2 pt-2">
          <Button variant="outline" onclick={cancelForm}>Annuler</Button>
          <Button onclick={saveSkill}>Enregistrer</Button>
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
        Êtes-vous sûr de vouloir supprimer cette compétence ? Les agents qui l'utilisent ne l'auront plus.
      </Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => pendingDeleteId = null}>Annuler</Button>
      <Button variant="destructive" onclick={confirmDelete}>Supprimer</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
