const { parseJson, parseJsona } = require("../pkg");
const fs = require("fs");
const path = require("path");
const assert = require("assert");

const target = fs.readFileSync(path.resolve(__dirname, "./spec/simple_openapi.jsona"), "utf8");
const expectJson = require("./spec/simple_openapi.json")
const expectJsona = require("./spec/simple_openapi_jsona.json")

assert.deepStrictEqual(parseJsona(target), expectJsona);
assert.deepStrictEqual(parseJson(target), expectJson);