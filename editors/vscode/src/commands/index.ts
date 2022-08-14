import * as vscode from "vscode";
import * as schemaCommands from "./schema";
import { BaseLanguageClient } from "vscode-languageclient";


export function registerCommands(
  ctx: vscode.ExtensionContext,
  c: BaseLanguageClient
) {
  schemaCommands.register(ctx, c);
}
