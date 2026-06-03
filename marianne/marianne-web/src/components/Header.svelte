<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { location, push } from 'svelte-spa-router';
  import type { StatusType, DownloadProgress } from '../lib/types';

  export let statusType: StatusType = 'loading';
  export let statusText = 'Initialisation...';
  export let downloadPct: DownloadProgress | null = null;

  function navigateTo(path: string) {
    push(path);
  }

  $: currentPath = $location;
</script>

<header class="app-header">
  <div class="header-logo">
    <span class="logo-icon">M</span>
    <h1>Marianne</h1>
    <span class="subtitle">Assistant Administratif</span>
  </div>

  <nav class="header-nav">
    <button
      class="nav-btn"
      class:active={currentPath === '/'}
      on:click={() => navigateTo('/')}
    >
      💬 Chat
    </button>
    <button
      class="nav-btn"
      class:active={currentPath === '/history'}
      on:click={() => navigateTo('/history')}
    >
      📜 Historique
    </button>
    <button
      class="nav-btn"
      class:active={currentPath === '/documents'}
      on:click={() => navigateTo('/documents')}
    >
      📄 Documents
    </button>
    <button
      class="nav-btn"
      class:active={currentPath.startsWith('/settings')}
      on:click={() => navigateTo('/settings')}
    >
      ⚙️ Réglages
    </button>
  </nav>

  <div class="header-right">
    <div class="header-status">
      <span class="status-dot" class:ready={statusType === 'ready'} class:error={statusType === 'error'}></span>
      <span class="status-text">{statusText}</span>
    </div>
  </div>
</header>

<style>
  .header-nav {
    display: flex;
    gap: var(--spacing-xs);
    align-items: center;
  }

  .nav-btn {
    padding: var(--spacing-sm) var(--spacing-md);
    background: none;
    border: none;
    color: var(--text-primary);
    font-size: 0.9rem;
    cursor: pointer;
    border-radius: var(--radius-sm);
    transition: all 0.2s;
    white-space: nowrap;
  }

  .nav-btn:hover {
    background: var(--bg-chat);
  }

  .nav-btn.active {
    background: var(--bleu-france);
    color: var(--blanc);
    font-weight: 600;
  }
</style>
