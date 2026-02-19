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
    #[wasm_bindgen(js_name = "setInputSchemaJson")]
    pub fn set_input_schema_json(&mut self, schema_json: String) -> Result<(), JsValue> {
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
                diagnostics.push(jsonschema_error_to_diagnostic(
                    &input_json,
                    error.to_string(),
                ));
            }
        }

        serde_json::to_string(&ValidationResult { diagnostics }).map_err(error_to_jsvalue)
    }

    /// Return static Rego completions as LSP completion response JSON.
    pub fn completions(&self) -> Result<String, JsValue> {
        let mut items: Vec<CompletionItem> = rego_keyword_completions()
            .into_iter()
            .map(|label| CompletionItem {
                label,
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            })
            .collect();
        if let Some(schema) = &self.input_schema {
            items.extend(
                schema_property_completions(schema)
                    .into_iter()
                    .map(|label| CompletionItem {
                        label,
                        kind: Some(CompletionItemKind::FIELD),
                        ..Default::default()
                    }),
            );
        }
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

fn rego_keyword_completions() -> Vec<String> {
    [
        "package", "import", "default", "if", "contains", "some", "not", "in", "with", "data",
        "input",
    ]
    .iter()
    .map(|label| (*label).to_string())
    .collect()
}

fn schema_property_completions(schema: &JsonValue) -> Vec<String> {
    schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default()
}

fn extract_line_col(message: &str) -> Option<(u32, u32)> {
    let mut numbers = message
        .split(':')
        .filter_map(|part| part.trim().parse::<u32>().ok());
    let line = numbers.next()?.saturating_sub(1);
    let col = numbers.next()?.saturating_sub(1);
    Some((line, col))
}

fn jsonschema_error_to_diagnostic(input_json: &str, message: String) -> Diagnostic {
    let range = extract_json_pointer(&message)
        .and_then(|pointer| find_json_pointer_range(input_json, &pointer))
        .unwrap_or_else(default_range);
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("regorus-jsonschema".to_string()),
        message,
        ..Default::default()
    }
}

fn extract_json_pointer(message: &str) -> Option<String> {
    let start = message.find("\"/")?;
    let rest = &message[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn find_json_pointer_range(input_json: &str, pointer: &str) -> Option<Range> {
    let token = pointer
        .split('/')
        .next_back()?
        .replace("~1", "/")
        .replace("~0", "~");
    if token.is_empty() {
        return Some(default_range());
    }
    let key = format!("\"{token}\"");
    let start = input_json.find(&key)?;
    let end = start.saturating_add(key.len());
    Some(Range {
        start: offset_to_position(input_json, start),
        end: offset_to_position(input_json, end),
    })
}

fn offset_to_position(text: &str, offset_bytes: usize) -> Position {
    let mut line = 0_u32;
    let mut character = 0_u32;
    for (idx, ch) in text.char_indices() {
        if idx >= offset_bytes {
            break;
        }
        if ch == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(1);
        }
    }
    Position { line, character }
}

fn default_range() -> Range {
    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_json_pointer, extract_line_col, find_json_pointer_range};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[test]
    fn parses_line_and_column_from_error() {
        assert_eq!(extract_line_col("file.rego:4:9: parse error"), Some((3, 8)));
    }

    #[test]
    fn extracts_json_pointer_from_message() {
        let message = "\"/user\": 12 is not of type \"string\"";
        assert_eq!(extract_json_pointer(message).as_deref(), Some("/user"));
    }

    #[test]
    fn computes_range_from_json_pointer() {
        let input = "{\n  \"user\": 12,\n  \"active\": true\n}";
        let range = find_json_pointer_range(input, "/user");
        assert!(range.is_some());
        if let Some(range) = range {
            assert_eq!(range.start.line, 1);
        }
    }

    #[test]
    fn computes_range_with_utf8_prefix() {
        let input = "{\"π\":1,\"user\":2}";
        let range = find_json_pointer_range(input, "/user");
        assert!(range.is_some());
        if let Some(range) = range {
            assert_eq!(range.start.character, 7);
        }
    }

    #[test]
    fn validates_rego_and_input_schema() {
        let mut server = crate::lsp::RegoLanguageServer::new();
        assert!(server
            .set_input_schema_json(
                r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#
                    .to_string(),
            )
            .is_ok());
        let result = server.validateDocument(
            "file:///policy.rego".to_string(),
            "package demo\nimport rego.v1\ndefault allow := true".to_string(),
            Some(r#"{"name":1}"#.to_string()),
        );
        assert!(result.is_ok());
        let result = result.unwrap_or_else(|_| String::new());
        assert!(result.contains("regorus-jsonschema"));
    }

    #[test]
    fn completions_include_schema_properties() {
        let mut server = crate::lsp::RegoLanguageServer::new();
        assert!(server
            .set_input_schema_json(
                r#"{"type":"object","properties":{"team":{"type":"string"}}}"#.to_string()
            )
            .is_ok());
        let completions = server.completions();
        assert!(completions.is_ok());
        let completions = completions.unwrap_or_else(|_| String::new());
        assert!(completions.contains("\"team\""));
    }

    #[wasm_bindgen_test]
    fn validates_rego_and_input_schema_wasm() -> Result<(), JsValue> {
        let mut server = crate::lsp::RegoLanguageServer::new();
        server.set_input_schema_json(
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
