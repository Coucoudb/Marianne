import { ipcMain, dialog } from 'electron';
import fs from 'fs/promises';
import path from 'path';

export function registerFileHandlers(): void {
  // Open file dialog
  ipcMain.handle('file:openDialog', async (_event, options) => {
    const properties = options?.properties || ['openFile', 'multiSelections'];
    const isDirectory = properties.includes('openDirectory');

    const result = await dialog.showOpenDialog({
      properties,
      filters: isDirectory ? undefined : (options?.filters || [
        { name: 'Documents', extensions: ['pdf', 'txt', 'md', 'doc', 'docx'] },
        { name: 'Tous les fichiers', extensions: ['*'] }
      ])
    });
    return result.filePaths;
  });

  // Save file dialog
  ipcMain.handle('file:saveDialog', async (_event, options) => {
    const result = await dialog.showSaveDialog({
      filters: options?.filters || [
        { name: 'Texte', extensions: ['txt'] },
        { name: 'Markdown', extensions: ['md'] },
        { name: 'Tous les fichiers', extensions: ['*'] }
      ]
    });
    return result.filePath;
  });

  // Read file
  ipcMain.handle('file:read', async (_event, filePath: string) => {
    try {
      const content = await fs.readFile(filePath, 'utf-8');
      return { success: true, content };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  });

  // Write file
  ipcMain.handle('file:write', async (_event, filePath: string, content: string) => {
    try {
      await fs.writeFile(filePath, content, 'utf-8');
      return { success: true };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  });

  // List directory
  ipcMain.handle('file:listDir', async (_event, dirPath: string) => {
    try {
      const entries = await fs.readdir(dirPath, { withFileTypes: true });
      const items = entries.map(entry => ({
        name: entry.name,
        path: path.join(dirPath, entry.name),
        isDirectory: entry.isDirectory()
      }));
      return { success: true, items };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  });

  // Get file stats
  ipcMain.handle('file:stat', async (_event, filePath: string) => {
    try {
      const stats = await fs.stat(filePath);
      return {
        success: true,
        stats: {
          size: stats.size,
          isFile: stats.isFile(),
          isDirectory: stats.isDirectory(),
          created: stats.birthtime,
          modified: stats.mtime
        }
      };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  });
}
