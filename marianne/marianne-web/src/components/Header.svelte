<script lang="ts">
  import SettingsPanel from './SettingsPanel.svelte';
  import type { StatusType, DownloadProgress } from '../lib/types';

  export let statusType: StatusType = 'loading';
  export let statusText = 'Initialisation...';
  /** Incremented by App.svelte when device info should be refreshed */
  export let refreshTick = 0;
  export let downloadPct: DownloadProgress | null = null;

  let showSettings = false;
  let headerEl: HTMLElement;

  function toggleSettings() {
    showSettings = !showSettings;
    if (showSettings) {
      // Bind click-outside listener
      setTimeout(() => {
        document.addEventListener('click', handleOutsideClick);
      }, 0);
    } else {
      document.removeEventListener('click', handleOutsideClick);
    }
  }

  function handleOutsideClick(e: MouseEvent) {
    if (headerEl && !headerEl.contains(e.target as Node)) {
      showSettings = false;
      document.removeEventListener('click', handleOutsideClick);
    }
  }

  // Trigger a refresh of the settings panel when refreshTick changes
  // by toggling a key on SettingsPanel — done simply via reactive assignment
  let panelKey = 0;
  $: if (refreshTick > 0) {
    panelKey = refreshTick;
  }
</script>

<header class="app-header" bind:this={headerEl}>
  <div class="header-logo">
    <span class="logo-icon">M</span>
    <h1>Marianne</h1>
    <span class="subtitle">Assistant Administratif</span>
  </div>

  <div class="header-right">
    <div class="header-status">
      <span class="status-dot" class:ready={statusType === 'ready'} class:error={statusType === 'error'}></span>
      <span class="status-text">{statusText}</span>
    </div>

    <button
      class="settings-button"
      class:active={showSettings}
      title="Paramètres"
      on:click|stopPropagation={toggleSettings}
    >
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06A1.65 1.65 0 0019.4 9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z"/>
      </svg>
    </button>

    {#if showSettings}
      <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-noninteractive-element-interactions -->
      <div role="presentation" on:click|stopPropagation={() => {}}>
        {#key panelKey}
          <SettingsPanel {downloadPct} />
        {/key}
      </div>
    {/if}
  </div>
</header>
