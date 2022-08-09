import * as vscode from "vscode";
import * as client from "vscode-languageclient/node";
import which from "which";
import { registerCommands } from "./commands";
import { syncExtensionSchemas } from "./jsonaValidation";

let output: vscode.OutputChannel;

export function getOutput(): vscode.OutputChannel {
  if (!output) {
    output = vscode.window.createOutputChannel("JSONA");
  }

  return output;
}

export async function activate(context: vscode.ExtensionContext) {
  const jsonaPath =
    vscode.workspace.getConfiguration().get("jsona.executable.path") ?? await getServerPath(context) ??  which.sync("jsona", { nothrow: true });
  
  // getOutput().appendLine(`Use jsona at ${jsonaPath}`);

  if (typeof jsonaPath !== "string") {
    await vscode.window.showErrorMessage(
        "Unfortunately we don't ship binaries for your platform yet. " +
            "You need to manually clone the jsona repository and " +
            "run `cargo build --release -p jsona-cli` to build the language server from sources. " +
            "If you feel that your platform should be supported, please create an issue " +
            "about that [here](https://github.com/jsona/jsona/issues) and we " +
            "will consider it."
    );
    return;
  }

  let extraArgs = vscode.workspace
    .getConfiguration()
    .get("jsona.executable.extraArgs");

  if (!Array.isArray(extraArgs)) {
    extraArgs = [];
  }

  const args: string[] = (extraArgs as any[]).filter(
    a => typeof a === "string"
  );

  const run: client.Executable = {
    command: jsonaPath,
    args: ["lsp", "stdio", ...args],
    options: {
      env:
        vscode.workspace
          .getConfiguration()
          .get("jsona.executable.environment") ?? undefined,
    },
  };

  let serverOpts: client.ServerOptions = {
    run,
    debug: run,
  };

  let clientOpts: client.LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "jsona" }],
    initializationOptions: {
      configuration: vscode.workspace.getConfiguration().get("jsona"),
      cachePath: context.globalStorageUri.fsPath,
    },
    synchronize: {
      configurationSection: "jsona",
      fileEvents: [vscode.workspace.createFileSystemWatcher("**/*.jsona")],
    },
  };

  let c = new client.LanguageClient(
    "JSONA",
    "JSONA Language Server",
    serverOpts,
    clientOpts
  );

  await c.start()
  
  registerCommands(context, c);
  syncExtensionSchemas(context, c);
  vscode.commands.executeCommand("setContext", "jsona.extensionActive", true);
  context.subscriptions.push(
    getOutput(),
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

async function getServerPath(context: vscode.ExtensionContext) {
    const ext = process.platform === "win32" ? ".exe" : "";
    const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `jsona${ext}`);
    const bundledExists = await vscode.workspace.fs.stat(bundled).then(
        () => true,
        () => false
    );
    if (bundledExists) {
        return bundled.fsPath
    }
}