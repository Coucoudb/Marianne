<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { DownloadProgress } from '../lib/types';

  export let downloadPct: DownloadProgress | null = null;

  const dispatch = createEventDispatcher<{ download: void }>();

  let downloading = false;

  function handleDownload() {
    if (downloading) return;
    downloading = true;
    dispatch('download');
  }

  // Reset button when downloadPct becomes null (download finished or errored)
  $: if (downloadPct === null) {
    downloading = false;
  }
</script>

<div class="modal">
  <div class="modal-content">
    <h2>Configuration initiale</h2>
    <p>Marianne doit télécharger son modèle d'intelligence artificielle (~2.2 Go).</p>
    <p>Ce téléchargement n'a lieu qu'une seule fois.</p>

    {#if downloadPct !== null}
      <div class="progress-container">
        <div class="progress-bar">
          <div class="progress-fill" style="width: {downloadPct.percent}%"></div>
        </div>
        <span class="progress-text">
          {downloadPct.downloaded_mb} Mo / {downloadPct.total_mb} Mo ({downloadPct.percent}%)
        </span>
      </div>
    {/if}

    <button
      class="primary-button"
      disabled={downloading}
      on:click={handleDownload}
    >
      {downloading ? 'Téléchargement en cours...' : 'Télécharger Marianne'}
    </button>
  </div>
</div>
