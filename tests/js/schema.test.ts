import { parse } from "@jsona/schema";
import { readFixtureSync } from "./helper";

const FILES = [
  "schema.jsona"
];

FILES.forEach(name => {
  test(`parse ${name}`, () => {
    const content = readFixtureSync(name)
    expect(parse(content)).toMatchSnapshot();
  })
})
