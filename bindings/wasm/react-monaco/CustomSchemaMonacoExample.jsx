import React, { useEffect, useMemo, useState } from "react";
import Editor from "@monaco-editor/react";

import {
  validateWithCustomSchema
} from "./languageServerAdapter.js";

const DEFAULT_SCHEMA = JSON.stringify(
  {
    type: "object",
    properties: {
      user: { type: "string" },
      active: { type: "boolean" }
    },
    required: ["user", "active"]
  },
  null,
  2
);

const DEFAULT_POLICY = `package demo
import rego.v1

allow if {
  input.user == "alice"
  input.active == true
}`;

const DEFAULT_INPUT = `{
  "user": 12,
  "active": true
}`;

export default function CustomSchemaMonacoExample({ regorus }) {
  const server = useMemo(() => new regorus.RegoLanguageServer(), [regorus]);
  const [policy, setPolicy] = useState(DEFAULT_POLICY);
  const [input, setInput] = useState(DEFAULT_INPUT);
  const [schema, setSchema] = useState(DEFAULT_SCHEMA);
  const [diagnostics, setDiagnostics] = useState([]);
  const [error, setError] = useState("");

  useEffect(() => () => {
    if (typeof server.dispose === "function") {
      server.dispose();
    }
  }, [server]);

  async function runValidation() {
    try {
      const markers = await validateWithCustomSchema(
        server,
        "file:///policy.rego",
        policy,
        input,
        schema
      );
      setDiagnostics(markers);
      setError("");
    } catch (validationError) {
      setDiagnostics([]);
      setError(String(validationError));
    }
  }

  return (
    <div style={{ display: "grid", gap: 12 }}>
      <h3>Regorus + Monaco custom JSON Schema validation</h3>
      <Editor
        height="180px"
        defaultLanguage="rego"
        value={policy}
        onChange={(next) => setPolicy(next ?? "")}
      />
      <Editor
        height="140px"
        defaultLanguage="json"
        value={input}
        onChange={(next) => setInput(next ?? "")}
      />
      <Editor
        height="180px"
        defaultLanguage="json"
        value={schema}
        onChange={(next) => setSchema(next ?? "")}
      />
      <button type="button" onClick={runValidation}>
        Validate policy + input
      </button>
      {error ? <pre>{error}</pre> : null}
      <pre>{JSON.stringify(diagnostics, null, 2)}</pre>
    </div>
  );
}
