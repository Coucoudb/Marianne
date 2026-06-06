<script lang="ts">
  import { afterUpdate } from 'svelte';
  import type { ChatMessage } from '../lib/types';
  import { parseMarkdown } from '../lib/markdown';
  import { formatSourceLabel, openUrl } from '../lib/sources';

  export let msgs: ChatMessage[] = [];

  let messagesEl: HTMLDivElement;

  afterUpdate(() => {
    scrollToBottom();
  });

  function scrollToBottom() {
    if (messagesEl) {
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
  }

  function badgeClass(kind: string): string {
    if (kind === 'done') return 'web-done';
    if (kind === 'empty') return 'web-empty';
    if (kind === 'offline') return 'offline';
    return '';
  }
</script>

<div class="messages" bind:this={messagesEl}>
  {#if msgs.length === 0}
    <div class="welcome-message">
      <div class="welcome-avatar">M</div>
      <h2>Bonjour, je suis Marianne</h2>
      <p>Votre assistante administrative française.</p>
      <p>Comment puis-je vous aider ?</p>
      <ul>
        <li>📋 Droit du travail — contrats, licenciement, congés</li>
        <li>💶 Aides sociales — CAF, RSA, APL, prime d'activité</li>
        <li>🏢 URSSAF — auto-entreprise, cotisations</li>
        <li>🏠 Logement — droits des locataires, bail, APL</li>
        <li>✉️ Courriers — rédaction de lettres officielles</li>
      </ul>
    </div>
  {:else}
    {#each msgs as msg (msg.id)}
      <div class="message {msg.role}" class:streaming={msg.streaming}>
        <div class="message-content">
          {#if msg.thinking}
            <span class="thinking">Marianne réfléchit...</span>
          {:else if msg.analyzing}
            <span class="thinking">Marianne analyse le(s) document(s)...</span>
          {:else if msg.role === 'assistant'}
            {@html parseMarkdown(msg.content)}
          {:else}
            {msg.content}
          {/if}
        </div>

        {#if msg.webBadge}
          <div class="web-search-badge">
            <span class="confidence-score {badgeClass(msg.webBadge.kind)}">
              {msg.webBadge.text}
            </span>
          </div>
        {/if}

        {#if msg.contradictionWarning}
          <div class="contradiction-badge">
            <span class="contradiction-text">{msg.contradictionWarning}</span>
          </div>
        {/if}

        {#if !msg.streaming && (msg.sources?.length || msg.stats)}
          <div class="message-footer">
            {#if msg.sources?.length}
              <div class="sources-list">
                <span class="sources-label">📚 Sources</span>
                <div class="sources-chips">
                  {#each msg.sources as url}
                    <button
                      class="source-chip"
                      title={url}
                      on:click={() => openUrl(url)}
                    >
                      {formatSourceLabel(url)}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}

            {#if msg.stats}
              <div class="generation-stats">
                <span class="stat-item">
                  <span class="stat-icon">⏱️</span>
                  {(msg.stats.time_ms / 1000).toFixed(1)}s
                </span>
                <span class="stat-item">
                  <span class="stat-icon">📝</span>
                  {msg.stats.tokens_generated} tokens
                </span>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .messages {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-lg);
    background: var(--bg-chat);
  }

  .welcome-message {
    max-width: 600px;
    margin: auto;
    padding: var(--spacing-xl);
    text-align: center;
  }

  .welcome-avatar {
    width: 64px;
    height: 64px;
    margin: 0 auto var(--spacing-lg);
    background: var(--bleu-france);
    color: white;
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 2rem;
    font-weight: 700;
  }

  .welcome-message h2 {
    color: var(--bleu-france);
    margin-bottom: var(--spacing-md);
  }

  .welcome-message ul {
    text-align: left;
    list-style: none;
    margin-top: var(--spacing-lg);
  }

  .welcome-message li {
    padding: var(--spacing-sm) 0;
  }

  .message {
    margin-bottom: var(--spacing-lg);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  .message.user {
    align-items: flex-end;
  }

  .message.user .message-content {
    background: var(--bleu-france);
    color: white;
    padding: var(--spacing-md) var(--spacing-lg);
    border-radius: var(--radius-lg);
    max-width: 70%;
  }

  .message.assistant .message-content {
    background: var(--bg-assistant);
    padding: var(--spacing-lg);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }

  .thinking {
    color: var(--text-secondary);
    font-style: italic;
  }

  .message-footer {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding-left: var(--spacing-lg);
  }

  .sources-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .sources-label {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .sources-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--spacing-xs);
  }

  .source-chip {
    padding: 0.25rem 0.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .source-chip:hover {
    background: var(--bleu-france);
    color: white;
    border-color: var(--bleu-france);
  }

  .generation-stats {
    display: flex;
    gap: var(--spacing-md);
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .stat-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .web-search-badge, .contradiction-badge {
    padding-left: var(--spacing-lg);
  }

  .confidence-score {
    padding: 0.25rem 0.5rem;
    border-radius: var(--radius-sm);
    font-size: 0.8125rem;
    font-weight: 500;
  }

  .web-done {
    background: #e8f5e9;
    color: var(--success);
  }

  .web-empty {
    background: #fff3e0;
    color: var(--warning);
  }

  .offline {
    background: #ffebee;
    color: var(--error);
  }

  .contradiction-text {
    color: var(--warning);
    font-size: 0.875rem;
  }
</style>
