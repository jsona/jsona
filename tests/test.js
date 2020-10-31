const { parseJson, parseAst } = require("../pkg");
const fs = require("fs");
const path = require("path");
const assert = require("assert");

const target = fs.readFileSync(path.resolve(__dirname, "./spec/simple_openapi.jsona"), "utf8");
const expectJson = require("./spec/simple_openapi.json")
const expectAst = require("./spec/simple_openapi_ast.json")

// console.log(JSON.stringify(parseAst(target), null, 2));
// console.log(JSON.stringify(parseJson(target), null, 2));
assert.deepStrictEqual(parseAst(target), expectAst);
assert.deepStrictEqual(parseJson(target), expectJson);