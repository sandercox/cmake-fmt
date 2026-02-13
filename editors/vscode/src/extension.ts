import * as vscode from 'vscode';
import { CMakeFormattingProvider, CMakeRangeFormattingProvider, initFormatter } from './formatter';
import { createDiagnosticsProvider } from './diagnostics';

export function activate(context: vscode.ExtensionContext) {
  console.log('cmake-fmt extension activated');

  // Resolve cmake-fmt binary (bundled or user-configured)
  initFormatter(context.extensionPath);

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

  // Register diagnostics for cmake-fmt directives in CMake files
  createDiagnosticsProvider(context);

  context.subscriptions.push(fullDocDisposable, rangeDisposable);
}

function configureTaploSchema(context: vscode.ExtensionContext) {
  const schemaUri = vscode.Uri.joinPath(context.extensionUri, 'schemas', 'cmake-fmt.schema.json');
  // Taplo needs file:// URI, not vscode-file:// URI
  const schemaPath = schemaUri.scheme === 'file' ? schemaUri.toString() : schemaUri.fsPath;
  const config = vscode.workspace.getConfiguration('evenBetterToml');
  const associations: Record<string, string> = config.get('schema.associations') ?? {};

  // File patterns for cmake-fmt TOML config files (regex matched against document URI)
  // Extensionless .cmake-fmt is YAML, so not included here
  const patterns = [
    '\\.cmake-fmt\\.toml$',
    '\\.cmake-fmt\\.tml$',
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
