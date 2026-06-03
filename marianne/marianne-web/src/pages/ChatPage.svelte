<script lang="ts">
  import { onMount } from 'svelte';
  import * as backend from '../lib/backend';
  import { IS_TAURI } from '../lib/api';

  import ChatMessages from '../components/ChatMessages.svelte';
  import InputArea from '../components/InputArea.svelte';

  import type { ChatMessage } from '../lib/types';

  // Chat state
  let generating = false;
  let conversationId: string | null = null;
  let msgs: ChatMessage[] = [];
  let stagedFiles: { path: string; name: string }[] = [];
  let tokenBuffer = '';
  let streamingId: string | null = null;
  let modelLoaded = true; // Assume loaded if we're on this page

  function addMsg(msg: ChatMessage) {
    msgs = [...msgs, msg];
  }

  function updateMsg(id: string, patch: Partial<ChatMessage>) {
    msgs = msgs.map(m => (m.id === id ? { ...m, ...patch } : m));
  }

  // Backend event handler for streaming
  function handleBackendEvent(event: string, payload: unknown) {
    type P = Record<string, unknown>;
    const p = payload as P;
    switch (event) {
      case 'stream-token': {
        if (streamingId) {
          if (tokenBuffer === '') {
            updateMsg(streamingId, { thinking: false, analyzing: false });
          }
          tokenBuffer += p.token as string;
          updateMsg(streamingId, { content: tokenBuffer });
        }
        break;
      }
      case 'generation-done': {
        if (streamingId) {
          updateMsg(streamingId, {
            content: (p.full_response as string) || tokenBuffer,
            streaming: false,
            thinking: false,
            analyzing: false,
            sources: p.sources as string[],
            stats: {
              time_ms: p.time_ms as number,
              tokens_generated: p.tokens_generated as number,
            },
          });
          generating = false;
          streamingId = null;
          tokenBuffer = '';
        }
        break;
      }
      case 'confidence-info': {
        if (streamingId && p.web_search_triggered) {
          updateMsg(streamingId, {
            webBadge: {
              text: `🔍 Confiance ${Math.round((p.score as number) * 100)}% — recherche web en cours...`,
              kind: 'searching',
            },
          });
        }
        break;
      }
      case 'web-search-status': {
        if (streamingId && p.status === 'done') {
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
        }
        break;
      }
      case 'offline-mode': {
        if (streamingId) {
          updateMsg(streamingId, {
            webBadge: { text: `📡 ${p.message as string}`, kind: 'offline' },
          });
        }
        break;
      }
      case 'contradiction-warning': {
        if (streamingId) {
          updateMsg(streamingId, { contradictionWarning: p.message as string });
        }
        break;
      }
    }
  }

  async function handleSend(e: CustomEvent<{ message: string; hasFiles: boolean }>) {
    const { message, hasFiles } = e.detail;
    if (!message.trim() || generating) return;

    if (hasFiles) {
      await sendWithDocuments(message);
    } else {
      await sendMessage(message);
    }
  }

  async function sendMessage(message: string) {
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
      conversationId = await backend.sendChat(
        { message, conversation_id: conversationId, max_tokens: 1024 },
        handleBackendEvent
      );
    } catch (error) {
      updateMsg(assistantId, {
        content: `❌ Erreur : ${error}`,
        streaming: false,
        thinking: false,
      });
      generating = false;
      streamingId = null;
      tokenBuffer = '';
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

      conversationId = await backend.sendChat(
        { message: prompt, conversation_id: conversationId, max_tokens: 1024 },
        handleBackendEvent
      );
    } catch (error) {
      updateMsg(assistantId, {
        content: `❌ ${error}`,
        streaming: false,
        analyzing: false,
      });
      generating = false;
      streamingId = null;
      tokenBuffer = '';
    }
  }

  function handleStop() {
    backend.stopGeneration();
    if (streamingId) {
      updateMsg(streamingId, { streaming: false, thinking: false, analyzing: false });
      streamingId = null;
    }
    generating = false;
    tokenBuffer = '';
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
        for (const path of paths) {
          const name = path.split(/[\\/]/).pop() || 'document';
          if (!stagedFiles.some(f => f.path === path)) {
            stagedFiles = [...stagedFiles, { path, name }];
          }
        }
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
        if (path && !stagedFiles.some(f => f.path === path)) {
          stagedFiles = [...stagedFiles, { path, name: file.name }];
        }
      } else {
        addMsg({
          id: crypto.randomUUID(),
          role: 'assistant',
          content: `⚠️ Fichier « ${file.name} » ignoré — format non supporté. Utilisez PDF, TXT ou MD.`,
        });
      }
    }
  }

  function handleRemoveFile(e: CustomEvent<string>) {
    stagedFiles = stagedFiles.filter(f => f.path !== e.detail);
  }

  onMount(() => {
    // Initial setup if needed
  });
</script>

<main class="chat-page">
  <ChatMessages {msgs} on:drop={handleDrop} />
  <InputArea
    {generating}
    {stagedFiles}
    {modelLoaded}
    on:send={handleSend}
    on:stop={handleStop}
    on:upload={openFilePicker}
    on:removeFile={handleRemoveFile}
  />
</main>

<style>
  .chat-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }
</style>

