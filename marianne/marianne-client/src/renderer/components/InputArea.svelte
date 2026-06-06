<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let disabled = false;

  const dispatch = createEventDispatcher<{ send: string }>();

  let inputValue = '';

  function handleSubmit() {
    if (!inputValue.trim() || disabled) return;
    
    const prompt = inputValue.trim();
    inputValue = '';
    dispatch('send', prompt);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
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
        console.log('Selected files:', files);
        // TODO: Handle file extraction and attachment
      }
    } catch (error) {
      console.error('Failed to open file:', error);
    }
  }
</script>

<div class="input-area">
  <div class="input-container">
    <textarea
      bind:value={inputValue}
      on:keydown={handleKeydown}
      placeholder={disabled ? "Connectez-vous au serveur pour commencer..." : "Posez votre question..."}
      {disabled}
      rows="1"
    />
    <div class="input-actions">
      <button 
        class="attach-button" 
        on:click={handleFileAttach}
        disabled={disabled}
        title="Joindre un document"
      >
        📎
      </button>
      <button
        class="send-button"
        on:click={handleSubmit}
        disabled={disabled || !inputValue.trim()}
        title="Envoyer"
      >
        ➤
      </button>
    </div>
  </div>
</div>

<style>
  .input-area {
    padding: var(--spacing-lg);
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
  }

  .input-container {
    max-width: 1000px;
    margin: 0 auto;
    display: flex;
    gap: var(--spacing-sm);
    align-items: flex-end;
  }

  textarea {
    flex: 1;
    padding: var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-family: var(--font-family);
    font-size: 0.95rem;
    resize: none;
    min-height: 48px;
    max-height: 200px;
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  textarea:disabled {
    background: var(--bg-primary);
    cursor: not-allowed;
  }

  .input-actions {
    display: flex;
    gap: var(--spacing-sm);
  }

  button {
    width: 48px;
    height: 48px;
    border: none;
    border-radius: var(--radius-md);
    cursor: pointer;
    font-size: 1.25rem;
    transition: all 0.2s;
  }

  .attach-button {
    background: var(--bg-primary);
    color: var(--text-secondary);
  }

  .attach-button:hover:not(:disabled) {
    background: var(--border);
  }

  .send-button {
    background: var(--bleu-france);
    color: white;
  }

  .send-button:hover:not(:disabled) {
    background: var(--bleu-france-light);
    transform: translateY(-2px);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
  }
</style>
