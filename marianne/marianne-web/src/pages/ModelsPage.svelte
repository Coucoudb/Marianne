<script lang="ts">
  import { onMount } from 'svelte';
  import * as backend from '../lib/backend';
  import { IS_TAURI } from '../lib/api';
  import type { ModelsStatus, DownloadModelRequest } from '../lib/types';

  let modelsStatus: ModelsStatus | null = null;
  let loading = true;
  let error: string | null = null;
  let downloading = false;
  let loadingModel = false;
  let showDownloadForm = false;

  let newModel: DownloadModelRequest = {
    repo_id: '',
    filename: '',
    name: '',
  };

  onMount(async () => {
    await refreshStatus();
  });

  async function refreshStatus() {
    loading = true;
    error = null;
    try {
      modelsStatus = await backend.getModelsStatus();
    } catch (err) {
      error = `Erreur lors du chargement : ${err}`;
      console.error(err);
    } finally {
      loading = false;
    }
  }

  async function handleDownloadModel() {
    if (!newModel.repo_id || !newModel.filename || !newModel.name) {
      error = 'Veuillez renseigner tous les champs pour télécharger un modèle.';
      return;
    }

    downloading = true;
    error = null;

    try {
      const response = await backend.downloadNewModel(newModel);
      showDownloadForm = false;
      newModel = { repo_id: '', filename: '', name: '' };
      // Refresh status after a delay to show the download progress
      setTimeout(refreshStatus, 1000);
    } catch (err) {
      error = `Erreur lors du téléchargement : ${err}`;
      console.error(err);
    } finally {
      downloading = false;
    }
  }

  async function handleLoadModel(modelId: string) {
    if (!confirm(`Charger le modèle ${modelId} ? Cela déchargera le modèle actuel.`)) return;

    loadingModel = true;
    error = null;

    try {
      await backend.loadModelById(modelId);
      await refreshStatus();
    } catch (err) {
      error = `Erreur lors du chargement du modèle : ${err}`;
      console.error(err);
    } finally {
      loadingModel = false;
    }
  }

  function toggleDownloadForm() {
    showDownloadForm = !showDownloadForm;
    if (!showDownloadForm) {
      newModel = { repo_id: '', filename: '', name: '' };
      error = null;
    }
  }

  function formatSize(sizeMb: number): string {
    if (sizeMb < 1024) return `${sizeMb.toFixed(0)} Mo`;
    return `${(sizeMb / 1024).toFixed(2)} Go`;
  }
</script>

<section class="models-page">
  <div class="page-header">
    <div class="header-title">
      <h2>🤖 Gestion des modèles</h2>
      <p class="page-subtitle">Gérez les modèles IA téléchargés et chargez-en de nouveaux</p>
    </div>
    <button type="button" class="refresh-btn" on:click={refreshStatus} disabled={loading}>
      🔄 Actualiser
    </button>
  </div>

  {#if loading}
    <p class="loading">Chargement des modèles...</p>
  {:else if error && !modelsStatus}
    <div class="error-box">{error}</div>
  {:else if modelsStatus}
    <div class="models-content">
      <!-- Modèle actuellement chargé -->
      <div class="current-model-card">
        <h3>Modèle actif</h3>
        {#if modelsStatus.loaded_model}
          <div class="model-info">
            <div class="model-name">{modelsStatus.loaded_model.name}</div>
            <div class="model-device">
              Device: {modelsStatus.loaded_model.device_label}
            </div>
            <div class="model-id">ID: {modelsStatus.loaded_model.id}</div>
          </div>
        {:else}
          <p class="no-model">Aucun modèle chargé</p>
        {/if}
      </div>

      <!-- Liste des modèles téléchargés -->
      <div class="downloaded-models">
        <div class="section-header">
          <h3>Modèles téléchargés</h3>
          <button type="button" class="add-model-btn" on:click={toggleDownloadForm}>
            {showDownloadForm ? '✖ Annuler' : '+ Télécharger un modèle'}
          </button>
        </div>

        {#if showDownloadForm}
          <div class="download-form">
            <div class="form-group">
              <label for="repo_id">Repository HuggingFace</label>
              <input
                type="text"
                id="repo_id"
                bind:value={newModel.repo_id}
                placeholder="microsoft/Phi-3-mini-4k-instruct-gguf"
              />
              <p class="help-text">Exemple : microsoft/Phi-3-mini-4k-instruct-gguf</p>
            </div>

            <div class="form-group">
              <label for="filename">Nom du fichier GGUF</label>
              <input
                type="text"
                id="filename"
                bind:value={newModel.filename}
                placeholder="Phi-3-mini-4k-instruct-q4.gguf"
              />
              <p class="help-text">Exemple : Phi-3-mini-4k-instruct-q4.gguf</p>
            </div>

            <div class="form-group">
              <label for="model_name">Nom lisible</label>
              <input
                type="text"
                id="model_name"
                bind:value={newModel.name}
                placeholder="Phi-3 Mini (Q4)"
              />
            </div>

            <button
              type="button"
              class="download-submit-btn"
              on:click={handleDownloadModel}
              disabled={downloading}
            >
              {downloading ? 'Téléchargement en cours...' : 'Démarrer le téléchargement'}
            </button>
          </div>
        {/if}

        {#if error}
          <div class="error-box">{error}</div>
        {/if}

        {#if modelsStatus.downloaded_models.length === 0}
          <p class="no-models">Aucun modèle téléchargé</p>
        {:else}
          <div class="models-grid">
            {#each modelsStatus.downloaded_models as model}
              <div class="model-card" class:active={modelsStatus.loaded_model?.id === model.id}>
                <div class="model-card-header">
                  <h4>{model.name}</h4>
                  {#if modelsStatus.loaded_model?.id === model.id}
                    <span class="active-badge">Actif</span>
                  {/if}
                </div>
                <div class="model-details">
                  <div class="detail-item">
                    <strong>ID:</strong> {model.id}
                  </div>
                  <div class="detail-item">
                    <strong>Repo:</strong> {model.repo_id}
                  </div>
                  <div class="detail-item">
                    <strong>Fichier:</strong> {model.filename}
                  </div>
                  <div class="detail-item">
                    <strong>Taille:</strong> {formatSize(model.size_mb)}
                  </div>
                </div>
                {#if modelsStatus.loaded_model?.id !== model.id}
                  <button
                    type="button"
                    class="load-model-btn"
                    on:click={() => handleLoadModel(model.id)}
                    disabled={loadingModel}
                  >
                    {loadingModel ? 'Chargement...' : 'Charger ce modèle'}
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</section>

<style>
  .models-page {
    padding: var(--spacing-lg);
    max-width: 1000px;
    margin: 0 auto;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-primary);
  }

  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: var(--spacing-xl);
    gap: var(--spacing-md);
  }

  .header-title {
    flex: 1;
  }

  .page-header h2 {
    font-size: 1.75rem;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-sm) 0;
  }

  .page-subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin: 0;
  }

  .refresh-btn {
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
  }

  .refresh-btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .refresh-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .loading {
    text-align: center;
    padding: var(--spacing-xl);
    color: var(--text-secondary);
  }

  .error-box {
    margin-bottom: var(--spacing-md);
    padding: var(--spacing-md);
    background: #ffebee;
    border-left: 4px solid var(--error);
    border-radius: var(--radius-sm);
    color: var(--error);
  }

  .current-model-card {
    background: var(--bg-secondary);
    padding: var(--spacing-lg);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
    margin-bottom: var(--spacing-lg);
    border: 2px solid var(--bleu-france);
  }

  .current-model-card h3 {
    margin-top: 0;
    margin-bottom: var(--spacing-md);
    color: var(--text-primary);
  }

  .model-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .model-name {
    font-size: 1.2rem;
    font-weight: 600;
    color: var(--bleu-france);
  }

  .model-device, .model-id {
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .no-model {
    color: var(--text-secondary);
    font-style: italic;
  }

  .downloaded-models {
    background: var(--bg-secondary);
    padding: var(--spacing-lg);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-md);
  }

  .section-header h3 {
    margin: 0;
    color: var(--text-primary);
  }

  .add-model-btn {
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bleu-france);
    color: var(--blanc);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.2s;
  }

  .add-model-btn:hover {
    background: var(--bleu-france-light);
  }

  .download-form {
    background: var(--bg-primary);
    padding: var(--spacing-md);
    border-radius: var(--radius-sm);
    margin-bottom: var(--spacing-md);
    border: 1px solid var(--border);
  }

  .form-group {
    margin-bottom: var(--spacing-md);
  }

  .form-group label {
    display: block;
    font-weight: 600;
    margin-bottom: var(--spacing-xs);
    color: var(--text-primary);
    font-size: 0.9rem;
  }

  .form-group input {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 0.95rem;
    font-family: inherit;
  }

  .help-text {
    margin-top: var(--spacing-xs);
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .download-submit-btn {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--success);
    color: var(--blanc);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .download-submit-btn:hover:not(:disabled) {
    background: #007c3a;
  }

  .download-submit-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .no-models {
    text-align: center;
    padding: var(--spacing-lg);
    color: var(--text-secondary);
    font-style: italic;
  }

  .models-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: var(--spacing-md);
  }

  .model-card {
    background: var(--bg-primary);
    padding: var(--spacing-md);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    transition: box-shadow 0.2s;
  }

  .model-card:hover {
    box-shadow: var(--shadow-md);
  }

  .model-card.active {
    border-color: var(--success);
    background: #f1f8f4;
  }

  .model-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--spacing-sm);
  }

  .model-card-header h4 {
    margin: 0;
    font-size: 1rem;
    color: var(--text-primary);
  }

  .active-badge {
    background: var(--success);
    color: var(--blanc);
    padding: 2px 8px;
    border-radius: var(--radius-full);
    font-size: 0.75rem;
    font-weight: 600;
  }

  .model-details {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
    margin-bottom: var(--spacing-md);
  }

  .detail-item {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .detail-item strong {
    color: var(--text-primary);
  }

  .load-model-btn {
    width: 100%;
    padding: var(--spacing-sm);
    background: var(--bleu-france);
    color: var(--blanc);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.2s;
  }

  .load-model-btn:hover:not(:disabled) {
    background: var(--bleu-france-light);
  }

  .load-model-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
