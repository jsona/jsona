import * as vscode from "vscode";
import { BaseLanguageClient } from "vscode-languageclient";

let output: vscode.OutputChannel;

export function getOutput(): vscode.OutputChannel {
  if (!output) {
    output = vscode.window.createOutputChannel("JSONA");
  }

  return output;
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
