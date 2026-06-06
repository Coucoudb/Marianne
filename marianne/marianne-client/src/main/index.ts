import { app, BrowserWindow } from 'electron';
import { createWindow } from './window';

let mainWindow: BrowserWindow | null = null;

app.on('ready', async () => {
  mainWindow = await createWindow();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', async () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    mainWindow = await createWindow();
  }
});

// Clean up reference when quitting
app.on('before-quit', () => {
  if (mainWindow) {
    mainWindow = null;
  }
});
