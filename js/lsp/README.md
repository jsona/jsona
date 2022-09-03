# @jsona/lsp

This is a JavaScript wrapper for the JSONA lsp.

## Install

```
npm i @jsona/lsp
yarn add @jsona/lsp
```

## Usage

```js
import JsonaLsp from '@jsona/lsp';

const worker: Worker = self as any;

const writer = new BrowserMessageWriter(worker);
const reader = new BrowserMessageReader(worker);

const lsp = await Jsona.getInstance(env, { 
  onMessage(message) {
    // from lsp server to host
    writer.send(message) 
}});

reader.listen(async (message: RpcMessage) => {
  // from host to lsp server
  lsp.send(message)
}
```