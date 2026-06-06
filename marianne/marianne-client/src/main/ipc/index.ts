import { BrowserWindow } from 'electron';
import { registerFileHandlers } from './files.js';
import { registerTerminalHandlers } from './terminal.js';
import { registerServerHandlers } from './server.js';

export function registerIPCHandlers(_window: BrowserWindow): void {
  registerFileHandlers();
  registerTerminalHandlers();
  registerServerHandlers();
}
