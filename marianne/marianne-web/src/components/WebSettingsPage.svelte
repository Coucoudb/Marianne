<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { DEFAULT_API_URL, getApiUrl, isValidHttpUrl, resetApiUrl, setApiUrl } from '../lib/api';

  const dispatch = createEventDispatcher<{ close: void }>();

  let apiUrl = getApiUrl();
  let saving = false;
  let testing = false;
  let statusType: 'info' | 'success' | 'error' = 'info';
  let statusText =
    "Configurez l'URL de marianne-server. Cette valeur est sauvegardée localement dans votre navigateur.";

  function closePage() {
    dispatch('close');
  }

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
  <div class="web-settings-head">
    <button type="button" class="web-settings-back" on:click={closePage}>
      ← Retour au chat
    </button>
    <h2>Réglages Web</h2>
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
