<script lang="ts">
  import { afterUpdate, createEventDispatcher } from 'svelte';
  import type { ChatMessage } from '../lib/types';
  import { parseMarkdown } from '../lib/markdown';
  import { formatSourceLabel } from '../lib/sources';

  export let msgs: ChatMessage[] = [];

  const dispatch = createEventDispatcher<{ drop: FileList }>();

  let messagesEl: HTMLDivElement;
  let isDragOver = false;

  afterUpdate(() => {
    scrollToBottom();
  });

  function scrollToBottom() {
    if (messagesEl) {
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
  }

  async function openUrl(url: string) {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
    } catch {
      console.error('Cannot open URL:', url);
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    isDragOver = true;
  }

  function handleDragLeave(e: DragEvent) {
    if (!messagesEl.contains(e.relatedTarget as Node)) {
      isDragOver = false;
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    if (e.dataTransfer?.files?.length) {
      dispatch('drop', e.dataTransfer.files);
    }
  }

  function badgeClass(kind: string): string {
    if (kind === 'done') return 'web-done';
    if (kind === 'empty') return 'web-empty';
    if (kind === 'offline') return 'offline';
    return '';
  }
</script>

<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div
  role="region"
  aria-label="Conversation"
  class="messages"
  class:drag-active={isDragOver}
  bind:this={messagesEl}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
>
  {#if msgs.length === 0}
    <div class="welcome-message">
      <div class="welcome-avatar">M</div>
      <h2>Bonjour, je suis Marianne</h2>
      <p>Votre assistante administrative française, 100% locale et confidentielle.</p>
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
