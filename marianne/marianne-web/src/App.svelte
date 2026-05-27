<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  import Header from './components/Header.svelte';
  import ChatMessages from './components/ChatMessages.svelte';
  import InputArea from './components/InputArea.svelte';
  import SetupModal from './components/SetupModal.svelte';

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
  let refreshTick = 0;

  // ─── Tauri event listeners ───────────────────────────────────────────────
  const unlisteners: UnlistenFn[] = [];

  onMount(async () => {
    const fns = await Promise.all([
      listen<{ token: string; conversation_id: string }>('stream-token', ({ payload }) => {
        if (!streamingId) return;
        if (tokenBuffer === '') {
          updateMsg(streamingId, { thinking: false, analyzing: false });
        }
        tokenBuffer += payload.token;
        updateMsg(streamingId, { content: tokenBuffer });
      }),

      listen<{
        full_response: string;
        sources: string[];
        tokens_generated: number;
        time_ms: number;
        conversation_id: string;
      }>('generation-done', ({ payload }) => {
        if (!streamingId) return;
        const id = streamingId;
        msgs = msgs.map(m =>
          m.id === id
            ? {
                ...m,
                content: payload.full_response || tokenBuffer,
                streaming: false,
                thinking: false,
                analyzing: false,
                sources: payload.sources,
                stats: { time_ms: payload.time_ms, tokens_generated: payload.tokens_generated },
              }
            : m
        );
        generating = false;
        streamingId = null;
        tokenBuffer = '';
      }),

      listen<DownloadProgress>('download-progress', ({ payload }) => {
        downloadPct = payload;
      }),

      listen('model-ready', async () => {
        setStatus('ready', 'Marianne est prête');
        modelLoaded = true;
        showModal = false;
        refreshTick += 1;
        checkCorpusUpdate();
      }),

      listen<{ score: number; web_search_triggered: boolean; conversation_id: string }>(
        'confidence-info',
        ({ payload }) => {
          if (!streamingId || !payload.web_search_triggered) return;
          updateMsg(streamingId, {
            webBadge: {
              text: `🔍 Confiance ${Math.round(payload.score * 100)}% — recherche web en cours...`,
              kind: 'searching',
            },
          });
        }
      ),

      listen<{ status: string; sources_count: number }>('web-search-status', ({ payload }) => {
        if (!streamingId || payload.status !== 'done') return;
        updateMsg(streamingId, {
          webBadge:
            payload.sources_count > 0
              ? {
                  text: `🌐 ${payload.sources_count} source(s) web officielle(s) trouvée(s)`,
                  kind: 'done',
                }
              : {
                  text: '⚠️ Aucune source web trouvée — réponse basée sur le corpus local',
                  kind: 'empty',
                },
        });
      }),

      listen<{ message: string; confidence: number }>('offline-mode', ({ payload }) => {
        if (!streamingId) return;
        updateMsg(streamingId, {
          webBadge: { text: `📡 ${payload.message}`, kind: 'offline' },
        });
      }),

      listen<{ message: string; conversation_id: string }>('contradiction-warning', ({ payload }) => {
        if (!streamingId) return;
        updateMsg(streamingId, { contradictionWarning: payload.message });
      }),

      listen<{ status: string; updated: number }>('corpus-update-status', ({ payload }) => {
        if (payload.status === 'done' && payload.updated > 0) {
          showCorpusToast(`📚 Corpus légal mis à jour — ${payload.updated} fiche(s) actualisée(s)`);
        }
      }),
    ]);

    unlisteners.push(...fns);
    await checkModelStatus();
  });

  onDestroy(() => {
    unlisteners.forEach(fn => fn());
  });

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
      const s = await invoke<{ model_downloaded: boolean; model_loaded: boolean }>(
        'check_model_status'
      );
      if (!s.model_downloaded) {
        showModal = true;
        setStatus('loading', 'Modèle non installé');
      } else if (!s.model_loaded) {
        setStatus('loading', 'Chargement du modèle...');
        try {
          await invoke('load_model');
          setStatus('loading', 'Initialisation du RAG...');
          await invoke('initialize_rag').catch(e => console.warn('RAG init:', e));
          modelLoaded = true;
          setStatus('ready', 'Marianne est prête');
          refreshTick += 1;
          checkCorpusUpdate();
        } catch {
          setStatus('loading', 'Erreur GPU — tentative en mode CPU...');
          try {
            await invoke('set_device_preference', { preference: 'Cpu' });
            await invoke('load_model');
            setStatus('loading', 'Initialisation du RAG...');
            await invoke('initialize_rag').catch(e => console.warn('RAG init:', e));
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
      showModal = true;
      setStatus('error', `Erreur : ${error}`);
    }
  }

  async function handleDownload() {
    downloadPct = { percent: 0, downloaded_mb: 0, total_mb: 0 };
    try {
      await invoke('download_model');
      setStatus('loading', 'Chargement du modèle...');
      try {
        await invoke('load_model');
      } catch {
        setStatus('loading', 'Erreur GPU — tentative en mode CPU...');
        await invoke('set_device_preference', { preference: 'Cpu' });
        await invoke('load_model');
      }
      setStatus('loading', 'Initialisation du RAG...');
      await invoke('initialize_rag').catch(e => console.warn('RAG init:', e));
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
      const needs = await invoke<boolean>('check_corpus_update');
      if (needs && modelLoaded) {
        invoke('update_corpus').catch(e => console.warn('Mise à jour corpus:', e));
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
      const convId = await invoke<string>('send_message', {
        request: { message, conversation_id: conversationId, max_tokens: 1024 },
      });
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

  async function stopGeneration() {
    if (!generating) return;
    try {
      await invoke('stop_generation');
    } catch (e) {
      console.warn('stop_generation:', e);
    }
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
        const result = await invoke<{ file_name: string; text: string }>('extract_document', {
          request: { file_path: file.path, question: null },
        });
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

      const convId = await invoke<string>('send_message', {
        request: { message: prompt, conversation_id: conversationId, max_tokens: 1024 },
      });
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
</script>

<div id="app">
  <Header {statusType} {statusText} {refreshTick} {downloadPct} />

  <main class="chat-container">
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
  </main>
</div>

{#if showModal}
  <SetupModal {downloadPct} on:download={() => handleDownload()} />
{/if}

{#if corpusToastText}
  <div class="corpus-toast">{corpusToastText}</div>
{/if}
