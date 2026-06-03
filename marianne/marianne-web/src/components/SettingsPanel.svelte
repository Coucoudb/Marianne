<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { IS_TAURI, getApiUrl } from '../lib/api';



  // ─── Device info ─────────────────────────────────────────────────────────
  let deviceLabel = '—';
  let activeModelName = '—';
  let modelActive = false;
  let devicePreference: 'Gpu' | 'Cpu' = 'Cpu';
  let gpuAvailable = false;
  let settingsHint = 'Appliqué au prochain démarrage';

  // ─── GPU selection ────────────────────────────────────────────────────────
  let showGpuSection = false;
  let gpuDevices: { index: number; name: string; vram_free_mb: number }[] = [];
  let gpuSelection: string = 'Auto'; // 'Auto' | 'AllGpus' | 'Specific:N'

  // ─── Installed models ─────────────────────────────────────────────────────
  interface InstalledEntry {
    model: { id: string; name: string; repo_id: string; size_mb: number };
    active: boolean;
  }
  let installedModels: InstalledEntry[] = [];
  let loadingModels = false;

  // ─── Web mode error state ─────────────────────────────────────────────────
  let webModeError = false;

  onMount(() => {
    if (!IS_TAURI) {
      loadSystemInfoWeb();
    } else {
      loadDeviceInfo();
      loadInstalledModels();
      loadDevicePreference();
    }
  });

  function formatGpuSelection(selection: string | { Specific: number }): string {
    if (typeof selection === 'string') return selection;
    if (selection && typeof selection === 'object' && 'Specific' in selection) {
      return `Specific:${selection.Specific}`;
    }
    return 'Auto';
  }

  async function loadSystemInfoWeb() {
    try {
      const apiUrl = getApiUrl();
      const response = await fetch(`${apiUrl}/api/v1/system/info`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();
      deviceLabel = data.device.label;
      activeModelName = data.model.name;
      modelActive = data.model.active;
      devicePreference = data.preference.device;
      gpuAvailable = data.device.gpu_available;
      gpuDevices = data.gpu_devices || [];
      gpuSelection = formatGpuSelection(data.preference.gpu_selection);
      webModeError = false;
    } catch (error) {
      console.error('Erreur chargement info système:', error);
      deviceLabel = 'Serveur inaccessible';
      activeModelName = '—';
      webModeError = true;
    }
  }

  async function loadDeviceInfo() {
    try {
      const info = await invoke<{ label: string }>('get_device_info');
      deviceLabel = info.label;
      const installed = await invoke<InstalledEntry[]>('list_installed_models');
      const active = installed.find(e => e.active);
      activeModelName = active ? active.model.name : 'Aucun';
    } catch {
      // silencieux
    }
  }

  async function loadDevicePreference() {
    try {
      const pref = await invoke<{ preference: 'Gpu' | 'Cpu'; gpu_available: boolean }>(
        'get_device_preference'
      );
      devicePreference = pref.preference;
      gpuAvailable = pref.gpu_available;
      if (!pref.gpu_available) {
        settingsHint = 'GPU non détecté sur cette machine';
        showGpuSection = false;
      } else if (pref.preference === 'Gpu') {
        await loadGpuDevices();
      }
    } catch {
      // silencieux
    }
  }

  async function loadGpuDevices() {
    try {
      const info = await invoke<{
        devices: { index: number; name: string; vram_free_mb: number }[];
        selection: string | { Specific: number };
      }>('list_gpu_devices');

      if (info.devices.length > 1) {
        gpuDevices = info.devices;
        showGpuSection = true;
        if (info.selection === 'Auto') gpuSelection = 'Auto';
        else if (info.selection === 'AllGpus') gpuSelection = 'AllGpus';
        else if (typeof info.selection === 'object' && info.selection.Specific !== undefined) {
          gpuSelection = `Specific:${info.selection.Specific}`;
        }
      } else {
        showGpuSection = false;
      }
    } catch {
      showGpuSection = false;
    }
  }

  async function setDevicePreference(pref: 'Gpu' | 'Cpu') {
    try {
      await invoke('set_device_preference', { preference: pref });
      devicePreference = pref;
      settingsHint = '✓ Appliqué au prochain démarrage';
      if (pref === 'Gpu') {
        await loadGpuDevices();
      } else {
        showGpuSection = false;
      }
    } catch {
      // silencieux
    }
  }

  async function setGpuSelection(value: string) {
    try {
      let selection: string | { Specific: number };
      if (value === 'Auto') selection = 'Auto';
      else if (value === 'AllGpus') selection = 'AllGpus';
      else if (value.startsWith('Specific:')) {
        selection = { Specific: parseInt(value.split(':')[1], 10) };
      } else return;
      await invoke('set_gpu_selection', { selection });
      settingsHint = '✓ Appliqué au prochain démarrage';
    } catch {
      // silencieux
    }
  }

  async function loadInstalledModels() {
    loadingModels = true;
    try {
      installedModels = await invoke<InstalledEntry[]>('list_installed_models');
    } catch {
      installedModels = [];
    } finally {
      loadingModels = false;
    }
  }

  async function activateModel(modelId: string) {
    try {
      await invoke('select_model', { modelId });
      await loadInstalledModels();
      await loadDeviceInfo();
      settingsHint = '⚠ Redémarrez pour charger le nouveau modèle';
    } catch (e) {
      alert('Erreur : ' + e);
    }
  }


</script>

<div class="settings-panel">
  <h3 class="settings-title">Paramètres</h3>

  <!-- ══════════════════════════════════════════════════════════════════════ -->
  <!-- Informations système actuelles -->
  <!-- ══════════════════════════════════════════════════════════════════════ -->
  <h4 class="settings-subtitle">Configuration active</h4>
  
  {#if !IS_TAURI && webModeError}
    <p class="settings-hint" style="text-align: center; padding: 16px 8px; color: #ef4444;">
      Impossible de contacter le serveur.<br>
      Vérifiez que marianne-server est lancé ({getApiUrl()}).
    </p>
  {:else}
    <div class="settings-item">
      <span class="settings-label">Mode d'exécution</span>
      <span class="settings-value">{deviceLabel}</span>
    </div>
    <div class="settings-item">
      <span class="settings-label">Modèle IA</span>
      <span class="settings-value">
        {activeModelName}
        {#if modelActive}
          <span style="color: #10b981; margin-left: 8px;">● Chargé</span>
        {:else}
          <span style="color: #ef4444; margin-left: 8px;">○ Non chargé</span>
        {/if}
      </span>
    </div>
  {/if}

  {#if !IS_TAURI && !webModeError}
    <div class="settings-separator"></div>
    <h4 class="settings-subtitle">Informations système</h4>
    
    {#if gpuAvailable && gpuDevices.length > 0}
      <div class="settings-item">
        <span class="settings-label">Configuration GPU</span>
        <span class="settings-value">
          {#if gpuSelection === 'Auto'}
            Auto (premier GPU détecté)
          {:else if gpuSelection === 'AllGpus'}
            Multi-GPU ({gpuDevices.length} GPU)
          {:else if gpuSelection.startsWith('Specific:')}
            GPU {gpuSelection.split(':')[1]}
          {/if}
        </span>
      </div>
      
      {#each gpuDevices as gpu}
        <div class="settings-item">
          <span class="settings-label">GPU {gpu.index}</span>
          <span class="settings-value">{gpu.name} ({gpu.vram_free_mb} Mo VRAM)</span>
        </div>
      {/each}
    {:else if gpuAvailable}
      <p class="settings-hint">GPU disponible mais non configuré</p>
    {:else}
      <p class="settings-hint">Exécution sur CPU uniquement</p>
    {/if}
  {/if}

  <div class="settings-separator"></div>

  <!-- ══════════════════════════════════════════════════════════════════════ -->
  <!-- Préférences utilisateur (éditable en Tauri, lecture seule en Web) -->
  <!-- ══════════════════════════════════════════════════════════════════════ -->
  <h4 class="settings-subtitle">Préférences</h4>
  
  {#if IS_TAURI}
    <div class="settings-item settings-item-preference">
      <span class="settings-label">Mode préféré</span>
      <div class="toggle-switch">
        <button
          class="toggle-option"
          class:active={devicePreference === 'Gpu'}
          disabled={!gpuAvailable}
          on:click={() => setDevicePreference('Gpu')}
        >
          GPU
        </button>
        <button
          class="toggle-option"
          class:active={devicePreference === 'Cpu'}
          on:click={() => setDevicePreference('Cpu')}
        >
          CPU
        </button>
      </div>
    </div>

    {#if showGpuSection}
      <div class="settings-item settings-item-preference">
        <span class="settings-label">Sélection GPU</span>
        <select
          class="settings-select"
          bind:value={gpuSelection}
          on:change={() => setGpuSelection(gpuSelection)}
        >
          <option value="Auto">Auto (premier détecté)</option>
          <option value="AllGpus">Tous les GPU ({gpuDevices.length})</option>
          {#each gpuDevices as dev}
            <option value="Specific:{dev.index}">
              {dev.name} ({dev.vram_free_mb} Mo VRAM)
            </option>
          {/each}
        </select>
      </div>
    {/if}

    <p class="settings-hint">{settingsHint}</p>
  {:else}
    <div class="settings-item">
      <span class="settings-label">Mode préféré</span>
      <span class="settings-value">{devicePreference === 'Gpu' ? 'GPU' : 'CPU'}</span>
    </div>
    <p class="settings-hint" style="text-align: center; padding-top: 8px;">
      Configuration modifiable uniquement dans l'application desktop
    </p>
  {/if}

  {#if IS_TAURI}
  <div class="settings-separator"></div>

  <!-- ══════════════════════════════════════════════════════════════════════ -->
  <!-- Gestion des modèles installés -->
  <!-- ══════════════════════════════════════════════════════════════════════ -->
  <h4 class="settings-subtitle">Modèles installés</h4>
  <div class="model-catalog">
    {#if loadingModels}
      <p class="settings-hint">Chargement...</p>
    {:else if installedModels.length === 0}
      <p class="settings-hint">Aucun modèle installé</p>
    {:else}
      {#each installedModels as entry}
        <div class="model-card" class:active={entry.active}>
          <div class="model-card-header">
            <span class="model-card-name">{entry.model.name}</span>
            <span class="model-card-badge" class:active={entry.active} class:downloaded={!entry.active}>
              {entry.active ? 'Actif' : entry.model.size_mb + ' Mo'}
            </span>
          </div>
          <div class="model-card-meta">
            <span>{entry.model.repo_id}</span>
          </div>
          <div class="model-card-actions">
            {#if !entry.active}
              <button class="model-btn primary" on:click={() => activateModel(entry.model.id)}>
                Activer
              </button>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>
  {/if}
</div>
