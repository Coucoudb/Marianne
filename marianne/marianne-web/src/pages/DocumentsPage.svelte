<script lang="ts">
  import { onMount } from 'svelte';
  import * as backend from '../lib/backend';
  import { IS_TAURI } from '../lib/api';
  import type { ExtractedDocument } from '../lib/types';
  import { push } from 'svelte-spa-router';
  import { parseMarkdown } from '../lib/markdown';

  let filePath = '';
  let question = 'Résume ce document.';
  let extracting = false;
  let result: ExtractedDocument | null = null;
  let error: string | null = null;

  async function handleSelectFile() {
    if (!IS_TAURI) {
      error = "La sélection de fichier n'est disponible qu'en mode desktop.";
      return;
    }
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Documents', extensions: ['pdf', 'txt', 'md', 'json'] },
        ],
      });
      if (selected && typeof selected === 'string') {
        filePath = selected;
        error = null;
      }
    } catch (err) {
      error = `Erreur lors de la sélection : ${err}`;
      console.error(err);
    }
  }

  async function handleExtract() {
    if (!filePath.trim()) {
      error = 'Veuillez sélectionner ou saisir un chemin de fichier.';
      return;
    }

    extracting = true;
    error = null;
    result = null;

    try {
      result = await backend.extractDocument({
        file_path: filePath,
        question: question || undefined,
      });
    } catch (err) {
      error = `Erreur lors de l'extraction : ${err}`;
      console.error(err);
    } finally {
      extracting = false;
    }
  }

  function goBack() {
    push('/');
  }
</script>

<section class="documents-page">
  <div class="documents-header">
    <button type="button" class="back-btn" on:click={goBack}>
      ← Retour au chat
    </button>
    <h2>Analyse de documents</h2>
  </div>

  {#if !IS_TAURI}
    <div class="info-box warning">
      ⚠️ L'analyse de documents n'est disponible qu'en mode desktop Tauri.
      <br>
      En mode web, le serveur ne peut pas accéder aux fichiers locaux de votre navigateur.
    </div>
  {/if}

  <div class="documents-form">
    <div class="form-group">
      <label for="file_path">Chemin du fichier</label>
      <div class="file-input-group">
        <input
          type="text"
          id="file_path"
          bind:value={filePath}
          placeholder="C:\Users\Marie\Documents\contrat.pdf"
          disabled={!IS_TAURI}
        />
        {#if IS_TAURI}
          <button type="button" class="select-file-btn" on:click={handleSelectFile}>
            Parcourir...
          </button>
        {/if}
      </div>
      <p class="help-text">Formats supportés : PDF, TXT, MD, JSON</p>
    </div>

    <div class="form-group">
      <label for="question">Question sur le document</label>
      <input
        type="text"
        id="question"
        bind:value={question}
        placeholder="Résume ce document."
        disabled={!IS_TAURI}
      />
    </div>

    <button
      type="button"
      class="extract-btn"
      on:click={handleExtract}
      disabled={!IS_TAURI || extracting || !filePath.trim()}
    >
      {extracting ? 'Extraction en cours...' : 'Extraire et analyser'}
    </button>

    {#if error}
      <div class="error-box">{error}</div>
    {/if}

    {#if result}
      <div class="result-box">
        <h3>Résultat de l'extraction</h3>
        <div class="result-meta">
          <div class="meta-item">
            <strong>Fichier :</strong> {result.file_name}
          </div>
          <div class="meta-item">
            <strong>Caractères :</strong> {result.char_count.toLocaleString('fr-FR')}
          </div>
        </div>
        <div class="result-content">
          <h4>Contenu extrait</h4>
          <div class="extracted-text">
            {result.text.slice(0, 2000)}{result.text.length > 2000 ? '...' : ''}
          </div>
        </div>
        <div class="result-prompt">
          <h4>Prompt généré</h4>
          <pre class="prompt-text">{result.prompt}</pre>
        </div>
      </div>
    {/if}
  </div>
</section>

<style>
  .documents-page {
    padding: var(--spacing-lg);
    max-width: 900px;
    margin: 0 auto;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-primary);
  }

  .documents-header {
    margin-bottom: var(--spacing-lg);
  }

  .back-btn {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 0.95rem;
    cursor: pointer;
    margin-bottom: var(--spacing-sm);
    padding: var(--spacing-sm);
  }

  .back-btn:hover {
    text-decoration: underline;
  }

  h2 {
    font-size: 1.5rem;
    color: var(--text-primary);
    margin: 0;
  }

  .info-box {
    padding: var(--spacing-md);
    border-radius: var(--radius-sm);
    margin-bottom: var(--spacing-lg);
    background: var(--bg-secondary);
    border-left: 4px solid var(--border);
  }

  .info-box.warning {
    border-left-color: var(--warning);
    background: #fff8e1;
  }

  .documents-form {
    background: var(--bg-secondary);
    padding: var(--spacing-lg);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
  }

  .form-group {
    margin-bottom: var(--spacing-lg);
  }

  .form-group label {
    display: block;
    font-weight: 600;
    margin-bottom: var(--spacing-sm);
    color: var(--text-primary);
  }

  .form-group input {
    width: 100%;
    padding: var(--spacing-sm) var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 0.95rem;
    font-family: inherit;
  }

  .form-group input:disabled {
    background: var(--bg-chat);
    cursor: not-allowed;
  }

  .file-input-group {
    display: flex;
    gap: var(--spacing-sm);
  }

  .file-input-group input {
    flex: 1;
  }

  .select-file-btn {
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font-size: 0.9rem;
    white-space: nowrap;
  }

  .select-file-btn:hover {
    background: var(--border-light);
  }

  .help-text {
    margin-top: var(--spacing-xs);
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .extract-btn {
    width: 100%;
    padding: var(--spacing-md) var(--spacing-lg);
    background: var(--bleu-france);
    color: var(--blanc);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }

  .extract-btn:hover:not(:disabled) {
    background: var(--bleu-france-light);
  }

  .extract-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-box {
    margin-top: var(--spacing-md);
    padding: var(--spacing-md);
    background: #ffebee;
    border-left: 4px solid var(--error);
    border-radius: var(--radius-sm);
    color: var(--error);
  }

  .result-box {
    margin-top: var(--spacing-lg);
    padding: var(--spacing-lg);
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }

  .result-box h3 {
    margin-top: 0;
    margin-bottom: var(--spacing-md);
    color: var(--text-primary);
  }

  .result-box h4 {
    margin-top: var(--spacing-md);
    margin-bottom: var(--spacing-sm);
    color: var(--text-primary);
    font-size: 1rem;
  }

  .result-meta {
    display: flex;
    gap: var(--spacing-lg);
    margin-bottom: var(--spacing-md);
  }

  .meta-item {
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .extracted-text {
    padding: var(--spacing-md);
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
    line-height: 1.6;
    white-space: pre-wrap;
    max-height: 300px;
    overflow-y: auto;
  }

  .prompt-text {
    padding: var(--spacing-md);
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-x: auto;
  }
</style>
