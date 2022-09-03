# @jsona/core

This is a JavaScript wrapper for the JSONA core.

## Install

```
npm i @jsona/core
yarn add @jsona/core
```

## Usage

```js
import Jsona from '@jsona/core';

const jsona = await Jsona.getInstance();

// parse as json
jsona.parse(jsonaContent);

// parse as ast
jsona.parseAst(jsonaContent);

// format jsona doc
jsona.format(jsonaContent);
```