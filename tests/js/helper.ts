import fs from "node:fs"
import path from "node:path"

export const FIXTUREs_DIR = path.resolve(__dirname, "../fixtures");

export function resolveFixture(file: string) {
    return path.resolve(FIXTUREs_DIR, file);
}

export function readFixtureSync(file: string) {
  return fs.readFileSync(resolveFixture(file), "utf-8");
}
