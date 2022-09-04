import { MonacoLanguageClient, CloseAction, ErrorAction, MonacoServices, MessageTransports } from 'monaco-languageclient';
import { BrowserMessageReader, BrowserMessageWriter } from 'vscode-languageserver-protocol/browser';
import { StandaloneServices } from 'vscode/services';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api.js';
import type { languages } from "monaco-editor";
import getDialogsServiceOverride from 'vscode/service-override/dialogs'
import getNotificationServiceOverride from 'vscode/service-override/notifications'

export const languageId = "jsona";

const languageConfiguration: languages.LanguageConfiguration = {
	wordPattern:
		/(-?\d*\.\d\w*)|([^\`\~\!\@\#\%\^\&\*\(\)\-\=\+\[\{\]\}\\\|\;\:\'\"\,\.\<\>\/\?\s]+)/g,

	comments: {
		lineComment: '//',
		blockComment: ['/*', '*/']
	},

	brackets: [
		['{', '}'],
		['[', ']'],
		['(', ')']
	],

	autoClosingPairs: [
		{ open: '{', close: '}' },
		{ open: '[', close: ']' },
		{ open: '(', close: ')' },
		{ open: '"', close: '"', notIn: ['string'] },
		{ open: "'", close: "'", notIn: ['string', 'comment'] },
		{ open: '`', close: '`', notIn: ['string', 'comment'] },
		{ open: '/**', close: ' */', notIn: ['string'] }
	],

	folding: {
		markers: {
			start: new RegExp('^\\s*//\\s*#?region\\b'),
			end: new RegExp('^\\s*//\\s*#?endregion\\b')
		}
	}
};

export const monarchLanguage: languages.IMonarchLanguage = {
	// Set defaultToken to invalid to see what you do not tokenize yet
	defaultToken: 'invalid',
	tokenPostfix: '.ts',
	keywords: [
    "null",
    "true",
    "false",
  ],

	// we include these common regular expressions
	escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,
	digits: /\d+(_+\d+)*/,
	octaldigits: /[0-7]+(_+[0-7]+)*/,
	binarydigits: /[0-1]+(_+[0-1]+)*/,
	hexdigits: /[[0-9a-fA-F]+(_+[0-9a-fA-F]+)*/,

	// The main tokenizer for our languages
	tokenizer: {
		root: [[/[{}]/, 'delimiter.bracket'], { include: 'common' }],

		common: [
			// identifiers and keywords
			[
				/[A-Za-z_]\w*/,
				{
					cases: {
						'@keywords': 'keyword',
						'@default': 'identifier'
					}
				}
			],

			// whitespace
			{ include: '@whitespace' },

			[/[()\[\]]/, '@brackets'],

			// @ annotations.
			[/@\s*[a-zA-Z_\$][\w\$]*/, 'annotation'],

			// numbers
			[/(@digits)[eE]([\-+]?(@digits))?/, 'number.float'],
			[/(@digits)\.(@digits)([eE][\-+]?(@digits))?/, 'number.float'],
			[/0[xX](@hexdigits)n?/, 'number.hex'],
			[/0[oO]?(@octaldigits)n?/, 'number.octal'],
			[/0[bB](@binarydigits)n?/, 'number.binary'],
			[/(@digits)n?/, 'number'],

			// delimiter
			[/[;,]/, 'delimiter'],

			// strings
			[/"([^"\\]|\\.)*$/, 'string.invalid'], // non-teminated string
			[/'([^'\\]|\\.)*$/, 'string.invalid'], // non-teminated string
			[/"/, 'string', '@string_double'],
			[/'/, 'string', '@string_single'],
			[/`/, 'string', '@string_backtick']
		],

		whitespace: [
			[/[ \t\r\n]+/, ''],
			[/\/\*/, 'comment', '@comment'],
			[/\/\/.*$/, 'comment']
		],

		comment: [
			[/[^\/*]+/, 'comment'],
			[/\*\//, 'comment', '@pop'],
			[/[\/*]/, 'comment']
		],

		string_double: [
			[/[^\\"]+/, 'string'],
			[/@escapes/, 'string.escape'],
			[/\\./, 'string.escape.invalid'],
			[/"/, 'string', '@pop']
		],

		string_single: [
			[/[^\\']+/, 'string'],
			[/@escapes/, 'string.escape'],
			[/\\./, 'string.escape.invalid'],
			[/'/, 'string', '@pop']
		],

		string_backtick: [
			[/\$\{/, { token: 'delimiter.bracket', next: '@bracketCounting' }],
			[/[^\\`$]+/, 'string'],
			[/@escapes/, 'string.escape'],
			[/\\./, 'string.escape.invalid'],
			[/`/, 'string', '@pop']
		],

		bracketCounting: [
			[/\{/, 'delimiter.bracket', '@bracketCounting'],
			[/\}/, 'delimiter.bracket', '@pop'],
			{ include: 'common' }
		]
	}
};

const DEFAULT_OPTIONS = {
  "schema": {
    "enabled": true,
    "associations": {
    },
    "storeUrl": "https://cdn.jsdelivr.net/npm/@jsona/schemastore@latest/index.json",
    "cache": false
  },
  "formatter": {
    "indentString": "  ",
    "trailingNewline": false,
    "trailingComma": false,
    "formatKey": false
  }
}
type RecursivePartial<T> = {
  [P in keyof T]?:
  T[P] extends (infer U)[] ? RecursivePartial<U>[] :
  T[P] extends object ? RecursivePartial<T[P]> :
  T[P];
};

type LanguageServiceOptions = RecursivePartial<typeof DEFAULT_OPTIONS>;

function createLanguageServiceDefaults(
  initialOptions: LanguageServiceOptions,
) {
  const onDidChange = new monaco.Emitter();
  let currentOptions = initialOptions;
  const languageServiceDefaults = {
    get onDidChange() {
      return onDidChange.event;
    },

    get options() {
      return currentOptions;
    },

    setOptions(options: LanguageServiceOptions) {
      currentOptions = merge(currentOptions, options);
      onDidChange.fire(languageServiceDefaults);
    },
  };

  return languageServiceDefaults;
}

export const jsonaDefaults = createLanguageServiceDefaults(DEFAULT_OPTIONS);

console.log(`register ${languageId}`);
monaco.languages.register({
	id: languageId,
	extensions: ['.jsona'],
	aliases: ['JSONA', 'Jsona', 'jsona'],
	mimetypes: ['text/x-jsona'],
});
monaco.languages.onLanguage(languageId, setupMode);
StandaloneServices.initialize({
  ...getDialogsServiceOverride(),
  ...getNotificationServiceOverride(),
});
MonacoServices.install();

function setupMode() {
  monaco.languages.setMonarchTokensProvider(languageId, monarchLanguage);
  monaco.languages.setLanguageConfiguration(languageId, languageConfiguration);
  const worker = getWorker(globalThis);
  const reader = new BrowserMessageReader(worker);
  const writer = new BrowserMessageWriter(worker);
  const languageClient = createLanguageClient({ reader, writer });
  jsonaDefaults.onDidChange(() => {
    languageClient.sendNotification("workspace/didChangeConfiguration", { settings: null });
  })
  languageClient.onRequest("workspace/configuration", async (parmas) => {
    return Array.from(Array(parmas.length)).map(() => jsonaDefaults.options);
  });
  languageClient.start();
  reader.onClose(() => languageClient.stop());
}

function createLanguageClient(transports: MessageTransports): MonacoLanguageClient {
  return new MonacoLanguageClient({
    name: 'JSONA Language Server',
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

function getWorker(globalObj: any): Worker {
  // Option for hosts to overwrite the worker script (used in the standalone editor)
  if (globalObj.MonacoEnvironment) {
    if (typeof globalObj.MonacoEnvironment.getWorker === 'function') {
      return globalObj.MonacoEnvironment.getWorker(null, languageId);
    }
    if (typeof globalObj.MonacoEnvironment.getWorkerUrl === 'function') {
      const workerUrl = globalObj.MonacoEnvironment.getWorkerUrl(null, languageId);
      return new Worker(workerUrl);
    }
  }
  throw new Error(`You must define a function MonacoEnvironment.getWorkerUrl or MonacoEnvironment.getWorker`);
}

function merge(target: any, source: any) {
  for (const key of Object.keys(source)) {
    if (source[key] instanceof Object) Object.assign(source[key], merge(target[key], source[key]))
  }

  Object.assign(target || {}, source)
  return target
}