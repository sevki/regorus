import assert from "node:assert/strict";
import test from "node:test";

import {
  diagnosticsToMonacoMarkers,
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
