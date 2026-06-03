<script lang="ts">
  import { DEFAULT_API_URL, getApiUrl, isValidHttpUrl, resetApiUrl, setApiUrl } from '../lib/api';

  let apiUrl = getApiUrl();
  let saving = false;
  let testing = false;
  let statusType: 'info' | 'success' | 'error' = 'info';
  let statusText =
    "Configurez l'URL de marianne-server. Cette valeur est sauvegardée localement dans votre navigateur.";

  function onSave() {
    const candidate = apiUrl.trim();
    if (!isValidHttpUrl(candidate)) {
      statusType = 'error';
      statusText = "URL invalide. Utilisez un endpoint HTTP(S), par exemple http://localhost:3000";
      return;
    }

    saving = true;
    try {
      setApiUrl(candidate);
      apiUrl = getApiUrl();
      statusType = 'success';
      statusText = `URL API enregistrée: ${apiUrl}`;
    } catch (e) {
      statusType = 'error';
      statusText = `Impossible d'enregistrer l'URL: ${e}`;
    } finally {
      saving = false;
    }
  }

  function onReset() {
    resetApiUrl();
    apiUrl = getApiUrl();
    statusType = 'info';
    statusText = `URL réinitialisée sur la valeur par défaut: ${apiUrl}`;
  }

  async function onTestConnection() {
    const candidate = apiUrl.trim();
    if (!isValidHttpUrl(candidate)) {
      statusType = 'error';
      statusText = "URL invalide. Utilisez un endpoint HTTP(S), par exemple http://localhost:3000";
      return;
    }

    testing = true;
    statusType = 'info';
    statusText = `Test de connexion vers ${candidate}...`;

    const ctrl = new AbortController();
    const timeout = window.setTimeout(() => ctrl.abort(), 4500);
    try {
      const res = await fetch(`${candidate.replace(/\/+$/, '')}/health`, {
        method: 'GET',
        signal: ctrl.signal,
      });
      if (!res.ok) {
        statusType = 'error';
        statusText = `Connexion échouée: HTTP ${res.status}`;
        return;
      }
      const text = (await res.text()).trim();
      if (text.toLowerCase() === 'ok') {
        statusType = 'success';
        statusText = `Connexion validée avec ${candidate}`;
      } else {
        statusType = 'error';
        statusText = `Endpoint /health inattendu: ${text || '(vide)'}`;
      }
    } catch {
      statusType = 'error';
      statusText = 'Connexion impossible. Vérifiez que le serveur est lancé et accessible.';
    } finally {
      window.clearTimeout(timeout);
      testing = false;
    }
  }
</script>

<section class="web-settings-page">
  <div class="page-header">
    <h2>🌐 Configuration du serveur</h2>
    <p class="page-subtitle">Définissez l'URL de marianne-server pour l'interface web</p>
  </div>

  <div class="web-settings-card">
    <label for="api-url" class="web-settings-label">Endpoint marianne-server</label>
    <input
      id="api-url"
      class="web-settings-input"
      type="url"
      placeholder="http://localhost:3000"
      bind:value={apiUrl}
      spellcheck="false"
      autocomplete="off"
    />
    <p class="web-settings-help">
      Exemple local: http://localhost:3000
      <br>
      Exemple distant: https://marianne.example.fr
    </p>

    <div class="web-settings-actions">
      <button
        type="button"
        class="web-settings-btn ghost"
        disabled={saving || testing}
        on:click={onReset}
      >
        Réinitialiser
      </button>
      <button
        type="button"
        class="web-settings-btn"
        disabled={saving || testing}
        on:click={onTestConnection}
      >
        {testing ? 'Test en cours...' : 'Tester la connexion'}
      </button>
      <button
        type="button"
        class="web-settings-btn primary"
        disabled={saving || testing}
        on:click={onSave}
      >
        {saving ? 'Enregistrement...' : 'Enregistrer'}
      </button>
    </div>

    <div class="web-settings-status" class:success={statusType === 'success'} class:error={statusType === 'error'}>
      {statusText}
    </div>

    <p class="web-settings-default">URL par défaut du build: {DEFAULT_API_URL}</p>
  </div>
</section>

<style>
  .web-settings-page {
    padding: var(--spacing-lg);
    max-width: 800px;
    margin: 0 auto;
  }

  .page-header {
    margin-bottom: var(--spacing-xl);
  }

  .page-header h2 {
    font-size: 1.75rem;
    color: var(--text-primary);
    margin: 0 0 var(--spacing-sm) 0;
  }

  .page-subtitle {
    color: var(--text-secondary);
    font-size: 0.95rem;
    margin: 0;
  }

  .web-settings-card {
    background: var(--bg-secondary);
    padding: var(--spacing-xl);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
  }

  .web-settings-label {
    display: block;
    font-weight: 600;
    margin-bottom: var(--spacing-sm);
    color: var(--text-primary);
  }

  .web-settings-input {
    width: 100%;
    padding: var(--spacing-md);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 1rem;
    font-family: 'Courier New', monospace;
    background: var(--bg-primary);
  }

  .web-settings-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .web-settings-help {
    margin-top: var(--spacing-sm);
    font-size: 0.85rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .web-settings-actions {
    display: flex;
    gap: var(--spacing-sm);
    margin-top: var(--spacing-lg);
    flex-wrap: wrap;
  }

  .web-settings-btn {
    padding: var(--spacing-sm) var(--spacing-lg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.95rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .web-settings-btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--bg-hover);
  }

  .web-settings-btn.ghost {
    background: none;
    border-color: transparent;
    color: var(--text-secondary);
  }

  .web-settings-btn.ghost:hover:not(:disabled) {
    color: var(--text-primary);
    background: var(--bg-hover);
  }

  .web-settings-btn.primary {
    background: var(--bleu-france);
    color: var(--blanc);
    border-color: var(--bleu-france);
  }

  .web-settings-btn.primary:hover:not(:disabled) {
    background: var(--bleu-france-light);
  }

  .web-settings-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .web-settings-status {
    margin-top: var(--spacing-lg);
    padding: var(--spacing-md);
    border-radius: var(--radius-sm);
    background: var(--bg-chat);
    border-left: 3px solid var(--border);
    font-size: 0.9rem;
  }

  .web-settings-status.success {
    border-left-color: var(--success);
    background: #e8f5e9;
  }

  .web-settings-status.error {
    border-left-color: var(--error);
    background: #ffebee;
  }

  .web-settings-default {
    margin-top: var(--spacing-md);
    font-size: 0.8rem;
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
