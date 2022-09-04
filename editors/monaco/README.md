# monaco-jsona

JSONA language plugin for the Monaco Editor. It provides the following features when editing JSONA files:

- Code completion, based on JSONA schemas or by looking at similar objects in the same file
- Hovers, based on JSON schemas
- Validation: Syntax errors and schema validation
- Formatting using Prettier
- Document Symbols

## Install

```
npm i monaco-jsona
yarn add monaco-jsona
```

## Usage

Import monaco-jsona and configure it before an editor instance is created.

```js
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api';
import { jsonaDefaults } from 'monaco-jsona';

jsonaDefaults.setOptions({
  schema: {
    enabled: true,
    associations: {},
    storeUrl: "https://cdn.jsdelivr.net/npm/@jsona/schemastore@latest/index.json",
    cache: false
  },
  formatter: {
    indentString: "  ",
    trailingNewline: false,
    trailingComma: false,
    formatKey: false
  }
});

monaco.editor.create(document.getElementById('container'), {
  model: monaco.editor.createModel(
    `{ @jsonaschema("schema")
  value: { @pattern(".*")
  }
}`,
    'jsona',
    monaco.Uri.parse("inmemory:///demo.jsona")
  ),
});
```

Also make sure to register the web worker. When using Webpack 5, this looks like the code below. Other bundlers may use a different syntax, but the idea is the same. Languages you don’t used can be omitted.

```js
window.MonacoEnvironment = {
  getWorker(moduleId, label) {
    switch (label) {
      case 'editorWorkerService':
        return new Worker(new URL('monaco-editor/esm/vs/editor/editor.worker', import.meta.url));
      case 'css':
      case 'less':
      case 'scss':
        return new Worker(new URL('monaco-editor/esm/vs/language/css/css.worker', import.meta.url));
      case 'handlebars':
      case 'html':
      case 'razor':
        return new Worker(
          new URL('monaco-editor/esm/vs/language/html/html.worker', import.meta.url),
        );
      case 'json':
        return new Worker(
          new URL('monaco-editor/esm/vs/language/json/json.worker', import.meta.url),
        );
      case 'javascript':
      case 'typescript':
        return new Worker(
          new URL('monaco-editor/esm/vs/language/typescript/ts.worker', import.meta.url),
        );
      case 'yaml':
        return new Worker(new URL('monaco-jsona/jsona.worker', import.meta.url));
      default:
        throw new Error(`Unknown label ${label}`);
    }
  },
};

```