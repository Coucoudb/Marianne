<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
    import { IS_TAURI } from '../lib/api';
    import type { DownloadProgress } from '../lib/types';

  export let downloadPct: DownloadProgress | null = null;

  // ─── Device info ─────────────────────────────────────────────────────────
  let deviceLabel = '—';
  let activeModelName = '—';
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

  // ─── HF search ───────────────────────────────────────────────────────────
  interface HfResult {
    name: string;
    repo_id: string;
    downloads: number;
    likes: number;
    description: string;
  }
  interface GgufFile {
    filename: string;
    quantization: string;
    size_mb: number;
  }
  let hfQuery = '';
  let hfResults: HfResult[] = [];
  let hfFiles: GgufFile[] | null = null; // null = show search results, [] = loading
  let hfFilesRepo = '';
  let hfFilesName = '';
  let hfSearching = false;
  let hfSearchTimeout: ReturnType<typeof setTimeout> | null = null;

  // ─── Model download in settings ──────────────────────────────────────────
  let modelDownloading = false;
  $: modelProgress = modelDownloading ? downloadPct : null;

  onMount(() => {
    if (!IS_TAURI) return;
    loadDeviceInfo();
    loadInstalledModels();
    loadDevicePreference();
  });

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

  async function deleteModel(modelId: string) {
    if (!confirm("Supprimer ce modèle ? Vous devrez le retélécharger pour l'utiliser à nouveau.")) return;
    try {
      await invoke('delete_model', { modelId });
      await loadInstalledModels();
      await loadDeviceInfo();
    } catch (e) {
      alert('Erreur lors de la suppression : ' + e);
    }
  }

  // ─── HF search ───────────────────────────────────────────────────────────
  function onHfInput() {
    if (hfSearchTimeout) clearTimeout(hfSearchTimeout);
    hfFiles = null;
    if (hfQuery.trim().length < 2) {
      hfResults = [];
      return;
    }
    hfSearchTimeout = setTimeout(() => searchHuggingFace(hfQuery.trim()), 500);
  }

  async function searchHuggingFace(query: string) {
    hfSearching = true;
    hfResults = [];
    try {
      hfResults = await invoke<HfResult[]>('search_huggingface', { query });
    } catch {
      hfResults = [];
    } finally {
      hfSearching = false;
    }
  }

  async function showGgufFiles(repoId: string, modelName: string) {
    hfFilesRepo = repoId;
    hfFilesName = modelName;
    hfFiles = []; // empty = loading state
    try {
      hfFiles = await invoke<GgufFile[]>('get_model_gguf_files', { repoId });
    } catch {
      hfFiles = [];
    }
  }

  function backToResults() {
    hfFiles = null;
  }

  async function installModel(repoId: string, filename: string, name: string) {
    if (modelDownloading) return;
    modelDownloading = true;
    try {
      await invoke('download_hf_model', { repoId, filename, name });
      // Short delay before hiding progress
      await new Promise(r => setTimeout(r, 1500));
      await loadInstalledModels();
      await loadDeviceInfo();
      hfFiles = null;
      hfQuery = '';
      hfResults = [];
    } catch (e) {
      console.error('Erreur installation modèle:', e);
    } finally {
      modelDownloading = false;
    }
  }

  function formatNumber(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
    if (n >= 1_000) return (n / 1_000).toFixed(1) + 'k';
    return String(n);
  }
</script>

<div class="settings-panel">
  {#if !IS_TAURI}
    <h3 class="settings-title">Paramètres</h3>
    <p class="settings-hint" style="text-align: center; padding: 16px 8px;">
      Les paramètres (modèles, GPU) sont disponibles<br>uniquement dans l'application desktop.
    </p>
  {:else}
  <h3 class="settings-title">Paramètres</h3>

  <!-- Device info -->
  <div class="settings-item">
    <span class="settings-label">Mode actuel</span>
    <span class="settings-value">{deviceLabel}</span>
  </div>
  <div class="settings-item">
    <span class="settings-label">Modèle IA</span>
    <span class="settings-value">{activeModelName}</span>
  </div>

  <div class="settings-separator"></div>

  <!-- Device preference -->
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
      <span class="settings-label">GPU utilisé</span>
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

  <div class="settings-separator"></div>

  <!-- Installed models -->
  <h4 class="settings-subtitle">Modèles IA installés</h4>
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
            <button class="model-btn danger" on:click={() => deleteModel(entry.model.id)}>
              Supprimer
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Model install progress (from HF download) -->
  {#if modelDownloading && modelProgress}
    <div class="model-progress" style="margin-top: 8px;">
      <div class="progress-bar">
        <div class="progress-fill" style="width: {modelProgress.percent}%"></div>
      </div>
      <span class="progress-text">
        {modelProgress.downloaded_mb}/{modelProgress.total_mb} Mo ({modelProgress.percent}%)
      </span>
    </div>
  {:else if modelDownloading}
    <div class="model-progress" style="margin-top: 8px;">
      <div class="progress-bar">
        <div class="progress-fill" style="width: 0%"></div>
      </div>
      <span class="progress-text">Démarrage...</span>
    </div>
  {/if}

  <div class="settings-separator"></div>

  <!-- HF search -->
  <h4 class="settings-subtitle">Rechercher un modèle sur HuggingFace</h4>
  <div class="hf-search-bar">
    <input
      type="text"
      class="hf-search-input"
      placeholder="Ex : mistral, llama, phi, gemma..."
      bind:value={hfQuery}
      on:input={onHfInput}
    />
  </div>

  <div class="model-catalog">
    {#if hfFiles !== null}
      <!-- GGUF file list for a model -->
      <button class="model-btn" style="margin-bottom: 8px;" on:click={backToResults}>
        ← Retour aux résultats
      </button>
      <p class="settings-hint" style="font-weight: 600; color: var(--text-primary);">
        {hfFilesName} — {hfFiles.length} fichier(s) GGUF
      </p>
      {#each hfFiles as file}
        <div class="model-card">
          <div class="model-card-header">
            <span class="model-card-name">{file.quantization}</span>
            <span class="model-card-badge not-downloaded">{file.size_mb} Mo</span>
          </div>
          <div class="model-card-desc">{file.filename}</div>
          <div class="model-card-actions">
            <button
              class="model-btn primary"
              disabled={modelDownloading}
              on:click={() => installModel(hfFilesRepo, file.filename, `${hfFilesName} (${file.quantization})`)}
            >
              {modelDownloading ? 'Installation...' : 'Installer'}
            </button>
          </div>
        </div>
      {/each}
    {:else if hfSearching}
      <p class="settings-hint">Recherche en cours...</p>
    {:else if hfResults.length === 0 && hfQuery.trim().length >= 2}
      <p class="settings-hint">Aucun modèle GGUF trouvé</p>
    {:else}
      {#each hfResults as result}
        <div class="model-card hf-result">
          <div class="model-card-header">
            <span class="model-card-name">{result.name}</span>
            <span class="model-card-badge not-downloaded">⬇ {formatNumber(result.downloads)}</span>
          </div>
          <div class="model-card-desc">{result.repo_id}</div>
          <div class="model-card-meta">
            <span>❤️ {formatNumber(result.likes)}</span>
            <span>•</span>
            <span>{result.description}</span>
          </div>
          <div class="model-card-actions">
            <button class="model-btn primary" on:click={() => showGgufFiles(result.repo_id, result.name)}>
              Voir les fichiers
            </button>
          </div>
        </div>
      {/each}
    {/if}
  </div>
  {/if}
</div>
