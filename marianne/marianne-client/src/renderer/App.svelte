<script lang="ts">
  import { onMount } from 'svelte';
  import { apiClient } from './lib/api';
  import type { ChatMessage } from './lib/types';
  import ChatMessages from './components/ChatMessages.svelte';
  import InputArea from './components/InputArea.svelte';
  
  let serverConfig = {
    host: 'localhost',
    port: 3000,
    protocol: 'http' as 'http' | 'https'
  };
  
  let connectionStatus: 'connected' | 'disconnected' | 'testing' = 'disconnected';
  let errorMessage = '';
  let appVersion = '';
  let showSettings = false;

  // Chat state
  let msgs: ChatMessage[] = [];
  let conversationId: string | null = null;
  let generating = false;

  onMount(async () => {
    // Load server config
    try {
      serverConfig = await window.electronAPI.server.getConfig();
      await testConnection();
    } catch (error) {
      console.error('Failed to load server config:', error);
    }

    // Get app version
    try {
      appVersion = await window.electronAPI.app.getVersion();
    } catch (error) {
      console.error('Failed to get app version:', error);
    }

    // Initialize API client
    await apiClient.init();
  });

  async function testConnection() {
    connectionStatus = 'testing';
    errorMessage = '';
    
    try {
      const result = await window.electronAPI.server.testConnection(serverConfig);
      connectionStatus = result.success ? 'connected' : 'disconnected';
      if (!result.success) {
        errorMessage = result.message || result.error || 'Échec de connexion';
      }
    } catch (error) {
      connectionStatus = 'disconnected';
      errorMessage = 'Impossible de se connecter au serveur';
    }
  }

  async function saveConfig() {
    try {
      await window.electronAPI.server.setConfig(serverConfig);
      await apiClient.init(); // Reinitialize with new config
      await testConnection();
    } catch (error) {
      errorMessage = 'Impossible de sauvegarder la configuration';
    }
  }

  async function sendMessage(prompt: string) {
    if (!prompt.trim() || generating) return;

    generating = true;

    // Add user message
    const userMsg: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: prompt
    };
    msgs = [...msgs, userMsg];

    // Add assistant message placeholder
    const assistantMsg: ChatMessage = {
      id: (Date.now() + 1).toString(),
      role: 'assistant',
      content: '',
      thinking: true
    };
    msgs = [...msgs, assistantMsg];

    let tokenBuffer = '';

    try {
      const newConvId = await apiClient.chatStream(
        conversationId,
        prompt,
        true,  // use_rag
        false, // use_web_search
        (token) => {
          // Stream token
          if (tokenBuffer === '') {
            // First token, remove thinking
            assistantMsg.thinking = false;
            assistantMsg.streaming = true;
            msgs = msgs;
          }
          tokenBuffer += token;
          assistantMsg.content = tokenBuffer;
          msgs = msgs;
        },
        (metadata) => {
          // Handle metadata events (generation-done, confidence-info, etc.)
          if (metadata.assistant_message) {
            // generation-done event
            assistantMsg.content = metadata.assistant_message;
            if (metadata.tokens_generated && metadata.generation_time_ms) {
              assistantMsg.stats = {
                tokens_generated: metadata.tokens_generated,
                time_ms: metadata.generation_time_ms
              };
            }
          }
          if (metadata.score !== undefined) {
            // confidence-info event
            // Could display confidence level in UI
          }
          if (metadata.message && metadata.status) {
            // web-search-status or offline-mode event
            if (metadata.status === 'searching') {
              assistantMsg.webBadge = { text: 'Recherche web...', kind: 'searching' };
            }
          }
          if (metadata.message && !metadata.status) {
            // contradiction-warning event
            assistantMsg.contradictionWarning = metadata.message;
          }
          msgs = msgs;
        },
        (error) => {
          errorMessage = error;
          generating = false;
        }
      );

      conversationId = newConvId;
      assistantMsg.streaming = false;
      msgs = msgs;
    } catch (error: any) {
      errorMessage = error.message || 'Erreur lors de la génération';
      // Remove assistant placeholder if error
      msgs = msgs.filter(m => m.id !== assistantMsg.id);
    } finally {
      generating = false;
    }
  }

  function newConversation() {
    conversationId = null;
    msgs = [];
  }

  $: serverUrl = `${serverConfig.protocol}://${serverConfig.host}:${serverConfig.port}`;
</script>

<div class="app">
  <header class="app-header">
    <div class="header-logo">
      <span>🇫🇷</span>
      <span>Marianne AI</span>
      {#if appVersion}
        <span class="version">v{appVersion}</span>
      {/if}
    </div>
    <div class="header-actions">
      <div class="status-indicator" class:connected={connectionStatus === 'connected'}>
        {connectionStatus === 'connected' ? '● Connecté' : '○ Déconnecté'}
      </div>
      <button class="icon-button" on:click={() => showSettings = !showSettings} title="Paramètres">
        ⚙️
      </button>
      <button class="icon-button" on:click={newConversation} title="Nouvelle conversation">
        ✨
      </button>
    </div>
  </header>

  {#if showSettings}
    <div class="settings-overlay" on:click={() => showSettings = false}>
      <div class="settings-modal" on:click|stopPropagation>
        <h2>Configuration du serveur</h2>
        
        <div class="form-group">
          <label for="protocol">Protocole</label>
          <select id="protocol" bind:value={serverConfig.protocol}>
            <option value="http">HTTP</option>
            <option value="https">HTTPS</option>
          </select>
        </div>

        <div class="form-group">
          <label for="host">Hôte</label>
          <input id="host" type="text" bind:value={serverConfig.host} placeholder="localhost" />
        </div>

        <div class="form-group">
          <label for="port">Port</label>
          <input id="port" type="number" bind:value={serverConfig.port} placeholder="3000" />
        </div>

        <div class="server-url">
          <strong>URL:</strong> {serverUrl}
        </div>

        <div class="button-group">
          <button on:click={testConnection} disabled={connectionStatus === 'testing'}>
            {connectionStatus === 'testing' ? 'Test en cours...' : 'Tester'}
          </button>
          <button on:click={saveConfig} class="primary">
            Sauvegarder
          </button>
          <button on:click={() => showSettings = false}>
            Fermer
          </button>
        </div>

        {#if errorMessage}
          <div class="error-message">{errorMessage}</div>
        {/if}
      </div>
    </div>
  {/if}

  <main class="app-main">
    <ChatMessages {msgs} />
    <InputArea on:send={(e) => sendMessage(e.detail)} disabled={generating || connectionStatus !== 'connected'} />
  </main>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--spacing-lg);
    background: var(--bg-secondary);
    height: 60px;
    box-shadow: var(--shadow-sm);
    position: relative;
  }

  .app-header::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    background: linear-gradient(90deg,
      var(--bleu-france) 0%, var(--bleu-france) 33.3%,
      var(--blanc) 33.3%, var(--blanc) 66.6%,
      var(--rouge-marianne) 66.6%, var(--rouge-marianne) 100%
    );
  }

  .header-logo {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--bleu-france);
  }

  .version {
    font-size: 0.875rem;
    color: var(--text-secondary);
    font-weight: 400;
  }

  .status-indicator {
    padding: 0.5rem 1rem;
    border-radius: var(--radius-full);
    background: var(--error);
    color: white;
    font-size: 0.875rem;
    font-weight: 500;
  }

  .status-indicator.connected {
    background: var(--success);
  }

  .app-main {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-xl);
    padding: var(--spacing-xl);
    overflow: auto;
  }

  .config-panel, .demo-panel {
    background: var(--bg-secondary);
    border-radius: var(--radius-md);
    padding: var(--spacing-xl);
    box-shadow: var(--shadow-sm);
  }

  h2 {
    margin-bottom: var(--spacing-lg);
    color: var(--bleu-france);
  }

  .form-group {
    margin-bottom: var(--spacing-md);
  }

  label {
    display: block;
    margin-bottom: var(--spacing-xs);
    font-weight: 500;
    color: var(--text-secondary);
  }

  input, select {
    width: 100%;
    padding: 0.625rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family);
    font-size: 0.95rem;
  }

  input:focus, select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .server-url {
    margin: var(--spacing-lg) 0;
    padding: var(--spacing-md);
    background: var(--bg-primary);
    border-radius: var(--radius-sm);
    font-family: monospace;
  }

  .button-group {
    display: flex;
    gap: var(--spacing-md);
  }

  button {
    padding: 0.625rem 1.25rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    cursor: pointer;
    font-size: 0.95rem;
    transition: all 0.2s;
  }

  button:hover:not(:disabled) {
    background: var(--bg-primary);
    transform: translateY(-1px);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  button.primary {
    background: var(--bleu-france);
    color: white;
    border-color: var(--bleu-france);
  }

  button.primary:hover:not(:disabled) {
    background: var(--bleu-france-light);
  }

  .error-message {
    margin-top: var(--spacing-md);
    padding: var(--spacing-md);
    background: #fee;
    color: var(--error);
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--error);
  }

  .feature-buttons {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
    margin-bottom: var(--spacing-lg);
  }

  .info-box {
    margin-top: var(--spacing-xl);
    padding: var(--spacing-lg);
    background: var(--bg-primary);
    border-radius: var(--radius-sm);
    border-left: 3px solid var(--bleu-france);
  }

  .info-box p {
    margin-bottom: var(--spacing-sm);
  }

  .info-box p:last-child {
    margin-bottom: 0;
  }
</style>
