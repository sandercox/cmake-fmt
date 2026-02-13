import * as vscode from 'vscode';
import { CMakeFormattingProvider, CMakeRangeFormattingProvider } from './formatter';

export function activate(context: vscode.ExtensionContext) {
  console.log('cmake-fmt extension activated');

  // Configure TOML schema association for Even Better TOML extension (taplo)
  configureTaploSchema(context);

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

function configureTaploSchema(context: vscode.ExtensionContext) {
  const schemaPath = vscode.Uri.joinPath(context.extensionUri, 'schemas', 'cmake-fmt.schema.json').toString();
  const config = vscode.workspace.getConfiguration('evenBetterToml');
  const associations: Record<string, string> = config.get('schema.associations') ?? {};

  // File patterns for cmake-fmt config files (regex matched against file path)
  const patterns = [
    '\\.cmake-fmt\\.toml$',
    '\\.cmake-fmt\\.tml$',
    '(^|/)\\.cmake-fmt$',
  ];

  let needsUpdate = false;
  for (const pattern of patterns) {
    if (associations[pattern] !== schemaPath) {
      associations[pattern] = schemaPath;
      needsUpdate = true;
    }
  }

  if (needsUpdate) {
    config.update('schema.associations', associations, vscode.ConfigurationTarget.Global);
  }
}

export function deactivate() {
  // Cleanup handled by disposables
}
