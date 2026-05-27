/** Détecte si le code s'exécute dans un contexte Tauri (desktop). */
export const IS_TAURI: boolean = '__TAURI_INTERNALS__' in window;

const API_URL_STORAGE_KEY = 'marianne.web.api_url';

/** URL par défaut fournie au build. */
export const DEFAULT_API_URL: string =
  (import.meta.env.VITE_API_URL as string | undefined) ?? 'http://localhost:3000';

function normalizeApiUrl(url: string): string {
  return url.trim().replace(/\/+$/, '');
}

/** Lit l'URL API effective (localStorage > .env). */
export function getApiUrl(): string {
  try {
    const saved = localStorage.getItem(API_URL_STORAGE_KEY);
    if (saved && saved.trim().length > 0) {
      return normalizeApiUrl(saved);
    }
  } catch {
    // Ignore en environnement sans localStorage
  }
  return normalizeApiUrl(DEFAULT_API_URL);
}

/** Persiste l'URL API au runtime sans rebuild. */
export function setApiUrl(url: string): void {
  const normalized = normalizeApiUrl(url);
  localStorage.setItem(API_URL_STORAGE_KEY, normalized);
}

/** Supprime l'override runtime et revient à .env. */
export function resetApiUrl(): void {
  localStorage.removeItem(API_URL_STORAGE_KEY);
}

export function isValidHttpUrl(url: string): boolean {
  try {
    const parsed = new URL(url.trim());
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}
