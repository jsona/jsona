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
  let schemaIndicatorParams: DidChangeSchemaAssociationParams;
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
      if (editor?.document.languageId === "jsona" && editor?.document.uri) {
        const doc_url = editor?.document.uri.toString();
        updateSchemaIndicator(schemaIndicator, schemaIndicatorParams, doc_url)
        schemaIndicatorParams = null;
        schemaIndicator.show();
      } else {
        schemaIndicator.hide();
      }
    }),
    c.onNotification(
      "jsona/didChangeSchemaAssociation",
      async (params: DidChangeSchemaAssociationParams) => {
        schemaIndicatorParams = null;
          const doc_url = vscode.window.activeTextEditor?.document.uri.toString();
          if (!doc_url) {
            schemaIndicatorParams = params
            return;
          }
          updateSchemaIndicator(schemaIndicator, params, doc_url);
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

function updateSchemaIndicator(schemaIndicator: vscode.StatusBarItem, params: DidChangeSchemaAssociationParams, doc_url: string) {
  resetSchemaIndicator(schemaIndicator);
  if (params?.documentUri === doc_url) {
    schemaIndicator.text =
      params.meta?.name ?? params.schemaUri?.split("/").slice(-1)[0] ?? "No JSONA Schema";
    schemaIndicator.tooltip = `JSONA Schema: ${params.schemaUri}`;
  }
}

interface DidChangeSchemaAssociationParams {
  documentUri: string;
  schemaUri?: string;
  meta?: Record<string, any>;
}