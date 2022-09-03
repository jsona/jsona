# @jsona/schema

This is a JavaScript wrapper for the JSONA schema.

## Install

```
npm i @jsona/schema
yarn add @jsona/schema
```

## Usage

```js
import JsonaSchema from '@jsona/schema';

const jsonaSchema = await JsonaSchema.getInstance();

// parse as jsonschema
jsonaSchema.parse(jsonaContent);
```