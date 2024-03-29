import * as vscode from "vscode";
import { BaseLanguageClient } from "vscode-languageclient";

let output: vscode.OutputChannel;

export const ID = "JSONA";
export const NAME = "JSONA Language Server";
export const SCHEMA_CACHE_KEY = "jsona.schema.cache"

export function getOutput(): vscode.OutputChannel {
  if (!output) {
    output = vscode.window.createOutputChannel(NAME);
  }

  return output;
}

export function toBase64(u8: ArrayBufferLike) {
  return btoa(String.fromCharCode.apply(null, u8));
}

export function fromBase64(str) {
  return atob(str).split('').map(function (c) { return c.charCodeAt(0); });
}

export async function showMessage(
  params: { kind: "info" | "warn" | "error"; message: string },
  c: BaseLanguageClient
) {
  let show: string | undefined;
  switch (params.kind) {
    case "info":
      show = await vscode.window.showInformationMessage(
        params.message,
        "Show Details"
      );
    case "warn":
      show = await vscode.window.showWarningMessage(
        params.message,
        "Show Details"
      );
    case "error":
      show = await vscode.window.showErrorMessage(
        params.message,
        "Show Details"
      );
  }

  if (show) {
    c.outputChannel.show();
  }
}
