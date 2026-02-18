const MONACO_SEVERITY_ERROR = 8;

export function diagnosticsToMonacoMarkers(diagnostics) {
  return diagnostics.map((diagnostic) => ({
    severity: MONACO_SEVERITY_ERROR,
    message: diagnostic.message,
    startLineNumber: diagnostic.range.start.line + 1,
    startColumn: diagnostic.range.start.character + 1,
    endLineNumber: diagnostic.range.end.line + 1,
    endColumn: diagnostic.range.end.character + 1
  }));
}

export function validateWithRegoLanguageServer(server, uri, rego, inputJson) {
  const resultJson = server.validateDocument(uri, rego, inputJson ?? null);
  const result = JSON.parse(resultJson);
  return diagnosticsToMonacoMarkers(result.diagnostics);
}

export function validateWithCustomSchema(
  server,
  uri,
  rego,
  inputJson,
  schemaJson
) {
  if (schemaJson) {
    server.setInputSchemaJson(schemaJson);
  }
  return validateWithRegoLanguageServer(server, uri, rego, inputJson);
}
