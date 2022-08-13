import type { Lsp } from "@jsona/lsp";
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
    vscode.window.onDidChangeActiveTextEditor(async editor => {
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
    }),
    c.onNotification(
      "jsona/didChangeSchemaAssociation",
      async (params: Lsp.Server.NotificationParams<"jsona/didChangeSchemaAssociation">) => {
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
