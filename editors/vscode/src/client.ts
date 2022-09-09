import * as vscode from "vscode";
import * as node from "vscode-languageclient/node";
import * as browser from "vscode-languageclient/browser";
import which from "which";
import { getOutput, ID, NAME, SCHEMA_CACHE_KEY } from "./util";
import { BaseLanguageClient } from "vscode-languageclient";


export async function createClient(
  context: vscode.ExtensionContext
): Promise<BaseLanguageClient> {
  if (import.meta.env.BROWSER) {
    return await createBrowserClient(context);
  } else {
    return await createNodeClient(context);
  }
}

async function createBrowserClient(context: vscode.ExtensionContext) {
  const serverMain = vscode.Uri.joinPath(
    context.extensionUri,
    "dist/server-worker.js"
  );
  const worker = new Worker(serverMain.toString(true));
  return new browser.LanguageClient(
    ID,
    NAME,
    {
      outputChannel: getOutput(),
      documentSelector: [{ language: "jsona" }],
    },
    worker
  );
}

async function createNodeClient(context: vscode.ExtensionContext) {
  const out = getOutput();

  const bundled = !!vscode.workspace
    .getConfiguration()
    .get("jsona.executable.bundled");

  let serverOpts: node.ServerOptions;
  if (bundled) {
    const jsonaPath = vscode.Uri.joinPath(
      context.extensionUri,
      "dist/server.js"
    ).fsPath;

    const run: node.NodeModule = {
      module: jsonaPath,
      transport: node.TransportKind.ipc,
      options: {
        env:
          vscode.workspace
            .getConfiguration()
            .get("jsona.executable.environment") ?? undefined,
      },
    };

    out.appendLine("Use bundled lsp");
    serverOpts = {
      run,
      debug: run,
    };
  } else {
    const jsonaPath =
      vscode.workspace.getConfiguration().get("jsona.executable.path") ?? which.sync("jsona", { nothrow: true });

    if (typeof jsonaPath !== "string") {
      throw new Error("failed to locate JSONA LSP");
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

    const run: node.Executable = {
      command: jsonaPath,
      args: ["lsp", "stdio", ...args],
      options: {
        env:
          vscode.workspace
            .getConfiguration()
            .get("jsona.executable.environment") ?? undefined,
      },
    };

    out.appendLine(`Use native lsp at ${jsonaPath}`);
    serverOpts = {
      run,
      debug: run,
    };
  }

  let cachePath = await syncSchemaCache(context);


  return new node.LanguageClient(
    ID,
    NAME,
    serverOpts,
    {
      outputChannel: getOutput(),
      documentSelector: [{ language: "jsona" }],
      initializationOptions: {
        cachePath: cachePath.toString(),
      },
    }
  );
}

export async function syncSchemaCache(context: vscode.ExtensionContext) {
  const cachePath = vscode.Uri.joinPath(context.globalStorageUri, "schema-cache-v1");
  try {
    if (vscode.workspace.getConfiguration().get(SCHEMA_CACHE_KEY)) {
      await vscode.workspace.fs.createDirectory(cachePath);
    } else {
      await vscode.workspace.fs.delete(cachePath, { recursive: true });
    }
  } catch (err) {}
  return cachePath;
}