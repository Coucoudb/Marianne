import { ipcMain } from 'electron';
import { app } from 'electron';
import Store from 'electron-store';

interface ServerConfig {
  host: string;
  port: number;
  protocol: 'http' | 'https';
}

const store = new Store<{ serverConfig: ServerConfig }>({
  defaults: {
    serverConfig: {
      host: 'localhost',
      port: 3000,
      protocol: 'http'
    }
  }
});

export function registerServerHandlers(): void {
  // Get server configuration
  ipcMain.handle('server:getConfig', async () => {
    return store.get('serverConfig');
  });

  // Set server configuration
  ipcMain.handle('server:setConfig', async (_event, config: ServerConfig) => {
    store.set('serverConfig', config);
    return { success: true };
  });

  // Test server connection
  ipcMain.handle('server:testConnection', async (_event, config?: ServerConfig) => {
    const serverConfig = config || store.get('serverConfig');
    const url = `${serverConfig.protocol}://${serverConfig.host}:${serverConfig.port}/health`;

    try {
      const response = await fetch(url, {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' }
      });

      return {
        success: response.ok,
        status: response.status,
        message: response.ok ? 'Connexion réussie' : 'Échec de connexion'
      };
    } catch (error) {
      return {
        success: false,
        error: (error as Error).message,
        message: 'Impossible de se connecter au serveur'
      };
    }
  });

  // Get app version
  ipcMain.handle('app:getVersion', async () => {
    return app.getVersion();
  });
}
