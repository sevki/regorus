import assert from "node:assert/strict";
import test from "node:test";

import {
  diagnosticsToMonacoMarkers,
  validateWithCustomSchema,
  validateWithRegoLanguageServer
} from "./languageServerAdapter.js";

test("converts LSP diagnostics to Monaco markers", () => {
  const diagnostics = [
    {
      message: "boom",
      range: {
        start: { line: 0, character: 1 },
        end: { line: 0, character: 4 }
      }
    }
  ];
  const markers = diagnosticsToMonacoMarkers(diagnostics);
  assert.equal(markers.length, 1);
  assert.equal(markers[0].startLineNumber, 1);
  assert.equal(markers[0].startColumn, 2);
});

test("uses wasm language server output", async () => {
  const fakeServer = {
    validateDocument() {
      return JSON.stringify({
        diagnostics: [
          {
            message: "schema mismatch",
            range: {
              start: { line: 1, character: 0 },
              end: { line: 1, character: 2 }
            }
          }
        ]
      });
    }
  };

  const markers = await validateWithRegoLanguageServer(
    fakeServer,
    "file:///policy.rego",
    "package demo",
    "{\"name\":1}"
  );
  assert.equal(markers.length, 1);
  assert.equal(markers[0].startLineNumber, 2);
});

test("sets custom input schema before validation", async () => {
  const calls = [];
  const fakeServer = {
    setInputSchemaJson(schemaJson) {
      calls.push(["setInputSchemaJson", schemaJson]);
    },
    validateDocument(uri, rego, input) {
      calls.push(["validateDocument", uri, rego, input]);
      return JSON.stringify({ diagnostics: [] });
    }
  };

  await validateWithCustomSchema(
    fakeServer,
    "file:///policy.rego",
    "package demo",
    "{\"name\":1}",
    "{\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"}},\"required\":[\"name\"]}"
  );

  assert.equal(calls[0][0], "setInputSchemaJson");
  assert.equal(calls[1][0], "validateDocument");
});
