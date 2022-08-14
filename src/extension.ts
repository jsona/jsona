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
  associateSchemas(c);
  vscode.commands.executeCommand("setContext", "jsona.extensionActive", true);
  context.subscriptions.push(
    getOutput(),
    schemaIndicator,
    vscode.extensions.onDidChange(() => {
      associateSchemas(c);
    }),
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

function associateSchemas(c: BaseLanguageClient) {
  let associations = getSchemaAssociations();
  if (associations.length > 0) {
    c.sendNotification("jsona/associateSchemas", { associations });
  }
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
  schemaIndicator.text = "No JSONA Schema";
  schemaIndicator.tooltip = "Select JSONA Schema";
  schemaIndicator.command = "jsona.selectSchema";
}

function getSchemaAssociations() {
  let associations: Lsp.AssociateSchema[] = [];
  for (const ext of vscode.extensions.all) {
    const jsonaValidation = ext.packageJSON?.contributes?.jsonaValidation;

    if (!Array.isArray(jsonaValidation)) {
      continue;
    }

    for (const rule of jsonaValidation) {
      if (typeof rule !== "object") {
        continue;
      }

      const url = rule.url;

      if (typeof url !== "string") {
        continue;
      }

      let fileMatch = rule.fileMatch;
      let regexMatch = rule.regexMatch;

      if (!Array.isArray(fileMatch)) {
        fileMatch = [fileMatch];
      }

      for (let m of fileMatch) {
        if (typeof m !== "string") {
          continue;
        }

        if (!m.startsWith("/")) {
          m = `/${m}`;
        }
        associations.push({
          schemaUri: url,
          rule: {
            glob: `**${m}`,
          },
          meta: {
            source: "extension",
            extensionId: ext.id,
          },
        });
      }

      if (!Array.isArray(regexMatch)) {
        regexMatch = [regexMatch];
      }

      for (const m of regexMatch) {
        if (typeof m !== "string") {
          continue;
        }

        associations.push({
          schemaUri: url,
          rule: {
            regex: m,
          },
          meta: {
            source: "extension",
            extensionId: ext.id,
          },
        });
      }
    }
  }
  return associations;
}