import type { 
  ServerConfig, 
  ChatRequest,
  ConversationTurn,
  UserProfile,
  ExtractRequest,
  ExtractedDocument,
  SystemInfo,
  ModelsStatus,
  DownloadRequest,
  LoadRequest
} from './types';

export interface Agent {
  id: string;
  name: string;
  description: string;
  system_prompt: string;
  skills: string[];
  tools: string[];
  working_directory?: string;
}

export interface Skill {
  id: string;
  name: string;
  description: string;
  content: string;
  scope?: string;
}

export type SaveLevel = 'global' | 'server' | 'project';

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

  // ═══════════════════════════════════════════════════════════════════════
  // Health Check
  // ═══════════════════════════════════════════════════════════════════════

  /** GET /health - Check server health */
  async health(): Promise<string> {
    const res = await this.fetch('/health');
    return res.text();
  }

  // ═══════════════════════════════════════════════════════════════════════
  // Chat
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * POST /api/v1/chat - Stream chat with SSE
   * 
   * @param conversationId - Existing conversation ID or null for new
   * @param userMessage - User's message
   * @param useRag - Enable RAG search (default: true)
   * @param useWebSearch - Enable web search (default: false)
   * @param onToken - Callback for each token streamed
   * @param onMetadata - Callback for metadata (generation done, confidence, etc.)
   * @param onError - Callback for errors
   * @param signal - AbortSignal for cancellation
   * @returns New or existing conversation ID
   */
  async chatStream(
    conversationId: string | null,
    userMessage: string,
    useRag: boolean = true,
    useWebSearch: boolean = false,
    onToken: (token: string) => void,
    onMetadata: (data: any) => void,
    onError: (error: string) => void,
    signal?: AbortSignal
  ): Promise<string> {
    if (!this.baseUrl) {
      await this.init();
    }

    const url = `${this.baseUrl}/api/v1/chat`;
    const body: ChatRequest = {
      message: userMessage,
      conversation_id: conversationId
    };

    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
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

            // stream-token event
            if (parsed.token) {
              onToken(parsed.token);
            }

            // Track conversation_id
            if (parsed.conversation_id && !newConvId) {
              newConvId = parsed.conversation_id;
            }

            // Pass all metadata events to callback
            if (parsed.assistant_message || parsed.score || parsed.status || parsed.message) {
              onMetadata(parsed);
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

  // ═══════════════════════════════════════════════════════════════════════
  // History
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * GET /api/v1/history/:conversation_id - Get conversation history
   * 
   * @param conversationId - Conversation ID
   * @returns Array of conversation turns (user/assistant messages)
   */
  async getConversationHistory(conversationId: string): Promise<ConversationTurn[]> {
    const res = await this.fetch(`/api/v1/history/${conversationId}`);
    return res.json();
  }

  // ═══════════════════════════════════════════════════════════════════════
  // Profile
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * GET /api/v1/profile - Get user profile
   * 
   * @returns User profile with preferences and personal info
   */
  async getProfile(): Promise<UserProfile> {
    const res = await this.fetch('/api/v1/profile');
    return res.json();
  }

  /**
   * PUT /api/v1/profile - Update user profile
   * 
   * @param profile - Updated user profile
   */
  async updateProfile(profile: UserProfile): Promise<void> {
    await this.fetch('/api/v1/profile', {
      method: 'PUT',
      body: JSON.stringify(profile)
    });
  }

  // ═══════════════════════════════════════════════════════════════════════
  // Documents
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * POST /api/v1/documents/extract - Extract text from document (PDF, TXT)
   * 
   * @param filePath - Absolute path to the document
   * @param question - Optional question about the document
   * @returns Extracted document with text and metadata
   */
  async extractDocument(filePath: string, question?: string): Promise<ExtractedDocument> {
    const body: ExtractRequest = { file_path: filePath };
    if (question) {
      body.question = question;
    }

    const res = await this.fetch('/api/v1/documents/extract', {
      method: 'POST',
      body: JSON.stringify(body)
    });
    return res.json();
  }

  // ═══════════════════════════════════════════════════════════════════════
  // System
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * GET /api/v1/system/info - Get system and model information
   * 
   * @returns System info (device, model, GPU devices)
   */
  async getSystemInfo(): Promise<SystemInfo> {
    const res = await this.fetch('/api/v1/system/info');
    return res.json();
  }

  // ═══════════════════════════════════════════════════════════════════════
  // Models Management
  // ═══════════════════════════════════════════════════════════════════════

  /**
   * GET /api/v1/models/status - Get models status
   * 
   * @returns Downloaded models and currently loaded model
   */
  async getModelsStatus(): Promise<ModelsStatus> {
    const res = await this.fetch('/api/v1/models/status');
    return res.json();
  }

  /**
   * POST /api/v1/models/download - Download a model from HuggingFace
   * 
   * @param repoId - HuggingFace repository ID (e.g., "microsoft/Phi-3-mini-4k-instruct-gguf")
   * @param filename - GGUF filename to download
   * @param name - Display name for the model
   * @returns Download status and model ID
   */
  async downloadModel(repoId: string, filename: string, name: string): Promise<{ status: string; model_id: string }> {
    const body: DownloadRequest = {
      repo_id: repoId,
      filename,
      name
    };

    const res = await this.fetch('/api/v1/models/download', {
      method: 'POST',
      body: JSON.stringify(body)
    });
    return res.json();
  }

  /**
   * POST /api/v1/models/replace - Download a model and replace the current one
   */
  async replaceModel(repoId: string, filename: string, name: string): Promise<{ status: string; model_id: string }> {
    const body: DownloadRequest = {
      repo_id: repoId,
      filename,
      name
    };

    const res = await this.fetch('/api/v1/models/replace', {
      method: 'POST',
      body: JSON.stringify(body)
    });
    return res.json();
  }

  /**
   * DELETE /api/v1/models/:id - Delete a downloaded model
   */
  async deleteModel(modelId: string): Promise<void> {
    await this.fetch(`/api/v1/models/${modelId}`, { method: 'DELETE' });
  }

  /**
   * POST /api/v1/models/load - Load a downloaded model into memory
   * 
   * @param modelId - Model ID to load
   * @returns Load status with model name and device
   */
  async loadModel(modelId: string): Promise<{ status: string; model_name: string; device: string }> {
    const body: LoadRequest = { model_id: modelId };

    const res = await this.fetch('/api/v1/models/load', {
      method: 'POST',
      body: JSON.stringify(body)
    });
    return res.json();
  }

  /**
   * POST /api/v1/models/setup - Re-run full initialization sequence
   * 
   * @returns Setup status with model name and RAG chunks
   */
  async setupModels(): Promise<{ status: string; model: string; rag_chunks: number }> {
    const res = await this.fetch('/api/v1/models/setup', {
      method: 'POST',
      body: JSON.stringify({})
    });
    return res.json();
  }

  // --- Workspace (Agents & Skills) ---

  async listAgents(): Promise<Agent[]> {
    const res = await this.fetch('/api/v1/workspace/agents');
    const json = await res.json();
    return json.data || [];
  }

  async saveAgent(agent: Agent, level: SaveLevel = 'server'): Promise<Agent> {
    const endpoint = agent.id ? `/api/v1/workspace/agents/${agent.id}?level=${level}` : `/api/v1/workspace/agents?level=${level}`;
    const res = await this.fetch(endpoint, {
      method: agent.id ? 'PUT' : 'POST',
      body: JSON.stringify(agent)
    });
    const json = await res.json();
    return json.data;
  }

  async deleteAgent(id: string): Promise<void> {
    await this.fetch(`/api/v1/workspace/agents/${id}`, { method: 'DELETE' });
  }

  async listSkills(): Promise<Skill[]> {
    const res = await this.fetch('/api/v1/workspace/skills');
    const json = await res.json();
    return json.data || [];
  }

  async saveSkill(skill: Skill, level: SaveLevel = 'server'): Promise<Skill> {
    const endpoint = skill.id ? `/api/v1/workspace/skills/${skill.id}?level=${level}` : `/api/v1/workspace/skills?level=${level}`;
    const res = await this.fetch(endpoint, {
      method: skill.id ? 'PUT' : 'POST',
      body: JSON.stringify(skill)
    });
    const json = await res.json();
    return json.data;
  }

  async deleteSkill(id: string): Promise<void> {
    await this.fetch(`/api/v1/workspace/skills/${id}`, { method: 'DELETE' });
  }
}

export const apiClient = new ApiClient();
