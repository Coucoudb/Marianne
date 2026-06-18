<script lang="ts">
  import { afterUpdate, createEventDispatcher } from 'svelte';
  import { fly, slide, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import type { ChatMessage } from '../lib/types';
  import { parseMarkdown } from '../lib/markdown';
  import { formatSourceLabel, openUrl } from '../lib/sources';

  export let msgs: ChatMessage[] = [];

  const dispatch = createEventDispatcher<{ suggest: string }>();

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

  function confidenceColor(score: number): string {
    if (score >= 0.7) return 'var(--success)';
    if (score >= 0.4) return 'var(--warning)';
    return 'var(--error)';
  }

  let expandedThinking: Set<string> = new Set();
  function toggleThinking(msgId: string) {
    if (expandedThinking.has(msgId)) {
      expandedThinking.delete(msgId);
    } else {
      expandedThinking.add(msgId);
    }
    expandedThinking = expandedThinking;
  }
  function phaseLabel(phase: string): string {
    switch (phase) {
      case 'decomposition': return '🔍 Décomposition';
      case 'thinking': return '💡 Raisonnement';
      case 'synthesis': return '✅ Synthèse';
      default: return '🧠 Réflexion';
    }
  }

  const suggestions = [
    { icon: '📋', title: 'Droit du travail', desc: 'Contrats, licenciement, congés', prompt: 'Quels sont mes droits en cas de licenciement ?' },
    { icon: '💶', title: 'Aides sociales', desc: 'CAF, RSA, APL, prime d\'activité', prompt: 'Comment faire une demande de RSA ?' },
    { icon: '🏢', title: 'URSSAF', desc: 'Auto-entreprise, cotisations', prompt: 'Comment créer une auto-entreprise ?' },
    { icon: '🏠', title: 'Logement', desc: 'Droits des locataires, bail, APL', prompt: 'Quels sont les droits d\'un locataire face à un propriétaire ?' },
    { icon: '✉️', title: 'Courriers', desc: 'Rédaction de lettres officielles', prompt: 'Aide-moi à rédiger une lettre de réclamation à ma banque.' },
    { icon: '💰', title: 'Impôts', desc: 'Déclaration, crédits d\'impôt', prompt: 'Comment déclarer mes revenus en ligne ?' },
  ];
</script>

<div class="messages" bind:this={messagesEl}>
  {#if msgs.length === 0}
    <div class="welcome-container">
      <div class="welcome-hero">
        <div class="welcome-avatar">
          <span class="avatar-letter">M</span>
          <div class="avatar-ring"></div>
        </div>
        <h2 class="welcome-title">Bonjour, je suis Marianne</h2>
        <p class="welcome-subtitle">
          Votre assistante administrative française.
          <br />Comment puis-je vous aider ?
        </p>
      </div>

      <div class="suggestions-grid">
        {#each suggestions as s, i}
          <button
            class="suggestion-card"
            on:click={() => dispatch('suggest', s.prompt)}
            style="animation-delay: {100 + i * 60}ms"
          >
            <span class="suggestion-icon">{s.icon}</span>
            <div class="suggestion-text">
              <span class="suggestion-title">{s.title}</span>
              <span class="suggestion-desc">{s.desc}</span>
            </div>
          </button>
        {/each}
      </div>
    </div>
  {:else}
    {#each msgs as msg, idx (msg.id)}
      <div class="message {msg.role}" class:streaming={msg.streaming} in:fly={{ y: 20, duration: 400, easing: cubicOut }}>
        {#if msg.role === 'assistant'}
          <div class="message-avatar">M</div>
        {/if}

        <div class="message-bubble">
          <div class="message-content">
            {#if msg.thinking}
              {#if msg.deepThink}
                <div class="deepthink-thinking">
                  <span class="deepthink-thinking-dot"></span>
                  <span class="deepthink-thinking-dot" style="animation-delay:200ms"></span>
                  <span class="deepthink-thinking-dot" style="animation-delay:400ms"></span>
                  <span class="deepthink-thinking-label" key={msg.thinkingPhase}>{msg.thinkingPhase || 'Thinking...'}</span>
                </div>
              {:else}
                <div class="thinking-indicator">
                  <span class="dot" style="animation-delay: 0ms"></span>
                  <span class="dot" style="animation-delay: 150ms"></span>
                  <span class="dot" style="animation-delay: 300ms"></span>
                </div>
              {/if}
            {:else if msg.analyzing}
              <div class="analyzing-indicator">
                <span class="analyzing-icon">📄</span>
                <span class="analyzing-text">Analyse du document en cours...</span>
              </div>
            {:else if msg.role === 'assistant'}
              {#if msg.thinkingSteps && msg.thinkingSteps.length > 0}
                <div class="deepthink-container">
                  <button
                    class="deepthink-toggle"
                    on:click={() => toggleThinking(msg.id)}
                    aria-expanded={expandedThinking.has(msg.id)}
                  >
                    <span class="deepthink-icon">🧠</span>
                    <span class="deepthink-label">DeepThink — {msg.thinkingSteps.length} étape{msg.thinkingSteps.length > 1 ? 's' : ''}</span>
                    <span class="deepthink-chevron" class:open={expandedThinking.has(msg.id)}>▼</span>
                  </button>
                  {#if expandedThinking.has(msg.id)}
                    <div class="deepthink-steps" in:slide={{ duration: 300, easing: cubicOut }}>
                      {#each msg.thinkingSteps as step}
                        <div class="deepthink-step" in:fade={{ duration: 200 }}>
                          <span class="step-phase">{phaseLabel(step.phase)}</span>
                          <p class="step-content">{step.content}</p>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
              <div class="markdown-body">
                {@html parseMarkdown(msg.content)}
              </div>
            {:else}
              {msg.content}
            {/if}
          </div>

          {#if msg.webBadge}
            <div class="badge-row">
              <span class="status-badge {badgeClass(msg.webBadge.kind)}">
                {#if msg.webBadge.kind === 'searching'}🔍{:else if msg.webBadge.kind === 'done'}✅{:else if msg.webBadge.kind === 'offline'}📡{:else}⚠️{/if}
                {msg.webBadge.text}
              </span>
            </div>
          {/if}

          {#if msg.contradictionWarning}
            <div class="badge-row">
              <span class="status-badge contradiction">
                ⚠️ {msg.contradictionWarning}
              </span>
            </div>
          {/if}

          {#if msg.confidence !== undefined}
            <div class="confidence-row">
              <div class="confidence-bar">
                <div class="confidence-fill" style="width: {msg.confidence * 100}%; background: {confidenceColor(msg.confidence)}"></div>
              </div>
              <span class="confidence-label">{Math.round(msg.confidence * 100)}% confiance</span>
            </div>
          {/if}

          {#if !msg.streaming && (msg.sources?.length || msg.stats)}
            <div class="message-footer">
              {#if msg.sources?.length}
                <div class="sources-section">
                  <span class="footer-label">📚 Sources</span>
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
                <div class="stats-row">
                  <span class="stat">⏱️ {(msg.stats.time_ms / 1000).toFixed(1)}s</span>
                  <span class="stat-sep">·</span>
                  <span class="stat">📝 {msg.stats.tokens_generated} tokens</span>
                </div>
              {/if}
            </div>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .messages {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-lg) var(--spacing-xl);
    background: var(--bg-chat);
    scroll-behavior: smooth;
  }

  /* ─── WELCOME ──────────────────────────────────────────── */

  .welcome-container {
    max-width: 640px;
    margin: var(--spacing-2xl) auto;
    animation: fadeIn var(--transition-smooth) ease-out;
  }

  .welcome-hero {
    text-align: center;
    margin-bottom: var(--spacing-2xl);
  }

  .welcome-avatar {
    width: 72px;
    height: 72px;
    margin: 0 auto var(--spacing-lg);
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .avatar-letter {
    width: 64px;
    height: 64px;
    background: linear-gradient(135deg, var(--bleu-france) 0%, var(--bleu-france-light) 100%);
    color: white;
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.75rem;
    font-weight: 700;
    position: relative;
    z-index: 1;
    box-shadow: 0 4px 16px rgba(0, 0, 145, 0.2);
  }

  .avatar-ring {
    position: absolute;
    inset: -4px;
    border-radius: var(--radius-full);
    border: 2px solid var(--bleu-france-subtle);
    animation: pulse 3s ease-in-out infinite;
  }

  .welcome-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--bleu-france);
    margin-bottom: var(--spacing-sm);
    letter-spacing: -0.02em;
  }

  .welcome-subtitle {
    color: var(--text-secondary);
    font-size: 0.9375rem;
    line-height: 1.6;
  }

  /* ─── SUGGESTION CARDS ─────────────────────────────────── */

  .suggestions-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: var(--spacing-sm);
  }

  .suggestion-card {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    padding: var(--spacing-md) var(--spacing-lg);
    background: var(--bg-secondary);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-md);
    cursor: pointer;
    text-align: left;
    transition: var(--transition-fast);
    animation: slideUp var(--transition-smooth) ease-out backwards;
    font-family: var(--font-family);
  }

  .suggestion-card:hover {
    border-color: var(--bleu-france);
    background: var(--bleu-france-subtle);
    transform: translateY(-2px);
    box-shadow: var(--shadow-md);
  }

  .suggestion-icon {
    font-size: 1.375rem;
    flex-shrink: 0;
  }

  .suggestion-text {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    min-width: 0;
  }

  .suggestion-title {
    font-weight: 600;
    font-size: 0.8125rem;
    color: var(--text-primary);
  }

  .suggestion-desc {
    font-size: 0.75rem;
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ─── MESSAGES ─────────────────────────────────────────── */

  .message {
    display: flex;
    gap: var(--spacing-sm);
    margin-bottom: var(--spacing-lg);
    max-width: 860px;
    margin-left: auto;
    margin-right: auto;
  }

  .message.user {
    flex-direction: row-reverse;
  }

  .message-avatar {
    width: 32px;
    height: 32px;
    min-width: 32px;
    background: linear-gradient(135deg, var(--bleu-france) 0%, var(--bleu-france-light) 100%);
    color: white;
    border-radius: var(--radius-full);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.8125rem;
    font-weight: 700;
    margin-top: 0.25rem;
  }

  .message-bubble {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    min-width: 0;
    max-width: 80%;
  }

  .message.user .message-bubble {
    align-items: flex-end;
  }

  .message.user .message-content {
    background: var(--bleu-france);
    color: white;
    padding: var(--spacing-sm) var(--spacing-lg);
    border-radius: var(--radius-lg) var(--radius-lg) var(--radius-xs) var(--radius-lg);
    font-size: 0.9375rem;
    line-height: 1.6;
  }

  .message.assistant .message-content {
    background: var(--bg-secondary);
    padding: var(--spacing-md) var(--spacing-lg);
    border-radius: var(--radius-xs) var(--radius-lg) var(--radius-lg) var(--radius-lg);
    box-shadow: var(--shadow-sm);
    border: 1px solid var(--border-light);
    font-size: 0.9375rem;
    line-height: 1.7;
  }

  /* ─── THINKING ANIMATION ──────────────────────────────── */

  .thinking-indicator {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: var(--spacing-xs) 0;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--bleu-france);
    animation: typingDot 1.2s ease-in-out infinite;
    display: inline-block;
  }

  /* ─── DEEPTHINK THINKING ANIMATION ────────────────────── */

  .deepthink-thinking {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: var(--spacing-xs) 0;
  }

  .deepthink-thinking-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--bleu-france, #000091);
    opacity: 0.5;
    animation: deepThinkPulse 1.2s ease-in-out infinite;
    display: inline-block;
    flex-shrink: 0;
  }

  .deepthink-thinking-label {
    font-size: 0.8125rem;
    font-style: italic;
    color: var(--bleu-france, #000091);
    opacity: 0.85;
    animation: deepThinkFade 1.5s ease-in-out;
    min-width: 120px;
  }

  @keyframes deepThinkPulse {
    0%, 100% { opacity: 0.25; transform: scale(0.8); }
    50% { opacity: 1; transform: scale(1.2); }
  }

  @keyframes deepThinkFade {
    0% { opacity: 0; transform: translateY(4px); }
    20% { opacity: 0.85; transform: translateY(0); }
    80% { opacity: 0.85; }
    100% { opacity: 0.85; }
  }

  .analyzing-indicator {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    color: var(--text-secondary);
    font-style: italic;
    font-size: 0.875rem;
  }

  .analyzing-icon {
    animation: pulse 1.5s ease-in-out infinite;
  }

  /* ─── MARKDOWN CONTENT ─────────────────────────────────── */

  .markdown-body :global(p) {
    margin-bottom: 0.75em;
  }

  .markdown-body :global(p:last-child) {
    margin-bottom: 0;
  }

  .markdown-body :global(ul), .markdown-body :global(ol) {
    margin: 0.5em 0;
    padding-left: 1.5em;
  }

  .markdown-body :global(li) {
    margin-bottom: 0.25em;
  }

  .markdown-body :global(code) {
    background: var(--bg-primary);
    padding: 0.125em 0.375em;
    border-radius: var(--radius-xs);
    font-size: 0.85em;
    font-family: var(--font-mono);
  }

  .markdown-body :global(pre) {
    background: #1e1e2e;
    color: #cdd6f4;
    padding: var(--spacing-md);
    border-radius: var(--radius-sm);
    overflow-x: auto;
    margin: 0.75em 0;
  }

  .markdown-body :global(pre code) {
    background: none;
    padding: 0;
    color: inherit;
  }

  .markdown-body :global(strong) {
    font-weight: 600;
    color: var(--text-primary);
  }

  .markdown-body :global(a) {
    color: var(--bleu-france);
    text-decoration: none;
  }

  .markdown-body :global(a:hover) {
    text-decoration: underline;
  }

  .markdown-body :global(blockquote) {
    border-left: 3px solid var(--bleu-france-subtle);
    padding-left: var(--spacing-md);
    margin: 0.75em 0;
    color: var(--text-secondary);
  }

  /* ─── BADGES ───────────────────────────────────────────── */

  .badge-row {
    display: flex;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.625rem;
    border-radius: var(--radius-full);
    font-size: 0.75rem;
    font-weight: 500;
    animation: fadeIn var(--transition-fast) ease-out;
  }

  .status-badge.web-done {
    background: var(--success-soft);
    color: var(--success);
  }

  .status-badge.web-empty {
    background: var(--warning-soft);
    color: var(--warning);
  }

  .status-badge.offline {
    background: var(--error-soft);
    color: var(--error);
  }

  .status-badge.contradiction {
    background: var(--warning-soft);
    color: var(--warning);
  }

  /* ─── CONFIDENCE BAR ───────────────────────────────────── */

  .confidence-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
  }

  .confidence-bar {
    flex: 1;
    max-width: 120px;
    height: 4px;
    background: var(--border-light);
    border-radius: var(--radius-full);
    overflow: hidden;
  }

  .confidence-fill {
    height: 100%;
    border-radius: var(--radius-full);
    transition: width 0.6s ease-out;
  }

  .confidence-label {
    font-size: 0.6875rem;
    color: var(--text-tertiary);
    font-weight: 500;
  }

  /* ─── FOOTER / SOURCES ─────────────────────────────────── */

  .message-footer {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding-top: var(--spacing-xs);
  }

  .sources-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .footer-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-tertiary);
  }

  .sources-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }

  .source-chip {
    padding: 0.1875rem 0.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-light);
    border-radius: var(--radius-full);
    font-size: 0.6875rem;
    cursor: pointer;
    transition: var(--transition-fast);
    color: var(--text-secondary);
    font-family: var(--font-family);
    font-weight: 500;
  }

  .source-chip:hover {
    background: var(--bleu-france);
    color: white;
    border-color: var(--bleu-france);
    transform: none;
    box-shadow: none;
  }

  .stats-row {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.6875rem;
    color: var(--text-tertiary);
  }

  .stat-sep {
    opacity: 0.4;
  }

  /* ─── DEEPTHINK ────────────────────────────────────────── */

  .deepthink-container {
    margin-bottom: var(--spacing-sm);
    border: 1px solid var(--bleu-france-subtle);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .deepthink-toggle {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-xs) var(--spacing-sm);
    background: var(--bleu-france-subtle);
    border: none;
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--bleu-france);
    text-align: left;
    font-family: var(--font-family);
  }
  .deepthink-toggle:hover {
    background: color-mix(in srgb, var(--bleu-france) 15%, transparent);
  }
  .deepthink-chevron {
    margin-left: auto;
    transition: transform var(--transition-fast);
    font-size: 0.7rem;
  }
  .deepthink-chevron.open {
    transform: rotate(180deg);
  }
  .deepthink-steps {
    padding: var(--spacing-sm);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
    background: var(--bg-primary);
  }
  .deepthink-step {
    padding: var(--spacing-xs);
    border-left: 2px solid var(--bleu-france);
  }
  .step-phase {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--bleu-france);
    display: block;
    margin-bottom: 2px;
  }
  .step-content {
    font-size: 0.82rem;
    color: var(--text-secondary);
    margin: 0;
    white-space: pre-wrap;
  }
</style>
