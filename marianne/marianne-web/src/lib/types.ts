export type StatusType = 'loading' | 'ready' | 'error';

export interface WebBadge {
  text: string;
  kind: 'searching' | 'done' | 'empty' | 'offline';
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  /** Rendered markdown (assistant) or plain text (user) */
  content: string;
  /** Show "Marianne réfléchit..." spinner — no content yet */
  thinking?: boolean;
  /** Show "analyse le(s) document(s)..." spinner */
  analyzing?: boolean;
  /** Currently streaming tokens */
  streaming?: boolean;
  webBadge?: WebBadge;
  contradictionWarning?: string;
  sources?: string[];
  stats?: { time_ms: number; tokens_generated: number };
}

export interface DownloadProgress {
  percent: number;
  downloaded_mb: number;
  total_mb: number;
}
