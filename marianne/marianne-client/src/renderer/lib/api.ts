import type { ServerConfig } from './types';

/** Get server URL from electron-store via IPC */
export async function getServerUrl(): Promise<string> {
  const config: ServerConfig = await window.electronAPI.server.getConfig();
  return `${config.protocol}://${config.host}:${config.port}`;
}

/** API client for marianne-server HTTP endpoints */
export class ApiClient {
  private baseUrl: string = '';

  async init() {
    this.baseUrl = await getServerUrl();
  }

  private async fetch(endpoint: string, options: RequestInit = {}): Promise<Response> {
    if (!this.baseUrl) {
      await this.init();
    }

    const url = `${this.baseUrl}${endpoint}`;
    const response = await fetch(url, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers
      }
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return response;
  }

  // ─── Health ────────────────────────────────────────────────────────────
  async health(): Promise<{ status: string }> {
    const res = await this.fetch('/health');
    return res.json();
  }

  // ─── Chat ──────────────────────────────────────────────────────────────
  async chatStream(
    conversationId: string | null,
    prompt: string,
    documents: any[] = [],
    onToken: (token: string) => void,
    onMetadata: (data: any) => void,
    onError: (error: string) => void,
    signal?: AbortSignal
  ): Promise<string> {
    if (!this.baseUrl) {
      await this.init();
    }

    const url = `${this.baseUrl}/chat`;
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        conversation_id: conversationId,
        prompt,
        documents
      }),
      signal
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const reader = response.body?.getReader();
    if (!reader) {
      throw new Error('No response body');
    }

    const decoder = new TextDecoder();
    let buffer = '';
    let newConvId = conversationId;

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (!line.trim() || !line.startsWith('data: ')) continue;

          const data = line.slice(6);
          if (data === '[DONE]') continue;

          try {
            const parsed = JSON.parse(data);

            if (parsed.token) {
              onToken(parsed.token);
            }

            if (parsed.conversation_id && !newConvId) {
              newConvId = parsed.conversation_id;
            }

            if (parsed.metadata) {
              onMetadata(parsed.metadata);
            }

            if (parsed.error) {
              onError(parsed.error);
            }
          } catch (e) {
            console.warn('Failed to parse SSE data:', data, e);
          }
        }
      }
    } finally {
      reader.releaseLock();
    }

    return newConvId || '';
  }

  // ─── History ───────────────────────────────────────────────────────────
  async getConversationsList(): Promise<any[]> {
    const res = await this.fetch('/history/conversations');
    return res.json();
  }

  async getConversation(id: string): Promise<any> {
    const res = await this.fetch(`/history/conversations/${id}`);
    return res.json();
  }

  async deleteConversation(id: string): Promise<void> {
    await this.fetch(`/history/conversations/${id}`, { method: 'DELETE' });
  }

  async deleteAllConversations(): Promise<void> {
    await this.fetch('/history/conversations', { method: 'DELETE' });
  }

  // ─── Profile ───────────────────────────────────────────────────────────
  async getProfile(): Promise<any> {
    const res = await this.fetch('/profile');
    return res.json();
  }

  async saveProfile(profile: any): Promise<void> {
    await this.fetch('/profile', {
      method: 'POST',
      body: JSON.stringify(profile)
    });
  }

  // ─── Models ────────────────────────────────────────────────────────────
  async getSystemInfo(): Promise<any> {
    const res = await this.fetch('/models/system-info');
    return res.json();
  }

  async listModels(): Promise<any[]> {
    const res = await this.fetch('/models');
    return res.json();
  }

  async selectModel(modelId: string): Promise<void> {
    await this.fetch('/models/select', {
      method: 'POST',
      body: JSON.stringify({ model_id: modelId })
    });
  }

  async downloadModel(modelId: string): Promise<void> {
    await this.fetch('/models/download', {
      method: 'POST',
      body: JSON.stringify({ model_id: modelId })
    });
  }

  // ─── Documents ─────────────────────────────────────────────────────────
  async extractDocument(filePath: string, question?: string): Promise<any> {
    const res = await this.fetch('/documents/extract', {
      method: 'POST',
      body: JSON.stringify({ file_path: filePath, question })
    });
    return res.json();
  }
}

export const apiClient = new ApiClient();
