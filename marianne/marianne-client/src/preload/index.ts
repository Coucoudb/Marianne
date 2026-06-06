import { contextBridge, ipcRenderer } from 'electron';

// Expose protected methods that allow the renderer process to use
// the ipcRenderer without exposing the entire object
contextBridge.exposeInMainWorld('electronAPI', {
  // File operations
  file: {
    openDialog: (options?: any) => ipcRenderer.invoke('file:openDialog', options),
    saveDialog: (options?: any) => ipcRenderer.invoke('file:saveDialog', options),
    read: (filePath: string) => ipcRenderer.invoke('file:read', filePath),
    write: (filePath: string, content: string) => ipcRenderer.invoke('file:write', filePath, content),
    listDir: (dirPath: string) => ipcRenderer.invoke('file:listDir', dirPath),
    stat: (filePath: string) => ipcRenderer.invoke('file:stat', filePath)
  },

  // Terminal operations
  terminal: {
    exec: (command: string, cwd?: string) => ipcRenderer.invoke('terminal:exec', command, cwd),
    create: (sessionId: string, cwd?: string) => ipcRenderer.invoke('terminal:create', sessionId, cwd),
    input: (sessionId: string, input: string) => ipcRenderer.invoke('terminal:input', sessionId, input),
    close: (sessionId: string) => ipcRenderer.invoke('terminal:close', sessionId)
  },

  // Server operations
  server: {
    getConfig: () => ipcRenderer.invoke('server:getConfig'),
    setConfig: (config: any) => ipcRenderer.invoke('server:setConfig', config),
    testConnection: (config?: any) => ipcRenderer.invoke('server:testConnection', config)
  },

  // App operations
  app: {
    getVersion: () => ipcRenderer.invoke('app:getVersion')
  }
});

// TypeScript declarations
export interface ElectronAPI {
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
    getConfig: () => Promise<any>;
    setConfig: (config: any) => Promise<any>;
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
