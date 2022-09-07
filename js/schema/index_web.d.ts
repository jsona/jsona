export { SchemaTypes } from "./index";
import type * as Module from "./index";

export default function init(input?: URL | RequestInfo): Promise<Omit<typeof Module, "SchemaTypes">>;
