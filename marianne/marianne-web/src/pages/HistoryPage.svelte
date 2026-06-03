<script lang="ts">
  import { onMount } from 'svelte';
  import * as backend from '../lib/backend';
  import type { ConversationTurn } from '../lib/types';
  import { push } from 'svelte-spa-router';

  let conversations: Array<{ id: string; preview: string; date: string; count: number }> = [];
  let selectedConvId: string | null = null;
  let selectedHistory: ConversationTurn[] = [];
  let loading = false;
  let error: string | null = null;

  onMount(() => {
    loadConversationsList();
  });

  function loadConversationsList() {
    // For now, we'll use localStorage to list conversations
    // In a real app, there would be a backend endpoint for this
    const keys = Object.keys(localStorage).filter(k => k.startsWith('marianne.conv.'));
    conversations = keys.map(key => {
      const id = key.replace('marianne.conv.', '');
      try {
        const data = JSON.parse(localStorage.getItem(key) || '[]') as ConversationTurn[];
        const lastMsg = data[data.length - 1]?.content || 'Conversation vide';
        const preview = lastMsg.slice(0, 100);
        const timestamp = data[data.length - 1]?.timestamp || 0;
        const date = timestamp ? new Date(timestamp * 1000).toLocaleDateString('fr-FR') : 'Date inconnue';
        return { id, preview, date, count: data.length };
      } catch {
        return { id, preview: 'Erreur de chargement', date: 'N/A', count: 0 };
      }
    }).sort((a, b) => b.count - a.count);
  }

  async function loadConversation(convId: string) {
    selectedConvId = convId;
    loading = true;
    error = null;
    try {
      selectedHistory = await backend.getHistory(convId);
    } catch (err) {
      error = `Erreur lors du chargement : ${err}`;
      console.error(err);
    } finally {
      loading = false;
    }
  }

  function deleteConversation(convId: string) {
    if (!confirm('Supprimer cette conversation ?')) return;
    localStorage.removeItem(`marianne.conv.${convId}`);
    if (selectedConvId === convId) {
      selectedConvId = null;
      selectedHistory = [];
    }
    loadConversationsList();
  }

  function backToList() {
    selectedConvId = null;
    selectedHistory = [];
  }

  function goToChat() {
    push('/');
  }
</script>

<section class="history-page">
  <div class="history-header">
    <button type="button" class="back-btn" on:click={goToChat}>
      ← Retour au chat
    </button>
    <h2>Historique des conversations</h2>
  </div>

  <div class="history-content">
    {#if !selectedConvId}
      <div class="conversations-list">
        {#if conversations.length === 0}
          <p class="empty-state">Aucune conversation sauvegardée</p>
        {:else}
          {#each conversations as conv}
            <div class="conv-card">
              <div class="conv-info" on:click={() => loadConversation(conv.id)} role="button" tabindex="0" on:keypress={(e) => e.key === 'Enter' && loadConversation(conv.id)}>
                <div class="conv-preview">{conv.preview}</div>
                <div class="conv-meta">
                  <span class="conv-date">{conv.date}</span>
                  <span class="conv-count">{conv.count} message(s)</span>
                </div>
              </div>
              <button class="delete-btn" on:click={() => deleteConversation(conv.id)} title="Supprimer">
                🗑️
              </button>
            </div>
          {/each}
        {/if}
      </div>
    {:else}
      <div class="conversation-detail">
        <button type="button" class="back-btn-detail" on:click={backToList}>
          ← Liste des conversations
        </button>
        {#if loading}
          <p class="loading">Chargement...</p>
        {:else if error}
          <p class="error-msg">{error}</p>
        {:else if selectedHistory.length === 0}
          <p class="empty-state">Conversation vide</p>
        {:else}
          <div class="history-messages">
            {#each selectedHistory as turn}
              <div class="history-msg" class:user={turn.role === 'user'} class:assistant={turn.role === 'assistant'}>
                <div class="msg-role">{turn.role === 'user' ? 'Vous' : 'Marianne'}</div>
                <div class="msg-content">{turn.content}</div>
                <div class="msg-time">{new Date(turn.timestamp * 1000).toLocaleString('fr-FR')}</div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</section>

<style>
  .history-page {
    padding: var(--spacing-lg);
    height: 100%;
    overflow-y: auto;
    background: var(--bg-primary);
  }

  .history-header {
    margin-bottom: var(--spacing-lg);
  }

  .back-btn, .back-btn-detail {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 0.95rem;
    cursor: pointer;
    margin-bottom: var(--spacing-sm);
    padding: var(--spacing-sm);
  }

  .back-btn:hover, .back-btn-detail:hover {
    text-decoration: underline;
  }

  h2 {
    font-size: 1.5rem;
    color: var(--text-primary);
    margin: 0;
  }

  .conversations-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .conv-card {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    background: var(--bg-secondary);
    padding: var(--spacing-md);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
  }

  .conv-info {
    flex: 1;
    cursor: pointer;
  }

  .conv-preview {
    font-size: 0.95rem;
    color: var(--text-primary);
    margin-bottom: var(--spacing-xs);
  }

  .conv-meta {
    display: flex;
    gap: var(--spacing-md);
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .delete-btn {
    background: none;
    border: none;
    font-size: 1.2rem;
    cursor: pointer;
    padding: var(--spacing-sm);
  }

  .delete-btn:hover {
    opacity: 0.7;
  }

  .empty-state {
    text-align: center;
    color: var(--text-secondary);
    padding: var(--spacing-xl);
  }

  .loading, .error-msg {
    text-align: center;
    padding: var(--spacing-lg);
  }

  .error-msg {
    color: var(--error);
  }

  .history-messages {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .history-msg {
    background: var(--bg-secondary);
    padding: var(--spacing-md);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
  }

  .history-msg.user {
    border-left: 3px solid var(--bleu-france);
  }

  .history-msg.assistant {
    border-left: 3px solid var(--rouge-marianne);
  }

  .msg-role {
    font-weight: 600;
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: var(--spacing-xs);
  }

  .msg-content {
    color: var(--text-primary);
    line-height: var(--line-height-base);
    white-space: pre-wrap;
  }

  .msg-time {
    margin-top: var(--spacing-sm);
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
</style>
