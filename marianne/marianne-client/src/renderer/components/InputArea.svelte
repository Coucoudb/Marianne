<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { apiClient } from '../lib/api';

  export let disabled = false;

  const dispatch = createEventDispatcher<{ send: { prompt: string; deepThink: boolean }; deepThinkChange: boolean }>();

  let inputValue = '';
  let attachedFile: { name: string; path: string; extractedText?: string } | null = null;
  let extracting = false;
  let extractError = '';
  export let deepThinkEnabled = false;

  function toggleDeepThink() {
    deepThinkEnabled = !deepThinkEnabled;
    dispatch('deepThinkChange', deepThinkEnabled);
  }

  function handleSubmit() {
    if ((!inputValue.trim() && !attachedFile) || disabled) return;

    let prompt = inputValue.trim();

    // If file is attached and text was extracted, prepend to message
    if (attachedFile?.extractedText) {
      const question = prompt || 'Résume ce document.';
      prompt = `[Document joint : ${attachedFile.name}]\n\n${question}`;
    }

    inputValue = '';
    attachedFile = null;
    extractError = '';
    dispatch('send', { prompt, deepThink: deepThinkEnabled });
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function autoResize(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    target.style.height = 'auto';
    target.style.height = Math.min(target.scrollHeight, 200) + 'px';
  }

  async function handleFileAttach() {
    try {
      const files = await window.electronAPI.file.openDialog({
        filters: [
          { name: 'Documents', extensions: ['pdf', 'txt', 'md', 'doc', 'docx'] },
          { name: 'Tous les fichiers', extensions: ['*'] }
        ]
      });

      if (files.length > 0) {
        const filePath = files[0];
        const fileName = filePath.split(/[\\/]/).pop() || filePath;

        attachedFile = { name: fileName, path: filePath };
        extracting = true;
        extractError = '';

        try {
          const doc = await apiClient.extractDocument(filePath, inputValue.trim() || undefined);
          attachedFile = { ...attachedFile, extractedText: doc.text };
        } catch (err: any) {
          extractError = err.message || 'Erreur d\'extraction';
          console.error('Document extraction failed:', err);
        } finally {
          extracting = false;
        }
      }
    } catch (error) {
      console.error('Failed to open file:', error);
    }
  }

  function removeFile() {
    attachedFile = null;
    extractError = '';
  }
</script>

<div class="input-area">
  {#if attachedFile}
    <div class="attached-file" class:error={!!extractError}>
      <div class="file-info">
        <span class="file-icon">
          {#if extracting}
            <span class="extracting-spinner">⏳</span>
          {:else if extractError}
            ❌
          {:else}
            📄
          {/if}
        </span>
        <div class="file-details">
          <span class="file-name">{attachedFile.name}</span>
          <span class="file-status">
            {#if extracting}
              Extraction en cours...
            {:else if extractError}
              {extractError}
            {:else}
              Prêt à envoyer
            {/if}
          </span>
        </div>
      </div>
      <button class="file-remove" on:click={removeFile} title="Retirer le document" aria-label="Retirer le document">✕</button>
    </div>
  {/if}

  <div class="input-container">
    <textarea
      bind:value={inputValue}
      on:keydown={handleKeydown}
      on:input={autoResize}
      placeholder={disabled ? "Connectez-vous au serveur pour commencer..." : attachedFile ? "Posez votre question sur le document..." : "Posez votre question à Marianne..."}
      {disabled}
      rows="1"
      id="chat-input"
    />
    <div class="input-actions">
      <button
        class="action-btn deepthink-btn"
        class:active={deepThinkEnabled}
        on:click={toggleDeepThink}
        title={deepThinkEnabled ? "DeepThink activé — cliquer pour désactiver" : "Activer DeepThink (raisonnement approfondi)"}
        aria-label="Mode DeepThink"
        type="button"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 2a7 7 0 0 1 7 7c0 2.38-1.19 4.47-3 5.74V17a2 2 0 0 1-2 2H10a2 2 0 0 1-2-2v-2.26C6.19 13.47 5 11.38 5 9a7 7 0 0 1 7-7z"/>
          <line x1="10" y1="21" x2="14" y2="21"/>
          <line x1="10" y1="17" x2="14" y2="17"/>
        </svg>
      </button>
      <button
        class="action-btn attach-btn"
        on:click={handleFileAttach}
        disabled={disabled || extracting}
        title="Joindre un document"
        aria-label="Joindre un document"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"/>
        </svg>
      </button>
      <button
        class="action-btn send-btn"
        on:click={handleSubmit}
        disabled={disabled || (!inputValue.trim() && !attachedFile?.extractedText)}
        title="Envoyer"
        aria-label="Envoyer le message"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="22" y1="2" x2="11" y2="13"/>
          <polygon points="22 2 15 22 11 13 2 9 22 2"/>
        </svg>
      </button>
    </div>
  </div>
</div>

<style>
  .input-area {
    padding: var(--spacing-md) var(--spacing-xl);
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-light);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
  }

  /* ─── ATTACHED FILE ────────────────────────────────────── */

  .attached-file {
    max-width: 860px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--bleu-france-subtle);
    border: 1px solid var(--bleu-france);
    border-radius: var(--radius-md);
    animation: slideUp var(--transition-fast) ease-out;
  }

  .attached-file.error {
    background: var(--error-soft);
    border-color: var(--error);
  }

  .file-info {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    min-width: 0;
  }

  .file-icon {
    font-size: 1.125rem;
    flex-shrink: 0;
  }

  .extracting-spinner {
    animation: pulse 1s ease-in-out infinite;
    display: inline-block;
  }

  .file-details {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .file-name {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-status {
    font-size: 0.6875rem;
    color: var(--text-secondary);
  }

  .file-remove {
    width: 24px;
    height: 24px;
    min-width: 24px;
    border: none;
    background: transparent;
    border-radius: var(--radius-xs);
    cursor: pointer;
    font-size: 0.75rem;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: var(--transition-fast);
  }

  .file-remove:hover {
    background: var(--error-soft);
    color: var(--error);
    transform: none;
    box-shadow: none;
  }

  /* ─── INPUT CONTAINER ──────────────────────────────────── */

  .input-container {
    max-width: 860px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    gap: var(--spacing-sm);
    align-items: flex-end;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: var(--spacing-xs) var(--spacing-sm);
    transition: var(--transition-fast);
  }

  .input-container:focus-within {
    border-color: var(--bleu-france);
    box-shadow: 0 0 0 3px var(--bleu-france-subtle);
  }

  textarea {
    flex: 1;
    padding: var(--spacing-sm) var(--spacing-sm);
    border: none;
    background: transparent;
    font-family: var(--font-family);
    font-size: 0.9375rem;
    resize: none;
    min-height: 40px;
    max-height: 200px;
    line-height: 1.5;
    color: var(--text-primary);
  }

  textarea::placeholder {
    color: var(--text-tertiary);
  }

  textarea:focus {
    outline: none;
  }

  textarea:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .input-actions {
    display: flex;
    gap: 0.25rem;
    padding-bottom: 0.25rem;
  }

  .action-btn {
    width: 36px;
    height: 36px;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: var(--transition-fast);
    padding: 0;
  }

  .attach-btn {
    background: transparent;
    color: var(--text-tertiary);
  }

  .attach-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
    transform: none;
    box-shadow: none;
  }

  .send-btn {
    background: var(--bleu-france);
    color: white;
  }

  .send-btn:hover:not(:disabled) {
    background: var(--bleu-france-light);
    transform: scale(1.05);
    box-shadow: 0 2px 8px rgba(0, 0, 145, 0.3);
  }

  .send-btn:active:not(:disabled) {
    transform: scale(0.95);
  }

  .action-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
    transform: none;
  }

  .deepthink-btn {
    color: var(--text-secondary);
    background: transparent;
    transition: color var(--transition-fast), background-color var(--transition-fast);
  }
  .deepthink-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
    transform: none;
    box-shadow: none;
  }
  .deepthink-btn.active {
    color: var(--bleu-france);
    background-color: var(--bleu-france-subtle);
  }
</style>
