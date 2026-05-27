/**
 * backend.ts — Couche d'abstraction transport
 *
 * Mode Tauri (desktop) : IPC natif via invoke() / listen()
 * Mode HTTP  (web)     : fetch() + Server-Sent Events vers marianne-server
 */

import { IS_TAURI, getApiUrl } from './api';

// ─── Types ──────────────────────────────────────────────────────────────────

/** Fonction de dispatch — même interface que les listeners Tauri côté App. */
export type Dispatcher = (event: string, payload: unknown) => void;

export interface ChatRequest {
  message: string;
  conversation_id: string | null;
  max_tokens: number;
}

// ─── État interne SSE ────────────────────────────────────────────────────────

let _abortController: AbortController | null = null;

// ─── Événements à relayer ────────────────────────────────────────────────────

const BACKEND_EVENTS = [
  'stream-token',
  'generation-done',
  'download-progress',
  'model-ready',
  'confidence-info',
  'web-search-status',
  'offline-mode',
  'contradiction-warning',
  'corpus-update-status',
] as const;

/**
 * Enregistre les listeners d'événements.
 * - Tauri : utilise listen() → retourne les fonctions de nettoyage.
 * - HTTP  : rien à enregistrer (les événements arrivent via SSE dans sendChat).
 */
export async function setup(dispatch: Dispatcher): Promise<Array<() => void>> {
  if (!IS_TAURI) return [];
  const { listen } = await import('@tauri-apps/api/event');
  return Promise.all(
    BACKEND_EVENTS.map(event =>
      listen(event, ({ payload }) => dispatch(event, payload))
    )
  );
}

// ─── Status ──────────────────────────────────────────────────────────────────

/**
 * Vérifie si le backend est prêt.
 * - Tauri : interroge check_model_status
 * - HTTP  : GET /health → retourne model_downloaded/model_loaded = true si ok
 */
export async function checkStatus(): Promise<{
  model_downloaded: boolean;
  model_loaded: boolean;
}> {
  if (IS_TAURI) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('check_model_status');
  }
  const res = await fetch(`${getApiUrl()}/health`);
  if (!res.ok) throw new Error('Serveur inaccessible');
  return { model_downloaded: true, model_loaded: true };
}

// ─── Gestion modèle (Tauri uniquement — silencieux en HTTP) ─────────────────

export async function loadModel(): Promise<void> {
  if (!IS_TAURI) return;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('load_model');
}

export async function initRag(): Promise<void> {
  if (!IS_TAURI) return;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('initialize_rag');
}

export async function downloadModel(): Promise<void> {
  if (!IS_TAURI) return;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('download_model');
}

export async function checkCorpusUpdate(): Promise<boolean> {
  if (!IS_TAURI) return false;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('check_corpus_update');
}

export async function updateCorpus(): Promise<void> {
  if (!IS_TAURI) return;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('update_corpus');
}

export async function setDevicePreference(preference: 'Gpu' | 'Cpu'): Promise<void> {
  if (!IS_TAURI) return;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('set_device_preference', { preference });
}

// ─── Chat ────────────────────────────────────────────────────────────────────

/**
 * Lance une génération de réponse.
 *
 * - Tauri : invoke('send_message') — les événements arrivent via Tauri event bus
 *           (setup() a déjà enregistré les listen() qui appellent dispatch).
 * - HTTP  : POST /api/v1/chat + lecture du flux SSE — les événements sont émis
 *           directement via dispatch() pour alimenter les mêmes handlers.
 *
 * @returns conversation_id
 */
export async function sendChat(
  request: ChatRequest,
  dispatch: Dispatcher
): Promise<string> {
  if (IS_TAURI) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('send_message', { request });
  }
  return _sendChatHttp(request, dispatch);
}

async function _sendChatHttp(
  request: ChatRequest,
  dispatch: Dispatcher
): Promise<string> {
  _abortController = new AbortController();

  const res = await fetch(`${getApiUrl()}/api/v1/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
    signal: _abortController.signal,
  });

  if (!res.ok || !res.body) {
    throw new Error(`Erreur serveur ${res.status}`);
  }

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let conversationId = request.conversation_id ?? crypto.randomUUID();
  let rawBuffer = '';
  let currentEvent = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      rawBuffer += decoder.decode(value, { stream: true });
      const lines = rawBuffer.split('\n');
      rawBuffer = lines.pop() ?? '';

      for (const line of lines) {
        if (line.startsWith('event: ')) {
          currentEvent = line.slice(7).trim();
        } else if (line.startsWith('data: ') && currentEvent) {
          try {
            const payload = JSON.parse(line.slice(6));
            // Récupère le conversation_id dès le premier stream-token
            if (
              currentEvent === 'stream-token' &&
              typeof payload.conversation_id === 'string'
            ) {
              conversationId = payload.conversation_id;
            }
            dispatch(currentEvent, payload);
          } catch {
            // JSON malformé — ignorer
          }
          currentEvent = '';
        } else if (line === '') {
          currentEvent = '';
        }
      }
    }
  } catch (err: unknown) {
    if ((err as Error).name !== 'AbortError') throw err;
  }

  return conversationId;
}

export function stopGeneration(): void {
  if (IS_TAURI) {
    import('@tauri-apps/api/core').then(({ invoke }) =>
      invoke('stop_generation').catch(() => {})
    );
  } else {
    _abortController?.abort();
    _abortController = null;
  }
}

// ─── Documents ───────────────────────────────────────────────────────────────

/**
 * Extrait le texte d'un document.
 * - Tauri  : invoke('extract_document') — le fichier est local
 * - HTTP   : non supporté (le serveur ne peut pas accéder aux fichiers locaux
 *            du navigateur sans upload multipart — non implémenté)
 */
export async function extractDocument(request: {
  file_path: string;
  question: string | null;
}): Promise<{ file_name: string; text: string }> {
  if (IS_TAURI) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('extract_document', { request });
  }
  throw new Error(
    "L'analyse de documents n'est pas disponible en mode client web. Utilisez l'application desktop Marianne."
  );
}
