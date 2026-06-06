/// <reference types="svelte" />
/// <reference types="vite/client" />

interface ElectronAPI {
  file: {
    openDialog: (options?: any) => Promise<string[]>;
    saveDialog: (options?: any) => Promise<string | undefined>;
    read: (filePath: string) => Promise<{ success: boolean; content?: string; error?: string }>;
    write: (filePath: string, content: string) => Promise<{ success: boolean; error?: string }>;
    listDir: (dirPath: string) => Promise<{ success: boolean; items?: any[]; error?: string }>;
    stat: (filePath: string) => Promise<{ success: boolean; stats?: any; error?: string }>;
  };
  terminal: {
    exec: (command: string, cwd?: string) => Promise<any>;
    create: (sessionId: string, cwd?: string) => Promise<any>;
    input: (sessionId: string, input: string) => Promise<any>;
    close: (sessionId: string) => Promise<any>;
  };
  server: {
    getConfig: () => Promise<{
      host: string;
      port: number;
      protocol: 'http' | 'https';
    }>;
    setConfig: (config: {
      host: string;
      port: number;
      protocol: 'http' | 'https';
    }) => Promise<any>;
    testConnection: (config?: any) => Promise<any>;
  };
  app: {
    getVersion: () => Promise<string>;
  };
}

declare global {
  interface Window {
    electronAPI: ElectronAPI;
  }
}

export {};
