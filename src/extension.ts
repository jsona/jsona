import * as vscode from "vscode";
import { createClient } from "./client";
import { registerCommands } from "./commands";
import { syncExtensionSchemas } from "./jsonaValidation";
import { showMessage, getOutput } from "./util";

export async function activate(context: vscode.ExtensionContext) {
  const c = await createClient(context);
  await c.start()
  
  registerCommands(context, c);
  syncExtensionSchemas(context, c);
  vscode.commands.executeCommand("setContext", "jsona.extensionActive", true);
  context.subscriptions.push(
    getOutput(),
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
