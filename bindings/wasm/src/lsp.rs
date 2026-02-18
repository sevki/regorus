// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use serde::Serialize;
use serde_json::Value as JsonValue;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Diagnostic, DiagnosticSeverity,
    InitializeResult, Position, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};
use wasm_bindgen::prelude::*;

fn error_to_jsvalue<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&format!("{e}"))
}

#[derive(Serialize)]
struct ValidationResult {
    diagnostics: Vec<Diagnostic>,
}

#[wasm_bindgen]
/// Minimal LSP-compatible helper for Monaco integrations.
pub struct RegoLanguageServer {
    input_schema: Option<JsonValue>,
}

#[wasm_bindgen]
impl RegoLanguageServer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { input_schema: None }
    }

    /// Return LSP initialize result JSON using tower-lsp types.
    pub fn initialize(&self) -> Result<String, JsValue> {
        let result = InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(Default::default()),
                ..Default::default()
            },
            ..Default::default()
        };
        serde_json::to_string(&result).map_err(error_to_jsvalue)
    }

    /// Set a custom JSON schema used to validate input documents.
    pub fn setInputSchemaJson(&mut self, schema_json: String) -> Result<(), JsValue> {
        let schema = serde_json::from_str(&schema_json).map_err(error_to_jsvalue)?;
        jsonschema::validator_for(&schema).map_err(error_to_jsvalue)?;
        self.input_schema = Some(schema);
        Ok(())
    }

    /// Validate a Rego document and optional input document using schema validation.
    pub fn validateDocument(
        &self,
        uri: String,
        rego: String,
        input_json: Option<String>,
    ) -> Result<String, JsValue> {
        let mut diagnostics = Vec::new();

        let mut engine = regorus::Engine::new();
        if let Err(err) = engine.add_policy(uri.clone(), rego) {
            diagnostics.push(policy_error_to_diagnostic(&uri, &err.to_string()));
        }

        if let (Some(schema), Some(input_json)) = (&self.input_schema, input_json) {
            let input: JsonValue = serde_json::from_str(&input_json).map_err(error_to_jsvalue)?;
            let validator = jsonschema::validator_for(schema).map_err(error_to_jsvalue)?;
            for error in validator.iter_errors(&input) {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 1,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("regorus-jsonschema".to_string()),
                    message: error.to_string(),
                    ..Default::default()
                });
            }
        }

        serde_json::to_string(&ValidationResult { diagnostics }).map_err(error_to_jsvalue)
    }

    /// Return static Rego completions as LSP completion response JSON.
    pub fn completions(&self) -> Result<String, JsValue> {
        let items = ["package", "import", "default", "if", "data", "input"]
            .iter()
            .map(|label| CompletionItem {
                label: (*label).to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            })
            .collect();
        serde_json::to_string(&CompletionResponse::Array(items)).map_err(error_to_jsvalue)
    }
}

impl Default for RegoLanguageServer {
    fn default() -> Self {
        Self::new()
    }
}

fn policy_error_to_diagnostic(uri: &str, message: &str) -> Diagnostic {
    let (line, character) = extract_line_col(message).unwrap_or((0, 0));
    Diagnostic {
        range: Range {
            start: Position { line, character },
            end: Position {
                line,
                character: character.saturating_add(1),
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("regorus".to_string()),
        message: format!("{uri}: {message}"),
        ..Default::default()
    }
}

fn extract_line_col(message: &str) -> Option<(u32, u32)> {
    let mut numbers = message
        .split(':')
        .filter_map(|part| part.trim().parse::<u32>().ok());
    let line = numbers.next()?.saturating_sub(1);
    let col = numbers.next()?.saturating_sub(1);
    Some((line, col))
}

#[cfg(test)]
mod tests {
    use super::extract_line_col;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[test]
    fn parses_line_and_column_from_error() {
        assert_eq!(extract_line_col("file.rego:4:9: parse error"), Some((3, 8)));
    }

    #[test]
    fn validates_rego_and_input_schema() {
        let mut server = crate::lsp::RegoLanguageServer::new();
        let schema_result = server.setInputSchemaJson(
            r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#
                .to_string(),
        );
        assert!(schema_result.is_ok());
        let result = server.validateDocument(
            "file:///policy.rego".to_string(),
            "package demo\nimport rego.v1\ndefault allow := true".to_string(),
            Some(r#"{"name":1}"#.to_string()),
        );
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|_| String::new());
        assert!(result.contains("regorus-jsonschema"));
    }

    #[wasm_bindgen_test]
    fn validates_rego_and_input_schema_wasm() -> Result<(), JsValue> {
        let mut server = crate::lsp::RegoLanguageServer::new();
        server.setInputSchemaJson(
            r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#
                .to_string(),
        )?;
        let result = server.validateDocument(
            "file:///policy.rego".to_string(),
            "package demo\nimport rego.v1\ndefault allow := true".to_string(),
            Some(r#"{"name":1}"#.to_string()),
        )?;

        assert!(result.contains("regorus-jsonschema"));
        Ok(())
    }
}
