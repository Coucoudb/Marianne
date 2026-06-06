import { BrowserWindow } from 'electron';
import { registerFileHandlers } from './files';
import { registerTerminalHandlers } from './terminal';
import { registerServerHandlers } from './server';

export function registerIPCHandlers(_window: BrowserWindow): void {
  registerFileHandlers();
  registerTerminalHandlers();
  registerServerHandlers();
}
