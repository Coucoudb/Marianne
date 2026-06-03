<script lang="ts">
  import { onMount } from 'svelte';
  import { location } from 'svelte-spa-router';
  import ProfilePage from './ProfilePage.svelte';
  import ModelsPage from './ModelsPage.svelte';
  import WebSettingsPage from '../components/WebSettingsPage.svelte';
  import { IS_TAURI } from '../lib/api';

  type Tab = 'profile' | 'models' | 'web';

  let activeTab: Tab = 'profile';

  // Parse hash parameter for active tab (e.g., /settings#models)
  onMount(() => {
    const hash = window.location.hash;
    if (hash.includes('#profile')) activeTab = 'profile';
    else if (hash.includes('#models')) activeTab = 'models';
    else if (hash.includes('#web')) activeTab = 'web';
  });

  function setTab(tab: Tab) {
    activeTab = tab;
    // Update URL hash without triggering navigation
    const baseUrl = window.location.href.split('#')[0];
    window.history.replaceState(null, '', `${baseUrl}#${tab}`);
  }
</script>

<div class="settings-page">
  <div class="settings-header">
    <h1>⚙️ Réglages</h1>
  </div>

  <div class="settings-tabs">
    <button
      class="tab-btn"
      class:active={activeTab === 'profile'}
      on:click={() => setTab('profile')}
    >
      👤 Profil
    </button>
    <button
      class="tab-btn"
      class:active={activeTab === 'models'}
      on:click={() => setTab('models')}
    >
      🤖 Modèles
    </button>
    {#if !IS_TAURI}
      <button
        class="tab-btn"
        class:active={activeTab === 'web'}
        on:click={() => setTab('web')}
      >
        🌐 Serveur
      </button>
    {/if}
  </div>

  <div class="settings-content">
    {#if activeTab === 'profile'}
      <ProfilePage />
    {:else if activeTab === 'models'}
      <ModelsPage />
    {:else if activeTab === 'web' && !IS_TAURI}
      <WebSettingsPage />
    {/if}
  </div>
</div>

<style>
  .settings-page {
    max-width: 1200px;
    margin: 0 auto;
    padding: var(--spacing-lg);
  }

  .settings-header {
    margin-bottom: var(--spacing-xl);
  }

  .settings-header h1 {
    font-size: 2rem;
    color: var(--text-primary);
    margin: 0;
  }

  .settings-tabs {
    display: flex;
    gap: var(--spacing-sm);
    border-bottom: 2px solid var(--border);
    margin-bottom: var(--spacing-xl);
  }

  .tab-btn {
    padding: var(--spacing-md) var(--spacing-lg);
    background: none;
    border: none;
    border-bottom: 3px solid transparent;
    color: var(--text-secondary);
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
    bottom: -2px;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .tab-btn.active {
    color: var(--bleu-france);
    border-bottom-color: var(--bleu-france);
    font-weight: 600;
  }

  .settings-content {
    animation: fadeIn 0.2s ease-in;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
