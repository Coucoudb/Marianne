<script lang="ts">
  import { onMount } from 'svelte';
  import { apiClient } from './lib/api';
  import type { ChatMessage, UserProfile, SystemInfo, ModelsStatus } from './lib/types';
  import ChatMessages from './components/ChatMessages.svelte';
  import InputArea from './components/InputArea.svelte';
  import ConversationList from './components/ConversationList.svelte';
  import AgentsManager from './components/AgentsManager.svelte';
  import SkillsManager from './components/SkillsManager.svelte';

  let serverConfig = $state({
    host: 'localhost',
    port: 3000,
    protocol: 'http' as 'http' | 'https'
  });

  let connectionStatus: 'connected' | 'disconnected' | 'testing' = $state('disconnected');
  let errorMessage = $state('');
  let appVersion = $state('');
  let currentView: 'chat' | 'agents' | 'skills' | 'settings' = $state('chat');
  let settingsTab: 'connection' | 'profile' | 'models' = $state('profile');
  let sidebarCollapsed = $state(false);
  let agentSubroute: string = $state('');
  let skillSubroute: string = $state('');

  // Chat state
  let msgs: ChatMessage[] = $state([]);
  let conversationId: string | null = $state(null);
  let generating = $state(false);

  // Conversations list (session-local)
  let conversations: Array<{
    id: string;
    preview: string;
    timestamp: number;
    messageCount: number;
  }> = $state([]);

  // Profile state
  let profile: UserProfile = $state({
    first_name: '',
    age: null,
    professional_status: null,
    family_status: null,
    department: null,
    topics_of_interest: [],
    language_level: 'Standard',
    device_preference: 'Gpu',
    gpu_selection: 'Auto',
    selected_model: null,
    updated_at: 0
  });
  let profileLoading = $state(false);
  let profileSaved = $state(false);

  // Custom model download state
  let downloadRepo = $state('');
  let downloadFilename = $state('');
  let downloadName = $state('');
  let isDownloading = $state(false);
  let downloadInterval: any;

  // Server system info
  let systemInfo: SystemInfo | null = $state(null);

  // Models state
  let modelsStatus: ModelsStatus | null = $state(null);
  let modelsLoading = $state(false);
  let modelLoadingId: string | null = $state(null);

  // Workspace directory (bouton 📁 au-dessus du prompt)
  let activeWorkspaceDir: string | null = $state(null);
  // Agent actif (sélectionné depuis la gestion des agents)
  let activeAgent: { id: string; name: string } | null = $state(null);

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

    // Charger le dossier de travail actif depuis le serveur
    try {
      activeWorkspaceDir = await apiClient.getProjectDir();
    } catch { /* ignore si le serveur n'est pas encore connecté */ }

    // Charger les conversations persistantes depuis le serveur
    await loadConversationsFromServer();
  });

  async function loadConversationsFromServer() {
    if (connectionStatus !== 'connected') return;
    try {
      const serverConversations = await apiClient.listConversations();
      if (serverConversations.length > 0) {
        conversations = serverConversations.map((c: any) => ({
          id: c.id,
          preview: c.first_message_preview || c.title || 'Conversation',
          timestamp: (c.last_message_at || c.created_at || 0) * 1000,
          messageCount: c.message_count || 0
        }));
      }
    } catch (err) {
      console.warn('Failed to load conversations from server:', err);
    }
  }

  async function testConnection() {
    connectionStatus = 'testing';
    errorMessage = '';

    try {
      if (window.electronAPI) {
        const configSnapshot = $state.snapshot(serverConfig);
        const result = await window.electronAPI.server.testConnection(configSnapshot);
        connectionStatus = result.success ? 'connected' : 'disconnected';
        if (!result.success) {
          errorMessage = result.message || result.error || 'Échec de connexion';
        }
      } else {
        // Fallback for browser testing
        const url = `${serverConfig.protocol}://${serverConfig.host}:${serverConfig.port}/health`;
        const response = await fetch(url);
        if (response.ok) {
          connectionStatus = 'connected';
        } else {
          connectionStatus = 'disconnected';
          errorMessage = `Erreur HTTP: ${response.status}`;
        }
      }
    } catch (error: any) {
      connectionStatus = 'disconnected';
      errorMessage = error.message || 'Impossible de se connecter au serveur';
    }
  }

  async function saveConfig() {
    try {
      if (window.electronAPI) {
        const configSnapshot = $state.snapshot(serverConfig);
        await window.electronAPI.server.setConfig(configSnapshot);
      } else {
        localStorage.setItem('serverConfig', JSON.stringify(serverConfig));
      }
      await apiClient.init();
      if (profile.updated_at !== 0) {
        await saveProfile();
      }
      await testConnection();
    } catch (error: any) {
      errorMessage = error.message || 'Impossible de sauvegarder la configuration';
    }
  }

  // ─── Settings tab data loading ──────────────────────────
  async function loadTabData(tab: string) {
    if (tab === 'profile') {
      if (!profile.updated_at) await loadProfile();
      if (!systemInfo) await loadSystemInfo();
    }
    if (tab === 'connection') {
      if (!profile.updated_at) await loadProfile();
      await loadSystemInfo();
    }
    if (tab === 'models') {
      await loadModelsStatus();
    }
  }

  async function loadProfile() {
    profileLoading = true;
    try {
      profile = await apiClient.getProfile();
    } catch (err) {
      console.error('Failed to load profile:', err);
    } finally {
      profileLoading = false;
    }
  }

  async function saveProfile() {
    try {
      profile.updated_at = Math.floor(Date.now() / 1000);
      await apiClient.updateProfile(profile);
      profileSaved = true;
      setTimeout(() => { profileSaved = false; }, 2000);
    } catch (err: any) {
      errorMessage = err.message || 'Erreur lors de la sauvegarde du profil';
    }
  }

  async function loadSystemInfo() {
    try {
      systemInfo = await apiClient.getSystemInfo();
    } catch (err) {
      console.error('Failed to load system info:', err);
    }
  }

  async function loadModelsStatus(showLoading = true) {
    if (showLoading) modelsLoading = true;
    try {
      modelsStatus = await apiClient.getModelsStatus();
      if (isDownloading && modelsStatus.loaded_model?.name === downloadName) {
         isDownloading = false;
         if (downloadInterval) clearInterval(downloadInterval);
         downloadRepo = '';
         downloadFilename = '';
         downloadName = '';
      }
    } catch (err) {
      console.error('Failed to load models status:', err);
    } finally {
      if (showLoading) modelsLoading = false;
    }
  }

  async function handleReplaceModel() {
    if (!downloadRepo || !downloadFilename || !downloadName) return;
    try {
      isDownloading = true;
      await apiClient.replaceModel(downloadRepo, downloadFilename, downloadName);
      
      downloadInterval = setInterval(async () => {
        await loadModelsStatus(false);
      }, 5000);
    } catch (err: any) {
      errorMessage = err.message || 'Erreur lors du téléchargement du modèle';
      isDownloading = false;
    }
  }

  async function handleDeleteModel(modelId: string) {
    if (!confirm("Voulez-vous vraiment supprimer ce modèle du disque ?")) return;
    try {
      await apiClient.deleteModel(modelId);
      await loadModelsStatus();
    } catch (err: any) {
      errorMessage = err.message || 'Erreur lors de la suppression';
    }
  }

  async function handleLoadModel(modelId: string) {
    modelLoadingId = modelId;
    try {
      await apiClient.loadModel(modelId);
      await loadModelsStatus();
      await loadSystemInfo();
    } catch (err: any) {
      errorMessage = err.message || 'Erreur lors du chargement du modèle';
    } finally {
      modelLoadingId = null;
    }
  }

  // ─── Chat ──────────────────────────────────────────────

  async function sendMessage({ prompt, deepThink }: { prompt: string; deepThink: boolean }) {
    if (!prompt.trim() || generating) return;

    generating = true;

    const userMsg: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: prompt
    };
    msgs = [...msgs, userMsg];

    const newMsgId = crypto.randomUUID();
    const assistantMsg: ChatMessage = {
      id: newMsgId,
      role: 'assistant',
      content: '',
      streaming: true,
      thinking: deepThink
    };
    msgs = [...msgs, assistantMsg];

    let tokenBuffer = '';

    try {
      const newConvId = await apiClient.chatStream(
        conversationId,
        prompt,
        true,
        false,
        deepThink,
        activeAgent?.id ?? null,
        (token) => {
          const currentMsg = msgs[msgs.length - 1];
          if (tokenBuffer === '') {
            currentMsg.thinking = false;
            currentMsg.streaming = true;
          }
          tokenBuffer += token;
          currentMsg.content = tokenBuffer;
        },
        (metadata) => {
          const currentMsg = msgs[msgs.length - 1];
          if (metadata.assistant_message) {
            currentMsg.content = metadata.assistant_message;
            if (metadata.tokens_generated && metadata.generation_time_ms) {
              currentMsg.stats = {
                tokens_generated: metadata.tokens_generated,
                time_ms: metadata.generation_time_ms
              };
            }
          }
          if (metadata.score !== undefined) {
            (currentMsg as any).confidence = metadata.score;
          }
          // Recherche web : mettre à jour le label DeepThink
          if (metadata.status === 'started' || metadata.status === 'searching') {
            currentMsg.thinkingPhase = 'Searching...';
            currentMsg.webBadge = { text: 'Recherche web...', kind: 'searching' };
          } else if (metadata.status === 'done') {
            currentMsg.thinkingPhase = 'Analyzing...';
            if (currentMsg.webBadge?.kind === 'searching') {
              currentMsg.webBadge = { text: `${metadata.sources_count ?? ''} source(s) trouvée(s)`, kind: 'done' };
            }
          }
          if (metadata.message && !metadata.status) {
            currentMsg.contradictionWarning = metadata.message;
          }
        },
        (error) => {
          errorMessage = error;
          generating = false;
        },
        (step) => {
          const currentMsg = msgs[msgs.length - 1];
          if (!currentMsg.thinkingSteps) currentMsg.thinkingSteps = [];
          currentMsg.thinkingSteps = [...currentMsg.thinkingSteps, step];
          // Mapper la phase DeepThink sur le label affiché
          if (step.phase === 'decomposition') currentMsg.thinkingPhase = 'Analyzing...';
          else if (step.phase === 'thinking') currentMsg.thinkingPhase = 'Computing...';
          else if (step.phase === 'synthesis') currentMsg.thinkingPhase = 'Synthesizing...';
        }
      );

      conversationId = newConvId;
      msgs[msgs.length - 1].streaming = false;

      // Update conversations sidebar
      updateConversationsList(newConvId, prompt);
    } catch (error: any) {
      errorMessage = error.message || 'Erreur lors de la génération';
      msgs = msgs.filter(m => m.id !== newMsgId);
    } finally {
      generating = false;
    }
  }

  function updateConversationsList(convId: string, lastMessage: string) {
    const existing = conversations.find(c => c.id === convId);
    if (existing) {
      existing.timestamp = Date.now();
      existing.messageCount = msgs.length;
    } else {
      conversations = [{
        id: convId,
        preview: lastMessage,
        timestamp: Date.now(),
        messageCount: msgs.length
      }, ...conversations];
    }
  }

  function newConversation() {
    conversationId = null;
    msgs = [];
  }

  async function selectConversation(convId: string) {
    if (convId === conversationId) return;

    conversationId = convId;
    msgs = [];

    try {
      const history = await apiClient.getConversationHistory(convId);
      msgs = history.map((msg: any, i: number) => ({
        id: `${msg.timestamp || Date.now()}-${i}`,
        role: msg.role,
        content: msg.content
      }));
    } catch (err) {
      console.error('Failed to load history:', err);
      errorMessage = 'Impossible de charger l\'historique';
    }
  }

  async function handleWorkspaceDirChange(path: string | null) {
    try {
      await apiClient.setProjectDir(path);
      activeWorkspaceDir = path;
    } catch (err: any) {
      errorMessage = err.message || 'Impossible de définir le dossier de travail';
    }
  }

  function handleSuggestion(e: CustomEvent<string>) {
    sendMessage({ prompt: e.detail, deepThink: false });
  }

  function openSettings(tab: 'connection' | 'profile' | 'models' = 'profile') {
    settingsTab = tab;
    currentView = 'settings';
    if (connectionStatus === 'connected') {
      loadTabData(tab);
    }
  }

  function switchTab(tab: 'connection' | 'profile' | 'models') {
    settingsTab = tab;
    if (connectionStatus === 'connected') {
      loadTabData(tab);
    }
  }

  let serverUrl = $derived(`${serverConfig.protocol}://${serverConfig.host}:${serverConfig.port}`);
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
        {connectionStatus === 'connected' ? 'Connecté' : 'Déconnecté'}
      </div>
      <button class="icon-button" onclick={() => { newConversation(); currentView = 'chat'; }} title="Nouvelle conversation" aria-label="Nouvelle conversation">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 20h9"/>
          <path d="M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4L16.5 3.5z"/>
        </svg>
      </button>
    </div>
  </header>

  <div class="app-layout">
    <ConversationList
      {conversations}
      activeConversationId={conversationId}
      collapsed={sidebarCollapsed}
      currentView={currentView}
      on:select={(e) => selectConversation(e.detail)}
      on:new={() => { newConversation(); currentView = 'chat'; }}
      on:toggle={() => sidebarCollapsed = !sidebarCollapsed}
      on:navigate={(e) => {
        currentView = e.detail;
        agentSubroute = '';
        skillSubroute = '';
        if (currentView === 'settings') {
          openSettings(settingsTab);
        }
      }}
    />

    <main class="app-main">
      {#if currentView === 'chat'}
        <ChatMessages {msgs} on:suggest={handleSuggestion} />
        <InputArea
          on:send={(e) => sendMessage(e.detail)}
          disabled={generating || connectionStatus !== 'connected'}
          workspaceDir={activeWorkspaceDir}
          {activeAgent}
          on:workspaceDirChange={(e) => handleWorkspaceDirChange(e.detail)}
          on:agentChange={(e) => { activeAgent = e.detail; }}
        />
      {:else if currentView === 'agents'}
        <div class="main-page-container">
          <h2 style="margin-bottom: 1rem; font-family: var(--font-family);">Gestion des Projets / Agents{agentSubroute ? ` / ${agentSubroute}` : ''}</h2>
          <AgentsManager
            onsubroute={(label) => agentSubroute = label}
            on:select={(e) => {
            const selected = e.detail;
            activeAgent = activeAgent?.id === selected.id ? null : { id: selected.id, name: selected.name };
            currentView = 'chat';
          }} />
        </div>
      {:else if currentView === 'skills'}
        <div class="main-page-container">
          <h2 style="margin-bottom: 1rem; font-family: var(--font-family);">Gestion des Artéfacts / Skills{skillSubroute ? ` / ${skillSubroute}` : ''}</h2>
          <SkillsManager onsubroute={(label) => skillSubroute = label} />
        </div>
      {:else if currentView === 'settings'}
        <div class="main-page-container">
          <div class="settings-header" style="border: none; padding-bottom: 0;">
            <h2 style="margin-bottom: 1rem; font-family: var(--font-family);">Personnaliser</h2>
          </div>
          <div class="settings-tabs">
            <button
              class="settings-tab"
              class:active={settingsTab === 'profile'}
              onclick={() => switchTab('profile')}
            >Profil</button>
            <button
              class="settings-tab"
              class:active={settingsTab === 'connection'}
              onclick={() => switchTab('connection')}
            >Connexion</button>
            <button
              class="settings-tab"
              class:active={settingsTab === 'models'}
              onclick={() => switchTab('models')}
            >Modèles</button>
          </div>
          <div class="settings-body">


        {#if settingsTab === 'connection'}
          <!-- ── Connection Tab ──────────────────────────────── -->
          <div class="section-label">Configuration serveur</div>

          <div class="form-group">
            <label for="protocol">Protocole</label>
            <select id="protocol" bind:value={serverConfig.protocol}>
              <option value="http">HTTP</option>
              <option value="https">HTTPS</option>
            </select>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label for="host">Hôte</label>
              <input id="host" type="text" bind:value={serverConfig.host} placeholder="localhost" />
            </div>
            <div class="form-group">
              <label for="port">Port</label>
              <input id="port" type="number" bind:value={serverConfig.port} placeholder="3000" />
            </div>
          </div>

          <div class="server-url">
            {serverUrl}
          </div>

          <div class="button-group">
            <button onclick={testConnection} disabled={connectionStatus === 'testing'}>
              {connectionStatus === 'testing' ? 'Test...' : '🔌 Tester'}
            </button>
            <button onclick={saveConfig} class="primary">
              💾 Sauvegarder
            </button>
          </div>

          <div style="margin-top: var(--spacing-xl);"></div>
          <div class="section-label">Préférences matérielles</div>

          <div class="form-row">
            <div class="form-group">
              <label for="device-pref">Dispositif</label>
              <select id="device-pref" bind:value={profile.device_preference}>
                <option value="Gpu">GPU</option>
                <option value="Cpu">CPU</option>
              </select>
            </div>
            <div class="form-group">
              <label for="gpu-sel">Sélection GPU</label>
              <select id="gpu-sel" 
                value={typeof profile.gpu_selection === 'object' ? `Specific_${profile.gpu_selection.Specific}` : profile.gpu_selection} 
                onchange={(e) => {
                  const val = e.currentTarget.value;
                  if (val.startsWith('Specific_')) {
                    profile.gpu_selection = { Specific: parseInt(val.split('_')[1], 10) };
                  } else {
                    profile.gpu_selection = val;
                  }
                }}>
                <option value="Auto">Auto (GPU principal)</option>
                <option value="AllGpus">Tous les GPU (Multi-GPU)</option>
                {#if systemInfo && systemInfo.gpu_devices}
                  {#each systemInfo.gpu_devices as gpu (gpu.index)}
                    <option value={`Specific_${gpu.index}`}>GPU #{gpu.index} — {gpu.name} ({gpu.vram_free_mb} Mo VRAM)</option>
                  {/each}
                {/if}
              </select>
              {#if systemInfo && systemInfo.gpu_devices && systemInfo.gpu_devices.length > 0}
                <div style="font-size: 0.75rem; color: var(--text-tertiary); margin-top: 0.25rem;">
                  Seuls les GPU compatibles avec llama-cpp sont listés (GPU dédiés en priorité).
                </div>
              {/if}
            </div>
          </div>

          {#if systemInfo}
            <div style="margin-top: var(--spacing-xl);">
              <div class="section-label">Informations système</div>
              <div class="info-card-row">
                <div class="info-card">
                  <div class="info-card-label">Dispositif</div>
                  <div class="info-card-value">{systemInfo.device.label}</div>
                </div>
                <div class="info-card">
                  <div class="info-card-label">GPU disponible</div>
                  <div class="info-card-value">{systemInfo.device.gpu_available ? '✅ Oui' : '❌ Non'}</div>
                </div>
              </div>
              <div class="info-card-row">
                <div class="info-card">
                  <div class="info-card-label">Modèle actif</div>
                  <div class="info-card-value">{systemInfo.model.active ? systemInfo.model.name : 'Aucun'}</div>
                </div>
                <div class="info-card">
                  <div class="info-card-label">Préférence</div>
                  <div class="info-card-value">{systemInfo.preference.device}</div>
                </div>
              </div>
              {#if systemInfo.gpu_devices.length > 0}
                <div class="section-label" style="margin-top: var(--spacing-md);">GPU compatibles llama-cpp</div>
                {#each systemInfo.gpu_devices as gpu (gpu.index)}
                  <div class="info-card">
                    <div class="info-card-label">GPU #{gpu.index}</div>
                    <div class="info-card-value">{gpu.name}</div>
                    <div style="font-size: 0.75rem; color: var(--text-tertiary); margin-top: 0.25rem;">
                      {gpu.vram_free_mb} Mo VRAM libre
                    </div>
                  </div>
                {/each}
              {/if}
            </div>
          {/if}

        {:else if settingsTab === 'profile'}
          <!-- ── Profile Tab ─────────────────────────────────── -->
          {#if profileLoading}
            <div style="text-align: center; padding: var(--spacing-2xl); color: var(--text-tertiary);">
              Chargement du profil...
            </div>
          {:else}
            <div class="section-label">Informations personnelles</div>

            <div class="form-row">
              <div class="form-group">
                <label for="first-name">Prénom</label>
                <input id="first-name" type="text" bind:value={profile.first_name} placeholder="Votre prénom" />
              </div>
              <div class="form-group">
                <label for="age">Âge</label>
                <input id="age" type="number" bind:value={profile.age} placeholder="25" min="0" max="120" />
              </div>
            </div>

            <div class="form-group">
              <label for="pro-status">Statut professionnel</label>
              <select id="pro-status" bind:value={profile.professional_status}>
                <option value={null}>Non renseigné</option>
                <option value="Salarie">Salarié</option>
                <option value="ChomeurIndemise">Chômeur indemnisé</option>
                <option value="ChomeurNonIndemise">Chômeur non indemnisé</option>
                <option value="EtudiantApprentis">Étudiant / Apprenti</option>
                <option value="Retraite">Retraité</option>
                <option value="Independant">Indépendant</option>
                <option value="FonctionPublique">Fonction publique</option>
                <option value="Autre">Autre</option>
              </select>
            </div>

            <div class="form-row">
              <div class="form-group">
                <label for="department">Département</label>
                <input id="department" type="text" bind:value={profile.department} placeholder="75" maxlength="3" />
              </div>
              <div class="form-group">
                <label for="lang-level">Niveau de langue</label>
                <select id="lang-level" bind:value={profile.language_level}>
                  <option value="Simple">Simple</option>
                  <option value="Standard">Standard</option>
                  <option value="Technique">Technique</option>
                </select>
              </div>
            </div>



            <div class="button-group">
              <button onclick={saveProfile} class="primary">
                {profileSaved ? '✅ Sauvegardé' : '💾 Sauvegarder le profil'}
              </button>
            </div>
          {/if}

        {:else if settingsTab === 'models'}
          <!-- ── Models Tab ──────────────────────────────────── -->
          {#if modelsLoading}
            <div style="text-align: center; padding: var(--spacing-2xl); color: var(--text-tertiary);">
              Chargement des modèles...
            </div>
          {:else if modelsStatus}
            {#if modelsStatus.loaded_model}
              <div class="section-label">Modèle actif</div>
              <div class="model-card active">
                <div class="model-card-info">
                  <span class="model-card-name">{modelsStatus.loaded_model.name}</span>
                  <span class="model-card-meta">{modelsStatus.loaded_model.device_label}</span>
                </div>
                <span class="model-badge loaded">Chargé</span>
              </div>
            {/if}

            <div class="section-label" style="margin-top: var(--spacing-lg);">Modèles téléchargés</div>
            {#if modelsStatus.downloaded_models.length === 0}
              <div style="text-align: center; padding: var(--spacing-xl); color: var(--text-tertiary); font-size: 0.875rem;">
                Aucun modèle téléchargé
              </div>
            {:else}
              {#each modelsStatus.downloaded_models as model (model.id)}
                <div class="model-card" class:active={modelsStatus.loaded_model?.id === model.id}>
                  <div class="model-card-info">
                    <span class="model-card-name">{model.name}</span>
                    <span class="model-card-meta">{model.size_mb} Mo — {model.filename}</span>
                  </div>
                  {#if modelsStatus.loaded_model?.id === model.id}
                    <span class="model-badge loaded">Actif</span>
                  {:else}
                    <div style="display: flex; gap: 0.5rem; align-items: center;">
                      <button
                        class="primary"
                        style="padding: 0.25rem 0.75rem; font-size: 0.75rem;"
                        disabled={modelLoadingId !== null}
                        onclick={() => handleLoadModel(model.id)}
                      >
                        {modelLoadingId === model.id ? 'Chargement...' : 'Charger'}
                      </button>
                      <button
                        style="padding: 0.25rem 0.75rem; font-size: 0.75rem; background: var(--surface-3); border: 1px solid #ff4444; color: #ff4444; cursor: pointer; border-radius: 4px;"
                        onclick={() => handleDeleteModel(model.id)}
                      >
                        Supprimer
                      </button>
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}

            <div class="section-label" style="margin-top: var(--spacing-xl);">📥 Nouveau Modèle HuggingFace</div>
            <div style="background: var(--surface-2); padding: 1rem; border-radius: 8px; border: 1px solid var(--border-color);">
              <div class="form-group" style="margin-bottom: 0.8rem;">
                <label for="downloadRepo">Repo ID HuggingFace</label>
                <input id="downloadRepo" bind:value={downloadRepo} type="text" placeholder="ex: bartowski/Llama-3.2-1B-Instruct-GGUF" disabled={isDownloading} />
              </div>
              <div class="form-group" style="margin-bottom: 0.8rem;">
                <label for="downloadFilename">Nom exact du fichier</label>
                <input id="downloadFilename" bind:value={downloadFilename} type="text" placeholder="ex: Llama-3.2-1B-Instruct-Q4_K_M.gguf" disabled={isDownloading} />
              </div>
              <div class="form-group" style="margin-bottom: 1rem;">
                <label for="downloadName">Nom d'affichage</label>
                <input id="downloadName" bind:value={downloadName} type="text" placeholder="ex: Llama 3.2 1B (Rapide)" disabled={isDownloading} />
              </div>
              <button 
                class="primary" 
                style="width: 100%; padding: 0.6rem; text-align: center;" 
                onclick={handleReplaceModel} 
                disabled={isDownloading || !downloadRepo || !downloadFilename || !downloadName}
              >
                {isDownloading ? '⏳ Téléchargement en cours... (ne fermez pas)' : 'Télécharger et Remplacer l\'Actif'}
              </button>
            </div>

            <div class="button-group" style="margin-top: var(--spacing-xl);">
              <button onclick={loadModelsStatus}>
                🔄 Actualiser
              </button>
            </div>
          {:else}
            <div style="text-align: center; padding: var(--spacing-2xl); color: var(--text-tertiary); font-size: 0.875rem;">
              Connectez-vous au serveur pour voir les modèles.
            </div>
          {/if}
        {/if}
          </div>
        </div>
      {/if}
    </main>
  </div>
</div>

<style>
  .main-page-container {
    padding: var(--spacing-2xl);
    width: 100%;
    max-width: 900px;
    margin: 0 auto;
    overflow-y: auto;
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  /* Layout override to match new app structure */
  .app-layout {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .app-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  /* All other styles come from app.css global */
</style>
