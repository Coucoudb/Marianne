# marianne-web

> Frontend Svelte pour Marianne — consomme l'API HTTP/SSE de `marianne-server`.

## Setup

```bash
npm create svelte@latest .
npm install
npm run dev
```

## Architecture

- `src/lib/api.ts` — Client HTTP + SSE vers `marianne-server`
- `src/routes/` — Pages SvelteKit
- `src/lib/components/` — Composants UI

## Connexion au serveur

Par défaut le frontend pointe vers `http://localhost:3000`. Configurable via la variable d'environnement `PUBLIC_API_URL`.

```typescript
// src/lib/api.ts
const API_URL = import.meta.env.PUBLIC_API_URL ?? 'http://localhost:3000';

export function streamChat(message: string, conversationId?: string) {
  return new EventSource(
    `${API_URL}/api/v1/chat?message=${encodeURIComponent(message)}`
  );
}
```

> Le frontend n'est pas encore créé — lancez `npm create svelte@latest .` dans ce dossier pour démarrer.
