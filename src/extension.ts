import * as vscode from "vscode";
import { createClient } from "./client";
import { registerCommands } from "./commands";
import { syncExtensionSchemas } from "./jsonaValidation";
import { showMessage, getOutput } from "./util";

export async function activate(context: vscode.ExtensionContext) {
  const schemaIndicator = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    0
  );
  resetSchemaIndicator(schemaIndicator);

  const c = await createClient(context);
  await c.start()

  if (vscode.window.activeTextEditor?.document.languageId === "jsona") {
    schemaIndicator.show();
  }
  
  registerCommands(context, c);
  syncExtensionSchemas(context, c);
  vscode.commands.executeCommand("setContext", "jsona.extensionActive", true);
  context.subscriptions.push(
    getOutput(),
    schemaIndicator,
    vscode.window.onDidChangeActiveTextEditor(editor => {
      if (editor?.document.languageId === "jsona") {
        let docUri = editor?.document.uri;
        if (docUri) {
          c.sendRequest("jsona/associatedSchema", {
            documentUri: docUri.toString(),
          });
        }
        schemaIndicator.show();
      } else {
        schemaIndicator.hide();
      }
    }),
    c.onNotification(
      "jsona/didChangeSchemaAssociation",
      async (params: {
        documentUri: string;
        schemaUri?: string;
        meta?: Record<string, any>;
      }) => {
          const currentDocumentUrl =
            vscode.window.activeTextEditor?.document.uri.toString();

          if (!currentDocumentUrl) {
            return;
          }

          if (params.documentUri === currentDocumentUrl) {
            schemaIndicator.text =
              params.meta?.name ?? params.schemaUri?.split("/").slice(-1)[0] ?? "no schema";
            schemaIndicator.tooltip = `JSONA Schema: ${params.schemaUri}`;
          }
      }
    ),
    c.onNotification("jsona/messageWithOutput", async params =>
      showMessage(params, c)
    ),
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

function resetSchemaIndicator(schemaIndicator: vscode.StatusBarItem) {
  schemaIndicator.text = "No JSONA Schema";
  schemaIndicator.tooltip = "Select JSONA Schema";
  schemaIndicator.command = "jsona.selectSchema";
}
