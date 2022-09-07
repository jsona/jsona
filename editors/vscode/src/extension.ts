import { ClientRpcResponses, ServerNotificationsParams } from "@jsona/lsp";
import * as vscode from "vscode";
import { createClient, syncSchemaCache } from "./client";
import { registerCommands } from "./commands";
import { showMessage, getOutput, SCHEMA_CACHE_KEY } from "./util";
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
    vscode.workspace.onDidChangeConfiguration(e => {
      if (e.affectsConfiguration(SCHEMA_CACHE_KEY)) {
        syncSchemaCache(context);
      }
    }),
    vscode.window.onDidChangeActiveTextEditor(async editor => {
      updateSchemaIndicator(c, editor, schemaIndicator);
    }),
    c.onNotification(
      "jsona/initializeWorkspace",
      (params: ServerNotificationsParams<"jsona/initializeWorkspace">) => {
        let editor = vscode.window.activeTextEditor;
        if (editor?.document.uri.toString().startsWith(params.rootUri)) {
          updateSchemaIndicator(c, editor, schemaIndicator);
        }
      }
    ),
    c.onNotification("jsona/messageWithOutput", async params =>
      showMessage(params, c)
    ),
    c.onRequest("lsp/readFile", ({ uri }) => {
      return vscode.workspace.fs.readFile(vscode.Uri.parse(uri));
    }),
    c.onRequest("lsp/writeFile", ({ uri, content }) => {
      return vscode.workspace.fs.writeFile(vscode.Uri.parse(uri), content);
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
        }) as ClientRpcResponses<"jsona/associatedSchema">;
        let url = res?.schema?.url;
        if (url) {
          schemaIndicator.text = url.split("/").slice(-1)[0]?.replace(/.jsona$/ , "") ?? "noschema";
          schemaIndicator.tooltip = `JSONA Schema: ${url}`;
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
  schemaIndicator.text = "noschema";
  schemaIndicator.tooltip = "Select JSONA Schema";
  schemaIndicator.command = "jsona.selectSchema";
}
