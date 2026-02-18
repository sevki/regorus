export function diagnosticsToMonacoMarkers(diagnostics) {
  return diagnostics.map((diagnostic) => ({
    severity: 8,
    message: diagnostic.message,
    startLineNumber: diagnostic.range.start.line + 1,
    startColumn: diagnostic.range.start.character + 1,
    endLineNumber: diagnostic.range.end.line + 1,
    endColumn: diagnostic.range.end.character + 1
  }));
}

export async function validateWithRegoLanguageServer(server, uri, rego, inputJson) {
  const resultJson = server.validateDocument(uri, rego, inputJson ?? null);
  const result = JSON.parse(resultJson);
  return diagnosticsToMonacoMarkers(result.diagnostics);
}
