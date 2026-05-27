<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let generating: boolean;
  export let modelLoaded: boolean;
  export let stagedFiles: { path: string; name: string }[] = [];

  const dispatch = createEventDispatcher<{
    send: { message: string; hasFiles: boolean };
    upload: void;
    removeFile: string;
    stop: void;
  }>();

  let inputValue = '';
  let textareaEl: HTMLTextAreaElement;

  $: hasContent = inputValue.trim().length > 0 || stagedFiles.length > 0;
  $: canSend = hasContent && !generating && modelLoaded;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      trySend();
    }
  }

  function handleInput() {
    autoResize();
  }

  function autoResize() {
    if (!textareaEl) return;
    textareaEl.style.height = 'auto';
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, 120) + 'px';
  }

  function trySend() {
    if (!canSend) return;
    dispatch('send', { message: inputValue.trim(), hasFiles: stagedFiles.length > 0 });
    inputValue = '';
    // Reset textarea height
    if (textareaEl) {
      textareaEl.style.height = 'auto';
    }
  }

  function handleStop() {
    dispatch('stop');
  }

  function handleUpload() {
    dispatch('upload');
  }

  function removeFile(path: string) {
    dispatch('removeFile', path);
  }
</script>

<div class="input-area">
  {#if stagedFiles.length > 0}
    <div class="staged-files">
      {#each stagedFiles as file (file.path)}
        <div class="staged-file-chip">
          <span class="file-icon">📄</span>
          <span class="file-name" title={file.name}>{file.name}</span>
          <button class="remove-file" title="Retirer" on:click={() => removeFile(file.path)}>×</button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="input-wrapper">
    <button
      class="upload-button"
      title="Joindre un ou plusieurs documents (PDF, TXT, MD)"
      disabled={generating || !modelLoaded}
      on:click={handleUpload}
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"/>
      </svg>
    </button>

    <textarea
      bind:this={textareaEl}
      bind:value={inputValue}
      class="user-input"
      placeholder="Posez votre question administrative..."
      rows="1"
      on:keydown={handleKeydown}
      on:input={handleInput}
    ></textarea>

    {#if generating}
      <button class="send-button stop-mode" title="Arrêter la génération" on:click={handleStop}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <rect x="6" y="6" width="12" height="12" rx="2"/>
        </svg>
      </button>
    {:else}
      <button
        class="send-button"
        disabled={!canSend}
        title="Envoyer"
        on:click={trySend}
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/>
        </svg>
      </button>
    {/if}
  </div>

  <div class="input-footer">
    <span class="privacy-badge">🔒 Vos données restent sur cet appareil</span>
  </div>
</div>
