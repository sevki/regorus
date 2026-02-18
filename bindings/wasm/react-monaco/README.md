# React Monaco custom JSON schema example

`CustomSchemaMonacoExample.jsx` shows a minimal `@monaco-editor/react` integration using `RegoLanguageServer` from the wasm package with custom JSON Schema validation:

```jsx
import regorus from "../pkg/regorusjs";
import CustomSchemaMonacoExample from "./CustomSchemaMonacoExample.jsx";

export default function App() {
  return <CustomSchemaMonacoExample regorus={regorus} />;
}
```

The example calls `validateWithCustomSchema(...)`, which sets `setInputSchemaJson(...)` and then runs `validateDocument(...)` so schema validation errors can be shown as Monaco markers.
