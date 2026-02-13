import * as vscode from 'vscode';

/** Valid cmake-fmt directive keywords */
const DIRECTIVE_KEYWORDS = ['off', 'on', 'skip'];

/** Valid style override keys with their allowed values */
const STYLE_KEYS: Record<string, { type: 'integer' | 'boolean' | 'enum'; values?: string[] }> = {
  indent_width: { type: 'integer' },
  max_line_length: { type: 'integer' },
  use_tabs: { type: 'boolean' },
  command_case: { type: 'enum', values: ['lowercase', 'uppercase', 'leave'] },
  user_command_case: { type: 'enum', values: ['lowercase', 'uppercase', 'leave', 'infer'] },
  max_blank_lines: { type: 'integer' },
  line_ending: { type: 'enum', values: ['auto', 'lf', 'crlf'] },
  closing_style: { type: 'enum', values: ['leave', 'remove', 'force'] },
  source_grouping: { type: 'enum', values: ['none', 'headers_first', 'sources_first'] },
  force_break_keywords: { type: 'boolean' },
};

/**
 * Parse a cmake-fmt directive from a comment line.
 * Returns null if the line is not a cmake-fmt directive.
 */
function parseDirectiveLine(lineText: string): {
  prefix: 'cmake-fmt:';
  afterPrefix: string;
  afterPrefixStart: number;
} | null {
  // Match: optional whitespace, #, optional whitespace, cmake-fmt, optional space, :, rest
  const match = lineText.match(/^(\s*#\s*cmake-fmt\s*:\s*)(.*)/);
  if (!match) {
    return null;
  }
  return {
    prefix: 'cmake-fmt:',
    afterPrefix: match[2].trim(),
    afterPrefixStart: match[1].length,
  };
}

/**
 * Validate a cmake-fmt directive value and return diagnostics.
 */
function validateDirective(
  afterPrefix: string,
  line: number,
  afterPrefixStart: number,
  lineText: string,
): vscode.Diagnostic[] {
  const diagnostics: vscode.Diagnostic[] = [];

  // Empty directive
  if (!afterPrefix) {
    const range = new vscode.Range(line, 0, line, lineText.length);
    diagnostics.push(new vscode.Diagnostic(
      range,
      `Empty cmake-fmt directive. Expected: off, on, skip, or key=value (e.g., indent_width=4)`,
      vscode.DiagnosticSeverity.Warning,
    ));
    return diagnostics;
  }

  // Check for known keywords
  if (DIRECTIVE_KEYWORDS.includes(afterPrefix)) {
    return diagnostics; // Valid
  }

  // Check for style override (key=value)
  const eqPos = afterPrefix.indexOf('=');
  if (eqPos !== -1) {
    const key = afterPrefix.substring(0, eqPos).trim();
    const value = afterPrefix.substring(eqPos + 1).trim();

    // Find positions in original line for precise ranges
    const afterPrefixInLine = lineText.indexOf(afterPrefix, afterPrefixStart);
    const keyStart = afterPrefixInLine >= 0 ? afterPrefixInLine : afterPrefixStart;

    // Validate key
    if (!STYLE_KEYS[key]) {
      const keyRange = new vscode.Range(line, keyStart, line, keyStart + key.length);
      const validKeys = Object.keys(STYLE_KEYS).join(', ');
      diagnostics.push(new vscode.Diagnostic(
        keyRange,
        `Unknown config key '${key}'. Valid keys: ${validKeys}`,
        vscode.DiagnosticSeverity.Error,
      ));
      return diagnostics;
    }

    // Validate value
    if (!value) {
      const eqInLine = lineText.indexOf('=', keyStart);
      const valRange = new vscode.Range(line, eqInLine + 1, line, lineText.length);
      diagnostics.push(new vscode.Diagnostic(
        valRange,
        `Missing value for '${key}'`,
        vscode.DiagnosticSeverity.Error,
      ));
      return diagnostics;
    }

    const spec = STYLE_KEYS[key];
    const valueStartInLine = lineText.indexOf(value, lineText.indexOf('=', keyStart) + 1);
    const valStart = valueStartInLine >= 0 ? valueStartInLine : keyStart + key.length + 1;
    const valRange = new vscode.Range(line, valStart, line, valStart + value.length);

    if (spec.type === 'integer') {
      if (!/^\d+$/.test(value)) {
        diagnostics.push(new vscode.Diagnostic(
          valRange,
          `Invalid value for '${key}': expected a non-negative integer, got '${value}'`,
          vscode.DiagnosticSeverity.Error,
        ));
      }
    } else if (spec.type === 'boolean') {
      if (value !== 'true' && value !== 'false') {
        diagnostics.push(new vscode.Diagnostic(
          valRange,
          `Invalid value for '${key}': expected true or false, got '${value}'`,
          vscode.DiagnosticSeverity.Error,
        ));
      }
    } else if (spec.type === 'enum' && spec.values) {
      if (!spec.values.includes(value)) {
        diagnostics.push(new vscode.Diagnostic(
          valRange,
          `Invalid value for '${key}': expected ${spec.values.join(', ')}; got '${value}'`,
          vscode.DiagnosticSeverity.Error,
        ));
      }
    }

    return diagnostics;
  }

  // Not a keyword and not key=value — unrecognized directive
  const afterPrefixInLine = lineText.indexOf(afterPrefix, afterPrefixStart);
  const start = afterPrefixInLine >= 0 ? afterPrefixInLine : afterPrefixStart;
  const range = new vscode.Range(line, start, line, start + afterPrefix.length);
  diagnostics.push(new vscode.Diagnostic(
    range,
    `Unknown cmake-fmt directive '${afterPrefix}'. Expected: off, on, skip, or key=value`,
    vscode.DiagnosticSeverity.Error,
  ));

  return diagnostics;
}

/**
 * Analyze cmake-fmt directives in a document and return diagnostics.
 */
export function analyzeCmakeFmtDirectives(document: vscode.TextDocument): vscode.Diagnostic[] {
  const diagnostics: vscode.Diagnostic[] = [];

  // Track suppression state for off/on pairing
  let suppressionActive = false;
  let suppressionStartLine = -1;

  for (let i = 0; i < document.lineCount; i++) {
    const lineText = document.lineAt(i).text;
    const parsed = parseDirectiveLine(lineText);
    if (!parsed) {
      continue;
    }

    // Validate the directive itself
    const lineDiags = validateDirective(parsed.afterPrefix, i, parsed.afterPrefixStart, lineText);
    diagnostics.push(...lineDiags);

    // Track suppression state for structural warnings
    if (parsed.afterPrefix === 'off') {
      if (suppressionActive) {
        // Nested off
        const match = lineText.match(/off/);
        const offStart = match ? lineText.indexOf('off', parsed.afterPrefixStart) : parsed.afterPrefixStart;
        diagnostics.push(new vscode.Diagnostic(
          new vscode.Range(i, offStart, i, offStart + 3),
          `Nested 'cmake-fmt: off' — already in a suppressed region (started at line ${suppressionStartLine + 1})`,
          vscode.DiagnosticSeverity.Warning,
        ));
      } else {
        suppressionActive = true;
        suppressionStartLine = i;
      }
    } else if (parsed.afterPrefix === 'on') {
      if (!suppressionActive) {
        const onStart = lineText.indexOf('on', parsed.afterPrefixStart);
        diagnostics.push(new vscode.Diagnostic(
          new vscode.Range(i, onStart, i, onStart + 2),
          `'cmake-fmt: on' without matching 'cmake-fmt: off'`,
          vscode.DiagnosticSeverity.Warning,
        ));
      } else {
        suppressionActive = false;
      }
    }
  }

  // Unclosed suppression region
  if (suppressionActive) {
    const lineText = document.lineAt(suppressionStartLine).text;
    diagnostics.push(new vscode.Diagnostic(
      new vscode.Range(suppressionStartLine, 0, suppressionStartLine, lineText.length),
      `Unclosed suppression region — missing 'cmake-fmt: on'`,
      vscode.DiagnosticSeverity.Warning,
    ));
  }

  return diagnostics;
}

/**
 * Create and register the diagnostics provider for cmake-fmt directives.
 */
export function createDiagnosticsProvider(context: vscode.ExtensionContext): vscode.DiagnosticCollection {
  const collection = vscode.languages.createDiagnosticCollection('cmake-fmt');
  context.subscriptions.push(collection);

  // Analyze on document open and change
  const updateDiagnostics = (document: vscode.TextDocument) => {
    if (document.languageId !== 'cmake') {
      return;
    }
    const diagnostics = analyzeCmakeFmtDirectives(document);
    collection.set(document.uri, diagnostics);
  };

  // Analyze all currently open cmake documents
  for (const document of vscode.workspace.textDocuments) {
    updateDiagnostics(document);
  }

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(updateDiagnostics),
    vscode.workspace.onDidChangeTextDocument(e => updateDiagnostics(e.document)),
    vscode.workspace.onDidCloseTextDocument(doc => collection.delete(doc.uri)),
  );

  return collection;
}
