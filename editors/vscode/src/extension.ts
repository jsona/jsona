import type { Lsp } from "@jsona/lsp";
import * as vscode from "vscode";
import { createClient } from "./client";
import { registerCommands } from "./commands";
import { showMessage, getOutput } from "./util";
import { BaseLanguageClient } from "vscode-languageclient";


export async function activate(context: vscode.ExtensionContext) {
  const schemaIndicator = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    0
  );
  resetSchemaIndicator(schemaIndicator);

  const c = await createClient(context);
  await c.start()
 
  registerCommands(context, c);
  vscode.commands.executeCommand("setContext", "jsona.extensionActive", true);
  context.subscriptions.push(
    getOutput(),
    schemaIndicator,
    vscode.window.onDidChangeActiveTextEditor(async editor => {
      updateSchemaIndicator(c, editor, schemaIndicator);
    }),
    c.onNotification(
      "jsona/initializeWorkspace",
      (params: Lsp.Server.NotificationParams<"jsona/initializeWorkspace">) => {
        let editor = vscode.window.activeTextEditor;
        if (editor?.document.uri.toString().startsWith(params.rootUri)) {
          updateSchemaIndicator(c, editor, schemaIndicator);
        }
      }
    ),
    c.onNotification("jsona/messageWithOutput", async params =>
      showMessage(params, c)
    ),
    c.onRequest("fs/readFile", ({ fsPath }) => {
      const folderUri = vscode.workspace.workspaceFolders[0].uri;
      let fileUri = vscode.Uri.file(fsPath);
      if (folderUri.scheme !== "file") {
        fileUri = vscode.Uri.joinPath(folderUri, fsPath);
      }
      return vscode.workspace.fs.readFile(fileUri);
    }),
    {
      dispose: () => {
        vscode.commands.executeCommand(
          "setContext",
          "jsona.extensionActive",
          false
        );
      },
    }
  );
}

async function updateSchemaIndicator(c: BaseLanguageClient, editor: vscode.TextEditor, schemaIndicator: vscode.StatusBarItem) {
    if (editor?.document.languageId === "jsona") {
      let documentUrl = editor?.document.uri;
      if (documentUrl) {
        const res = await c.sendRequest("jsona/associatedSchema", {
          documentUri: documentUrl.toString(),
        }) as Lsp.Client.RequestResponse<"jsona/associatedSchema">;
        if (res?.schema?.url) {
          let schema = res.schema;
          schemaIndicator.text =
            schema.meta?.name ?? schema.url?.split("/").slice(-1)[0] ?? "no schema";
          schemaIndicator.tooltip = `JSONA Schema: ${schema.url}`;
        } else {
          resetSchemaIndicator(schemaIndicator);
        }
      }
      schemaIndicator.show();
    } else {
      schemaIndicator.hide();
    }
}

function resetSchemaIndicator(schemaIndicator: vscode.StatusBarItem) {
  schemaIndicator.text = "no schema";
  schemaIndicator.tooltip = "Select JSONA Schema";
  schemaIndicator.command = "jsona.selectSchema";
}
