import { parse, parseAst } from "@jsona/core";
import { readFixtureSync } from "./helper";

const FILES = [
  "spec.jsona"
];

FILES.forEach(name => {
  test(`parse ${name}`, () => {
    const content = readFixtureSync(name)
    expect(parse(content)).toMatchSnapshot();
  })
})

FILES.forEach(name => {
  test(`parseAst ${name}`, () => {
    const content = readFixtureSync(name)
    expect(parseAst(content)).toMatchSnapshot();
  })
})