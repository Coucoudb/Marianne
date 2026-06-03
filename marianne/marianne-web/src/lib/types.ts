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

// ─── API Types ──────────────────────────────────────────────────────────────

export interface ConversationTurn {
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
}

export type ProfessionalStatus = 'Salarie' | 'ChomeurIndemise' | 'ChomeurNonIndemise' | 
  'EtudiantApprentis' | 'Retraite' | 'Independant' | 'FonctionPublique' | 'Autre';

export type FamilyStatus = 'Celibataire' | 'EnCouple' | 
  { Parent: { children_count: number } } | 
  { ParentIsolé: { children_count: number } } |
  { CoupleAvecEnfants: { children_count: number } };

export type LanguageLevel = 'Simple' | 'Standard' | 'Technique';
export type DevicePreference = 'Gpu' | 'Cpu';
export type GpuSelection = 'Auto' | 'AllGpus' | { Specific: number };

export interface UserProfile {
  first_name: string;
  age: number | null;
  professional_status: ProfessionalStatus | null;
  family_status: FamilyStatus | null;
  department: string | null;
  topics_of_interest: string[];
  language_level: LanguageLevel;
  device_preference: DevicePreference;
  gpu_selection: GpuSelection;
  selected_model: string | null;
  updated_at: number;
}

export interface ExtractRequest {
  file_path: string;
  question?: string;
}

export interface ExtractedDocument {
  text: string;
  file_name: string;
  char_count: number;
  prompt: string;
}

export interface SystemInfo {
  device: {
    backend: string;
    label: string;
    gpu_available: boolean;
  };
  model: {
    name: string;
    active: boolean;
  };
  preference: {
    device: DevicePreference;
    gpu_selection: GpuSelection;
  };
  gpu_devices: GpuDevice[];
}

export interface GpuDevice {
  index: number;
  name: string;
  device_type: string;
  vram_free_mb: number;
}

export interface ModelInfo {
  id: string;
  name: string;
  repo_id: string;
  filename: string;
  size_mb: number;
}

export interface LoadedModelInfo {
  id: string;
  name: string;
  device: string;
  device_label: string;
}

export interface ModelsStatus {
  downloaded_models: ModelInfo[];
  loaded_model: LoadedModelInfo | null;
}

export interface DownloadModelRequest {
  repo_id: string;
  filename: string;
  name: string;
}

export interface LoadModelRequest {
  model_id: string;
}
