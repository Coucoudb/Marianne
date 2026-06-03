<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Router from 'svelte-spa-router';
  import { routes } from './routes';
  import * as backend from './lib/backend';
  import { IS_TAURI } from './lib/api';

  import Header from './components/Header.svelte';
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
            checkCorpusUpdate();
          } catch (cpuError) {
            setStatus('error', `Impossible de charger le modèle : ${cpuError}`);
          }
        }
      } else {
        modelLoaded = true;
        setStatus('ready', 'Marianne est prête');
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

  // Props to pass to routed components
  const routeProps = {
    generating,
    conversationId,
    msgs,
    stagedFiles,
    modelLoaded,
    downloadPct,
    tokenBuffer,
    streamingId,
  };
</script>

<div id="app">
  <Header
    {statusType}
    {statusText}
    {downloadPct}
  />

  <main class="app-content">
    <Router {routes} />
  </main>
</div>

{#if showModal}
  <SetupModal {downloadPct} on:download={() => handleDownload()} />
{/if}

{#if corpusToastText}
  <div class="corpus-toast">{corpusToastText}</div>
{/if}

<style>
  .app-content {
    flex: 1;
    overflow: hidden;
  }
</style>
