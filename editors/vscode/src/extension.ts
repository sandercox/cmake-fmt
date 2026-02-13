import * as vscode from 'vscode';
import { CMakeFormattingProvider, CMakeRangeFormattingProvider } from './formatter';

export function activate(context: vscode.ExtensionContext) {
  console.log('cmake-fmt extension activated');

  // Register full-document formatting provider
  const cmakeFormattingProvider = new CMakeFormattingProvider();
  const fullDocDisposable = vscode.languages.registerDocumentFormattingEditProvider(
    { language: 'cmake', scheme: 'file' },
    cmakeFormattingProvider
  );

  // Register range formatting provider (Format Selection)
  const cmakeRangeFormattingProvider = new CMakeRangeFormattingProvider();
  const rangeDisposable = vscode.languages.registerDocumentRangeFormattingEditProvider(
    { language: 'cmake', scheme: 'file' },
    cmakeRangeFormattingProvider
  );

  context.subscriptions.push(fullDocDisposable, rangeDisposable);
}

export function deactivate() {
  // Cleanup handled by disposables
}
