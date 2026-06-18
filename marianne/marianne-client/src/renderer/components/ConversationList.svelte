<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import MessageSquareIcon from '@lucide/svelte/icons/message-square';
  import FolderIcon from '@lucide/svelte/icons/folder';
  import BlocksIcon from '@lucide/svelte/icons/blocks';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import PlusIcon from '@lucide/svelte/icons/plus';

  export let conversations: Array<{
    id: string;
    preview: string;
    timestamp: number;
    messageCount: number;
  }> = [];
  export let activeConversationId: string | null = null;
  export let collapsed = false;
  export let currentView: 'chat' | 'agents' | 'skills' | 'settings' = 'chat';

  const dispatch = createEventDispatcher<{
    select: string;
    new: void;
    toggle: void;
    navigate: 'chat' | 'agents' | 'skills' | 'settings';
  }>();

  function formatRelativeTime(timestamp: number): string {
    const now = Date.now();
    const diff = now - timestamp;
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return 'À l\'instant';
    if (minutes < 60) return `il y a ${minutes}min`;
    if (hours < 24) return `il y a ${hours}h`;
    if (days === 1) return 'Hier';
    if (days < 7) return `il y a ${days}j`;
    return new Date(timestamp).toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' });
  }

  function truncate(text: string, len: number): string {
    return text.length > len ? text.slice(0, len) + '…' : text;
  }
</script>

{#if !collapsed}
  <aside class="sidebar" aria-label="Conversations">
    <div class="sidebar-header">
      <span class="sidebar-title">Conversations</span>
      <button
        class="sidebar-toggle"
        on:click={() => dispatch('toggle')}
        title="Réduire"
        aria-label="Réduire la barre latérale"
      >
        ◀
      </button>
    </div>

    <button
      class="new-conversation-btn"
      on:click={() => dispatch('new')}
      aria-label="Nouvelle conversation"
    >
      <PlusIcon size={16} />
      Nouvelle conversation
    </button>

    <div class="nav-links">
      <button class="nav-link" class:active={currentView === 'chat'} on:click={() => dispatch('navigate', 'chat')}>
        <MessageSquareIcon size={16} />
        Discussions
      </button>
      <button class="nav-link" class:active={currentView === 'agents'} on:click={() => dispatch('navigate', 'agents')}>
        <FolderIcon size={16} />
        Projets Agents
      </button>
      <button class="nav-link" class:active={currentView === 'skills'} on:click={() => dispatch('navigate', 'skills')}>
        <BlocksIcon size={16} />
        Skills
      </button>
      <button class="nav-link" class:active={currentView === 'settings'} on:click={() => dispatch('navigate', 'settings')}>
        <SettingsIcon size={16} />
        Personnaliser
      </button>
    </div>

    {#if currentView === 'chat'}
    <div class="conversation-list">
      {#each conversations as conv, i (conv.id)}
        <button
          class="conversation-item"
          class:active={conv.id === activeConversationId}
          on:click={() => dispatch('select', conv.id)}
          style="animation-delay: {i * 30}ms"
          title={conv.preview}
        >
          <div class="conv-preview">{truncate(conv.preview, 50)}</div>
          <div class="conv-meta">
            <span class="conv-time">{formatRelativeTime(conv.timestamp)}</span>
            <span class="conv-count">{conv.messageCount} msg</span>
          </div>
        </button>
      {:else}
        <div class="empty-conversations">
          <span class="empty-icon">💬</span>
          <span class="empty-text">Aucune conversation</span>
        </div>
      {/each}
    </div>
    {/if}
  </aside>
{:else}
  <div class="sidebar-collapsed">
    <button
      class="sidebar-expand"
      on:click={() => dispatch('toggle')}
      title="Afficher les conversations"
      aria-label="Afficher la barre latérale"
    >
      ▶
    </button>
  </div>
{/if}

<style>
  .sidebar {
    width: var(--sidebar-width);
    min-width: var(--sidebar-width);
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-light);
    display: flex;
    flex-direction: column;
    animation: slideInLeft var(--transition-smooth) ease-out;
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-md) var(--spacing-lg);
    border-bottom: 1px solid var(--border-light);
  }

  .sidebar-title {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .sidebar-toggle {
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    border-radius: var(--radius-xs);
    cursor: pointer;
    font-size: 0.625rem;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: var(--transition-fast);
  }

  .sidebar-toggle:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    transform: none;
    box-shadow: none;
  }

  .new-conversation-btn {
    margin: var(--spacing-md) var(--spacing-sm) var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-color);
    border-radius: 20px;
    cursor: pointer;
    font-size: 0.875rem;
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    transition: var(--transition-fast);
    font-family: var(--font-family);
  }

  .new-conversation-btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .nav-links {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 var(--spacing-sm) var(--spacing-md);
  }

  .nav-link {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-sm) var(--spacing-md);
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font-size: 0.875rem;
    cursor: pointer;
    transition: var(--transition-fast);
    font-family: var(--font-family);
  }

  .nav-link:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .nav-link.active {
    background: var(--surface-2);
    color: var(--text-primary);
    font-weight: 500;
  }

  .conversation-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-xs) var(--spacing-sm);
  }

  .conversation-item {
    width: 100%;
    text-align: left;
    padding: var(--spacing-sm) var(--spacing-md);
    margin-bottom: 2px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: var(--transition-fast);
    animation: fadeIn var(--transition-smooth) ease-out backwards;
    font-family: var(--font-family);
    display: block;
  }

  .conversation-item:hover {
    background: var(--bg-hover);
    transform: none;
    box-shadow: none;
  }

  .conversation-item.active {
    background: var(--bleu-france-subtle);
    border-left: 3px solid var(--bleu-france);
  }

  .conv-preview {
    font-size: 0.8125rem;
    color: var(--text-primary);
    line-height: 1.4;
    margin-bottom: 0.25rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .conv-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .conv-time, .conv-count {
    font-size: 0.6875rem;
    color: var(--text-tertiary);
  }

  .empty-conversations {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-2xl) var(--spacing-md);
    color: var(--text-tertiary);
  }

  .empty-icon {
    font-size: 1.5rem;
    opacity: 0.5;
  }

  .empty-text {
    font-size: 0.8125rem;
  }

  .sidebar-collapsed {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: var(--spacing-md);
    width: 40px;
    min-width: 40px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-light);
  }

  .sidebar-expand {
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    border-radius: var(--radius-xs);
    cursor: pointer;
    font-size: 0.625rem;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: var(--transition-fast);
  }

  .sidebar-expand:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    transform: none;
    box-shadow: none;
  }
</style>
