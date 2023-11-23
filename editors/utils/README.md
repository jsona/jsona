# Jsona editor utils

JSONA language plugin for the Monaco Editor. It provides the following features when editing JSONA files:

- Code completion, based on JSONA schemas or by looking at similar objects in the same file
- Hovers, based on JSON schemas
- Validation: Syntax errors and schema validation
- Formatting using Prettier
- Document Symbols

## Install

```
npm i @jsona/editor-utils
yarn add @jsona/editor-utils
```

## Example

```js
import { editor } from 'monaco-editor';
import { MonacoLanguageClient, initServices, useOpenEditorStub } from 'monaco-languageclient';
import { BrowserMessageReader, BrowserMessageWriter } from 'vscode-languageserver-protocol/browser.js';
import { CloseAction, ErrorAction, MessageTransports } from 'vscode-languageclient';
import { createConfiguredEditor } from 'vscode/monaco';
import { ExtensionHostKind, registerExtension } from 'vscode/extensions';
import getConfigurationServiceOverride, { updateUserConfiguration } from '@codingame/monaco-vscode-configuration-service-override';
import getEditorServiceOverride from '@codingame/monaco-vscode-editor-service-override';
import getKeybindingsServiceOverride from '@codingame/monaco-vscode-keybindings-service-override';
import getThemeServiceOverride from '@codingame/monaco-vscode-theme-service-override';
import getTextmateServiceOverride from '@codingame/monaco-vscode-textmate-service-override';
import { LogLevel } from 'vscode/services';
import { JSONA_EXTENSION_CONFIG, JSONA_SCHEMA_STORE_URL } from '@jsona/editor-utils';
import { Uri } from 'vscode';

import '@codingame/monaco-vscode-theme-defaults-default-extension';

export async function setupJsonaClient() {
    const serviceConfig = {
        userServices: {
            ...getThemeServiceOverride(),
            ...getTextmateServiceOverride(),
            ...getConfigurationServiceOverride(),
            ...getEditorServiceOverride(useOpenEditorStub),
            ...getKeybindingsServiceOverride()
        },
        debugLogging: true,
        logLevel: LogLevel.Info
    };
    await initServices(serviceConfig);

    const { registerFileUrl } = registerExtension(JSONA_EXTENSION_CONFIG, ExtensionHostKind.LocalProcess);

    registerFileUrl('/jsona.configuration.json', new URL('../../node_modules/@jsona/editor-utils/jsona.configuration.json', window.location.href).href);
    registerFileUrl('/jsona.grammar.json', new URL('../../node_modules/@jsona/editor-utils/jsona.grammar.json', window.location.href).href);

    updateUserConfiguration(`{
    "editor.fontSize": 14,
    "workbench.colorTheme": "Default Dark Modern"
    "jsona.schema.enabled": true,
    "jsona.schema.storeUrl": "${JSONA_SCHEMA_STORE_URL}",
}`);

    const languageId = 'jsona';
    const exampleJsonaUrl = new URL('./src/jsona/example.jsona', window.location.href).href;
    const editorText = await (await fetch(exampleJsonaUrl)).text();

    const editorOptions = {
        model: editor.createModel(editorText, languageId, Uri.parse('/workspace/example.jsona')),
        automaticLayout: true
    };
    createConfiguredEditor(document.getElementById('container')!, editorOptions);

    function createLanguageClient(transports: MessageTransports): MonacoLanguageClient {
        return new MonacoLanguageClient({
            name: 'Jsona Client',
            clientOptions: {
                // use a language id as a document selector
                documentSelector: [{ language: languageId }],
                // disable the default error handler
                errorHandler: {
                    error: () => ({ action: ErrorAction.Continue }),
                    closed: () => ({ action: CloseAction.DoNotRestart })
                }
            },
            // create a language client connection to the server running in the web worker
            connectionProvider: {
                get: () => {
                    return Promise.resolve(transports);
                }
            }
        });
    }

    const workerUrl = new URL('../../node_modules/@jsona/editor-utils/jsona.worker.js', window.location.href).href;
    const worker = new Worker(workerUrl, {
        type: 'module',
        name: 'Jsona Language Server'
    });
    const reader = new BrowserMessageReader(worker);
    const writer = new BrowserMessageWriter(worker);
    const languageClient = createLanguageClient({ reader, writer });
    languageClient.start();
    reader.onClose(() => languageClient.stop());
}
```