import * as vscode from 'vscode';
import { CMakeFormattingProvider } from './formatter';

export function activate(context: vscode.ExtensionContext) {
  console.log('cmake-fmt extension activated');

  const cmakeFormattingProvider = new CMakeFormattingProvider();

  const disposable = vscode.languages.registerDocumentFormattingEditProvider(
    { language: 'cmake', scheme: 'file' },
    cmakeFormattingProvider
  );

  context.subscriptions.push(disposable);
}

export function deactivate() {
  // Cleanup handled by disposables
}
