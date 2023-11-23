# monaco-jsona

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

## Usage

```js
import getEditorServiceOverride from '@codingame/monaco-vscode-editor-service-override';
import getKeybindingsServiceOverride from '@codingame/monaco-vscode-keybindings-service-override';
import { JSONA_EXTENSION_CONFIG, JSONA_USER_CONFIGURATION } from '@jsona/editor-utils';
import { useOpenEditorStub } from 'monaco-languageclient';
import { UserConfig } from 'monaco-editor-wrapper';

export function createJsonaGlobalConfig(code: string): UserConfig {
    const jsonaConfigurationUrl = new URL('../node_modules/@jsona/editor-utils/jsona.configuration.json', window.location.href);
    const jsonaGrammarUrl = new URL('../node_modules/@jsona/editor-utils/jsona.grammar.json', window.location.href);
    const jsonaWorkerUrl = new URL('../node_modules/@jsona/editor-utils/jsona.worker.js', window.location.href);

    const jsonaWorker = new Worker(jsonaWorkerUrl, {
        type: 'module',
        name: 'JSONA worker',
    });

    const extensionFilesOrContents = new Map<string, string | URL>();
    extensionFilesOrContents.set('/jsona.configuration.json', jsonaConfigurationUrl);
    extensionFilesOrContents.set('/jsona.grammar.json', jsonaGrammarUrl);

    return {
        wrapperConfig: {
            serviceConfig: {
                userServices: {
                    ...getEditorServiceOverride(useOpenEditorStub),
                    ...getKeybindingsServiceOverride()
                },
                debugLogging: true
            },
            editorAppConfig: {
                $type: 'extended',
                languageId: 'jsona',
                code,
                useDiffEditor: false,
                extensions: [{
                    config: JSONA_EXTENSION_CONFIG,
                    filesOrContents: extensionFilesOrContents
                }],
                userConfiguration: {
                    json: JSON.stringify({
                        ...JSONA_USER_CONFIGURATION,
                    })
                }
            }
        },
        languageClientConfig: {
            options: {
                $type: 'WorkerDirect',
                worker: jsonaWorker
            }
        }
    };
}
```