<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as backend from './lib/backend';
  import { IS_TAURI } from './lib/api';

  import Header from './components/Header.svelte';
  import ChatMessages from './components/ChatMessages.svelte';
  import InputArea from './components/InputArea.svelte';
  import SetupModal from './components/SetupModal.svelte';
  import WebSettingsPage from './components/WebSettingsPage.svelte';

  import type { ChatMessage, StatusType, DownloadProgress } from './lib/types';

  // ─── App state ───────────────────────────────────────────────────────────
  let statusType: StatusType = 'loading';
  let statusText = 'Initialisation...';
  let modelLoaded = false;
  let generating = false;
  let conversationId: string | null = null;
  let msgs: ChatMessage[] = [];
  let showModal = false;
  let downloadPct: DownloadProgress | null = null;
  let corpusToastText: string | null = null;
  let tokenBuffer = '';
  let streamingId: string | null = null;
  let stagedFiles: { path: string; name: string }[] = [];
  let showWebSettings = false;
  let refreshTick = 0;

  // ─── Tauri event listeners ───────────────────────────────────────────────
  const unlisteners: Array<() => void> = [];

  onMount(async () => {
    const fns = await backend.setup(handleBackendEvent);
    unlisteners.push(...fns);
    await checkModelStatus();
  });

  onDestroy(() => {
    unlisteners.forEach(fn => fn());
  });

  // ─── Backend event dispatcher ─────────────────────────────────────────────
  function handleBackendEvent(event: string, payload: unknown) {
    type P = Record<string, unknown>;
    const p = payload as P;
    switch (event) {
      case 'stream-token': {
        if (!streamingId) return;
        if (tokenBuffer === '') {
          updateMsg(streamingId, { thinking: false, analyzing: false });
        }
        tokenBuffer += p.token as string;
        updateMsg(streamingId, { content: tokenBuffer });
        break;
      }
      case 'generation-done': {
        if (!streamingId) return;
        const id = streamingId;
        msgs = msgs.map(m =>
          m.id === id
            ? {
                ...m,
                content: (p.full_response as string) || tokenBuffer,
                streaming: false,
                thinking: false,
                analyzing: false,
                sources: p.sources as string[],
                stats: {
                  time_ms: p.time_ms as number,
                  tokens_generated: p.tokens_generated as number,
                },
              }
            : m
        );
        generating = false;
        streamingId = null;
        tokenBuffer = '';
        break;
      }
      case 'download-progress':
        downloadPct = payload as DownloadProgress;
        break;
      case 'model-ready':
        setStatus('ready', 'Marianne est prête');
        modelLoaded = true;
        showModal = false;
        refreshTick += 1;
        checkCorpusUpdate();
        break;
      case 'confidence-info': {
        if (!streamingId || !p.web_search_triggered) return;
        updateMsg(streamingId, {
          webBadge: {
            text: `🔍 Confiance ${Math.round((p.score as number) * 100)}% — recherche web en cours...`,
            kind: 'searching',
          },
        });
        break;
      }
      case 'web-search-status': {
        if (!streamingId || p.status !== 'done') return;
        updateMsg(streamingId, {
          webBadge:
            (p.sources_count as number) > 0
              ? {
                  text: `🌐 ${p.sources_count} source(s) web officielle(s) trouvée(s)`,
                  kind: 'done',
                }
              : {
                  text: '⚠️ Aucune source web trouvée — réponse basée sur le corpus local',
                  kind: 'empty',
                },
        });
        break;
      }
      case 'offline-mode':
        if (!streamingId) return;
        updateMsg(streamingId, {
          webBadge: { text: `📡 ${p.message as string}`, kind: 'offline' },
        });
        break;
      case 'contradiction-warning':
        if (!streamingId) return;
        updateMsg(streamingId, { contradictionWarning: p.message as string });
        break;
      case 'corpus-update-status':
        if (p.status === 'done' && (p.updated as number) > 0) {
          showCorpusToast(`📚 Corpus légal mis à jour — ${p.updated} fiche(s) actualisée(s)`);
        }
        break;
    }
  }

  // ─── Helpers ─────────────────────────────────────────────────────────────
  function setStatus(type: StatusType, text: string) {
    statusType = type;
    statusText = text;
  }

  function updateMsg(id: string, patch: Partial<ChatMessage>) {
    msgs = msgs.map(m => (m.id === id ? { ...m, ...patch } : m));
  }

  function addMsg(msg: ChatMessage) {
    msgs = [...msgs, msg];
  }

  function showCorpusToast(text: string) {
    corpusToastText = text;
    setTimeout(() => {
      corpusToastText = null;
    }, 5500);
  }

  // ─── Model management ─────────────────────────────────────────────────────
  async function checkModelStatus() {
    try {
      const s = await backend.checkStatus();
      if (!s.model_downloaded) {
        showModal = true;
        setStatus('loading', 'Modèle non installé');
      } else if (!s.model_loaded) {
        setStatus('loading', 'Chargement du modèle...');
        try {
          await backend.loadModel();
          setStatus('loading', 'Initialisation du RAG...');
          await backend.initRag().catch(e => console.warn('RAG init:', e));
          modelLoaded = true;
          setStatus('ready', 'Marianne est prête');
          refreshTick += 1;
          checkCorpusUpdate();
        } catch {
          setStatus('loading', 'Erreur GPU — tentative en mode CPU...');
          try {
            await backend.setDevicePreference('Cpu');
            await backend.loadModel();
            setStatus('loading', 'Initialisation du RAG...');
            await backend.initRag().catch(e => console.warn('RAG init:', e));
            modelLoaded = true;
            setStatus('ready', 'Marianne est prête (mode CPU)');
            refreshTick += 1;
            checkCorpusUpdate();
          } catch (cpuError) {
            setStatus('error', `Impossible de charger le modèle : ${cpuError}`);
          }
        }
      } else {
        modelLoaded = true;
        setStatus('ready', 'Marianne est prête');
        refreshTick += 1;
      }
    } catch (error) {
      showModal = IS_TAURI;
      setStatus('error', `${IS_TAURI ? 'Erreur : ' : 'Serveur inaccessible : '}${error}`);
    }
  }

  async function handleDownload() {
    downloadPct = { percent: 0, downloaded_mb: 0, total_mb: 0 };
    try {
      await backend.downloadModel();
      setStatus('loading', 'Chargement du modèle...');
      try {
        await backend.loadModel();
      } catch {
        setStatus('loading', 'Erreur GPU — tentative en mode CPU...');
        await backend.setDevicePreference('Cpu');
        await backend.loadModel();
      }
      setStatus('loading', 'Initialisation du RAG...');
      await backend.initRag().catch(e => console.warn('RAG init:', e));
      modelLoaded = true;
      setStatus('ready', 'Marianne est prête');
      showModal = false;
      downloadPct = null;
      refreshTick += 1;
      checkCorpusUpdate();
    } catch (error) {
      setStatus('error', `Erreur : ${error}`);
      downloadPct = null;
    }
  }

  async function checkCorpusUpdate() {
    try {
      const needs = await backend.checkCorpusUpdate();
      if (needs && modelLoaded) {
        backend.updateCorpus().catch(e => console.warn('Mise à jour corpus:', e));
      }
    } catch {
      // silencieux
    }
  }

  // ─── Chat ─────────────────────────────────────────────────────────────────
  async function sendMessage(message: string) {
    if (!message.trim() || generating || !modelLoaded) return;
    generating = true;

    addMsg({ id: crypto.randomUUID(), role: 'user', content: message });

    const assistantId = crypto.randomUUID();
    streamingId = assistantId;
    tokenBuffer = '';
    addMsg({
      id: assistantId,
      role: 'assistant',
      content: '',
      thinking: true,
      streaming: true,
    });

    try {
      const convId = await backend.sendChat(
        { message, conversation_id: conversationId, max_tokens: 1024 },
        handleBackendEvent
      );
      conversationId = convId;
    } catch (error) {
      updateMsg(assistantId, {
        content: `❌ Erreur : ${error}`,
        streaming: false,
        thinking: false,
      });
      generating = false;
      streamingId = null;
    }
  }

  function stopGeneration() {
    if (!generating) return;
    backend.stopGeneration();
  }

  // ─── Documents ────────────────────────────────────────────────────────────
  function stageFile(path: string) {
    if (stagedFiles.some(f => f.path === path)) return;
    const name = path.split(/[\\/]/).pop() || 'document';
    stagedFiles = [...stagedFiles, { path, name }];
  }

  function removeStagedFile(path: string) {
    stagedFiles = stagedFiles.filter(f => f.path !== path);
  }

  async function openFilePicker() {
    if (generating || !modelLoaded) return;
    if (!IS_TAURI) {
      addMsg({
        id: crypto.randomUUID(),
        role: 'assistant',
        content: "⚠️ L'analyse de documents n'est pas disponible en mode client web. Utilisez l'application desktop Marianne.",
      });
      return;
    }
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        filters: [{ name: 'Documents', extensions: ['pdf', 'txt', 'md'] }],
        multiple: true,
      });
      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected as string];
        for (const p of paths) stageFile(p);
      }
    } catch (e) {
      console.error('Erreur sélection fichier:', e);
    }
  }

  function handleDrop(e: CustomEvent<FileList>) {
    for (const file of e.detail) {
      const ext = file.name.split('.').pop()?.toLowerCase();
      if (['pdf', 'txt', 'md'].includes(ext ?? '')) {
        const path = (file as any).path;
        if (path) stageFile(path);
      } else {
        addMsg({
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `⚠️ Fichier « ${file.name} » ignoré — format non supporté. Utilisez PDF, TXT ou MD.`,
        });
      }
    }
  }

  async function sendWithDocuments(message: string) {
    const files = [...stagedFiles];
    stagedFiles = [];
    generating = true;

    const fileLabels = files.map(f => `📄 ${f.name}`).join(', ');
    const displayMessage = message ? `${fileLabels}\n\n${message}` : fileLabels;
    addMsg({ id: crypto.randomUUID(), role: 'user', content: displayMessage });

    const assistantId = crypto.randomUUID();
    streamingId = assistantId;
    tokenBuffer = '';
    addMsg({
      id: assistantId,
      role: 'assistant',
      content: '',
      analyzing: true,
      streaming: true,
    });

    try {
      const extractions: { file_name: string; text: string }[] = [];
      for (const file of files) {
        const result = await backend.extractDocument({ file_path: file.path, question: null });
        extractions.push(result);
      }

      let prompt: string;
      if (extractions.length === 1) {
        const doc = extractions[0];
        const q = message || 'Explique ce document en langage clair et dis-moi ce que je dois faire.';
        prompt = `Voici un document administratif français (${doc.file_name}) :\n\n---\n${doc.text}\n---\n\nQuestion : ${q}`;
      } else {
        const docsText = extractions
          .map((doc, i) => `── Document ${i + 1} : ${doc.file_name} ──\n${doc.text}`)
          .join('\n\n');
        const q = message || 'Explique ces documents en langage clair et dis-moi ce que je dois faire.';
        prompt = `Voici ${extractions.length} documents administratifs français :\n\n${docsText}\n\n---\n\nQuestion : ${q}`;
      }

      const convId = await backend.sendChat(
        { message: prompt, conversation_id: conversationId, max_tokens: 1024 },
        handleBackendEvent
      );
      conversationId = convId;
    } catch (error) {
      updateMsg(assistantId, {
        content: `❌ ${error}`,
        streaming: false,
        analyzing: false,
      });
      generating = false;
      streamingId = null;
    }
  }

  // ─── Event handlers from child components ─────────────────────────────────
  function handleSend(e: CustomEvent<{ message: string; hasFiles: boolean }>) {
    if (e.detail.hasFiles) {
      sendWithDocuments(e.detail.message);
    } else {
      sendMessage(e.detail.message);
    }
  }

  function openWebSettings() {
    if (IS_TAURI) return;
    showWebSettings = true;
  }

  function closeWebSettings() {
    showWebSettings = false;
  }
</script>

<div id="app">
  <Header
    {statusType}
    {statusText}
    {refreshTick}
    {downloadPct}
    on:openWebSettings={openWebSettings}
  />

  <main class="chat-container">
    {#if showWebSettings}
      <WebSettingsPage on:close={closeWebSettings} />
    {:else}
      <ChatMessages {msgs} on:drop={handleDrop} />

      <InputArea
        {generating}
        {modelLoaded}
        {stagedFiles}
        on:send={handleSend}
        on:upload={openFilePicker}
        on:removeFile={e => removeStagedFile(e.detail)}
        on:stop={stopGeneration}
      />
    {/if}
  </main>
</div>

{#if showModal}
  <SetupModal {downloadPct} on:download={() => handleDownload()} />
{/if}

{#if corpusToastText}
  <div class="corpus-toast">{corpusToastText}</div>
{/if}
