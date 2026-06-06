import { ipcMain } from 'electron';
import { spawn, ChildProcess } from 'child_process';

const terminals = new Map<string, ChildProcess>();

export function registerTerminalHandlers(): void {
  // Execute command
  ipcMain.handle('terminal:exec', async (_event, command: string, cwd?: string) => {
    return new Promise((resolve) => {
      const shell = process.platform === 'win32' ? 'powershell.exe' : '/bin/bash';
      const args = process.platform === 'win32' ? ['-Command', command] : ['-c', command];
      
      const proc = spawn(shell, args, {
        cwd: cwd || process.cwd(),
        shell: false
      });

      let stdout = '';
      let stderr = '';

      proc.stdout?.on('data', (data) => {
        stdout += data.toString();
      });

      proc.stderr?.on('data', (data) => {
        stderr += data.toString();
      });

      proc.on('close', (code) => {
        resolve({
          success: code === 0,
          exitCode: code,
          stdout,
          stderr
        });
      });

      proc.on('error', (error) => {
        resolve({
          success: false,
          error: error.message,
          stdout,
          stderr
        });
      });
    });
  });

  // Create persistent terminal session
  ipcMain.handle('terminal:create', async (_event, sessionId: string, cwd?: string) => {
    try {
      const shell = process.platform === 'win32' ? 'powershell.exe' : '/bin/bash';
      const proc = spawn(shell, [], {
        cwd: cwd || process.cwd(),
        shell: false
      });

      terminals.set(sessionId, proc);

      return { success: true, sessionId };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  });

  // Send input to terminal
  ipcMain.handle('terminal:input', async (_event, sessionId: string, input: string) => {
    const proc = terminals.get(sessionId);
    if (!proc) {
      return { success: false, error: 'Terminal session not found' };
    }

    try {
      proc.stdin?.write(input + '\n');
      return { success: true };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  });

  // Close terminal session
  ipcMain.handle('terminal:close', async (_event, sessionId: string) => {
    const proc = terminals.get(sessionId);
    if (!proc) {
      return { success: false, error: 'Terminal session not found' };
    }

    try {
      proc.kill();
      terminals.delete(sessionId);
      return { success: true };
    } catch (error) {
      return { success: false, error: (error as Error).message };
    }
  });
}
