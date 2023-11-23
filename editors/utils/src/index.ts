export const JSONA_EXTENSION_CONFIG = {
  name: 'jsona',
  publisher: '@jsona/monaco-components',
  version: '0.1.0',
  engines: {
    vscode: '*'
  },
  contributes: {
    languages: [{
      id: 'jsona',
      extensions: ['.jsona'],
      aliases: ['jsona', 'Jsona'],
      configuration: './jsona.configuration.json'
    }],
    grammars: [{
      language: 'jsona',
      scopeName: 'source.jsona',
      path: './jsona.grammar.json'
    }]
  }
}

export const JSONA_SCHEMA_STORE_URL = 'https://cdn.jsdelivr.net/npm/@jsona/schemastore@latest/index.json';

export const JSONA_DEFAULT_CONFIG = {
  "schema": {
    "enabled": true,
    "associations": {
    },
    "storeUrl": JSONA_SCHEMA_STORE_URL,
    "cache": false
  },
  "formatter": {
    "indentString": "  ",
    "trailingNewline": false,
    "trailingComma": false,
    "formatKey": false
  }
}