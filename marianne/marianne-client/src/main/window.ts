import { BrowserWindow, app } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import { registerIPCHandlers } from './ipc/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const isDev = process.env.NODE_ENV === 'development' || !app.isPackaged;

function createSplashWindow(): BrowserWindow {
  const splash = new BrowserWindow({
    width: 360,
    height: 400,
    frame: false,
    transparent: false,
    resizable: false,
    movable: true,
    center: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    backgroundColor: '#f8f6f2',
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false
    }
  });

  if (isDev) {
    splash.loadFile(path.join(__dirname, '../../src/renderer/splash.html'));
  } else {
    splash.loadFile(path.join(__dirname, '../renderer/src/renderer/splash.html'));
  }

  return splash;
}

export async function createWindow(): Promise<BrowserWindow> {
  // Show splash immediately
  const splash = createSplashWindow();

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
    backgroundColor: '#f8f6f2',
    show: false
  });

  // Register IPC handlers
  registerIPCHandlers(window);

  // Show main window and close splash when ready. MUST be registered before load.
  window.once('ready-to-show', () => {
    splash.destroy();
    window.show();
  });

  // Load the app
  if (isDev) {
    await window.loadURL('http://localhost:5173');
    window.webContents.openDevTools();
  } else {
    await window.loadFile(path.join(__dirname, '../renderer/index.html'));
  }

  // Safety: close splash if main window closes unexpectedly
  window.on('closed', () => {
    if (!splash.isDestroyed()) {
      splash.destroy();
    }
  });

  return window;
}
