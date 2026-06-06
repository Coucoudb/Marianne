import { BrowserWindow, app } from 'electron';
import path from 'path';
import { registerIPCHandlers } from './ipc';

const isDev = process.env.NODE_ENV === 'development' || !app.isPackaged;

export async function createWindow(): Promise<BrowserWindow> {
  const window = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 600,
    title: 'Marianne AI',
    webPreferences: {
      preload: path.join(__dirname, '../preload/index.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false
    },
    backgroundColor: '#1e1e1e',
    show: false
  });

  // Register IPC handlers
  registerIPCHandlers(window);

  // Load the app
  if (isDev) {
    await window.loadURL('http://localhost:5173');
    window.webContents.openDevTools();
  } else {
    await window.loadFile(path.join(__dirname, '../renderer/index.html'));
  }

  // Show window when ready
  window.once('ready-to-show', () => {
    window.show();
  });

  return window;
}
