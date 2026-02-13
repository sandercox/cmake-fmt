import * as vscode from 'vscode';
import { spawn } from 'child_process';
import * as path from 'path';

/**
 * Shared CLI formatting function used by both full-document and range providers.
 * Spawns cmake-fmt with specified arguments and returns formatted text.
 */
async function formatWithCli(
  documentText: string,
  args: string[],
  token: vscode.CancellationToken
): Promise<string> {
  return new Promise((resolve, reject) => {
    const config = vscode.workspace.getConfiguration('cmakeFmt');
    const binaryPath = path.normalize(config.get<string>('binaryPath', 'cmake-fmt'));

    // Spawn cmake-fmt with provided arguments
    const childProcess = spawn(binaryPath, args);

    let stdout = '';
    let stderr = '';
    let killed = false;

    // Set timeout (30 seconds)
    const timeout = setTimeout(() => {
      killed = true;
      childProcess.kill();
      reject(new Error('timeout'));
    }, 30000);

    // Handle cancellation
    const cancellationListener = token.onCancellationRequested(() => {
      killed = true;
      childProcess.kill();
      reject(new Error('cancelled'));
    });

    // Attach data listeners BEFORE writing to stdin (prevents buffer deadlock)
    childProcess.stdout.on('data', (data: Buffer) => {
      stdout += data.toString();
    });

    childProcess.stderr.on('data', (data: Buffer) => {
      stderr += data.toString();
    });

    childProcess.on('error', (err: NodeJS.ErrnoException) => {
      clearTimeout(timeout);
      cancellationListener.dispose();
      reject(err);
    });

    childProcess.on('exit', (code) => {
      clearTimeout(timeout);
      cancellationListener.dispose();

      if (killed) {
        return; // Already rejected with timeout or cancelled
      }

      if (code === 0) {
        resolve(stdout);
      } else {
        reject({
          code,
          stderr,
          stdout,
        });
      }
    });

    // Write document text to stdin and close
    childProcess.stdin.write(documentText);
    childProcess.stdin.end();
  });
}

/**
 * Shared error handler for both full-document and range providers.
 */
function handleError(error: any, fileName: string): void {
  const basename = path.basename(fileName);

  // Handle binary not found (ENOENT)
  if (error.code === 'ENOENT' || (error.message && error.message.includes('ENOENT'))) {
    vscode.window
      .showErrorMessage(
        'cmake-fmt binary not found. Please configure the path in settings.',
        'Open Settings'
      )
      .then((selection) => {
        if (selection === 'Open Settings') {
          vscode.commands.executeCommand('workbench.action.openSettings', 'cmakeFmt.binaryPath');
        }
      });
    return;
  }

  // Handle timeout
  if (error.message && (error.message.includes('timeout') || error.message.includes('ETIMEDOUT'))) {
    vscode.window.showErrorMessage(
      `cmake-fmt timed out formatting ${basename}. The file may be too large or contain syntax errors.`
    );
    return;
  }

  // Handle cancellation
  if (error.message && error.message.includes('cancelled')) {
    // Silent - user cancelled the operation
    return;
  }

  // General error with stderr context
  let errorMessage = `cmake-fmt failed: ${error.message || 'Unknown error'}`;

  if (error.stderr) {
    const stderrPreview = error.stderr.substring(0, 200);
    errorMessage = `cmake-fmt failed: ${stderrPreview}`;
  }

  vscode.window.showErrorMessage(errorMessage);
}

/**
 * Full-document formatting provider for CMake files.
 * Formats entire document using cmake-fmt CLI.
 */
export class CMakeFormattingProvider implements vscode.DocumentFormattingEditProvider {
  public async provideDocumentFormattingEdits(
    document: vscode.TextDocument,
    options: vscode.FormattingOptions,
    token: vscode.CancellationToken
  ): Promise<vscode.TextEdit[]> {
    // Check if formatting is enabled
    const config = vscode.workspace.getConfiguration('cmakeFmt');
    const enabled = config.get<boolean>('enable', true);

    if (!enabled) {
      return [];
    }

    try {
      const documentText = document.getText();
      const documentPath = document.fileName;
      const args = ['-', '--assume-filename', documentPath];

      const formattedText = await formatWithCli(documentText, args, token);

      // If output is identical to input, return empty array (no-op)
      if (formattedText === documentText) {
        return [];
      }

      // Construct a single TextEdit replacing the full document
      const fullRange = new vscode.Range(
        document.lineAt(0).range.start,
        document.lineAt(document.lineCount - 1).range.end
      );

      return [vscode.TextEdit.replace(fullRange, formattedText)];
    } catch (error) {
      handleError(error, document.fileName);
      return [];
    }
  }
}

/**
 * Range formatting provider for CMake files.
 * Formats only selected lines using cmake-fmt --line-ranges flag.
 */
export class CMakeRangeFormattingProvider implements vscode.DocumentRangeFormattingEditProvider {
  public async provideDocumentRangeFormattingEdits(
    document: vscode.TextDocument,
    range: vscode.Range,
    options: vscode.FormattingOptions,
    token: vscode.CancellationToken
  ): Promise<vscode.TextEdit[]> {
    // Check if formatting is enabled
    const config = vscode.workspace.getConfiguration('cmakeFmt');
    const enabled = config.get<boolean>('enable', true);

    if (!enabled) {
      return [];
    }

    try {
      const documentText = document.getText();
      const documentPath = document.fileName;

      // Convert VS Code 0-based line indices to 1-based for CLI
      const startLine = range.start.line + 1;
      const endLine = range.end.line + 1;
      const lineRanges = `${startLine}:${endLine}`;

      const args = ['-', '--assume-filename', documentPath, '--line-ranges', lineRanges];

      const formattedText = await formatWithCli(documentText, args, token);

      // If output is identical to input, return empty array (no-op)
      if (formattedText === documentText) {
        return [];
      }

      // Replace full document (CLI returns full spliced document)
      const fullRange = new vscode.Range(
        document.lineAt(0).range.start,
        document.lineAt(document.lineCount - 1).range.end
      );

      return [vscode.TextEdit.replace(fullRange, formattedText)];
    } catch (error) {
      handleError(error, document.fileName);
      return [];
    }
  }
}
