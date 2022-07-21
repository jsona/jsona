import * as vscode from "vscode";
import { BaseLanguageClient } from "vscode-languageclient";

export function syncExtensionSchemas(
  _ctx: vscode.ExtensionContext,
  c: BaseLanguageClient
) {
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
        c.sendNotification("jsona/associateSchema", {
          schemaUri: url,
          priority: 10,
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

        c.sendNotification("jsona/associateSchema", {
          schemaUri: url,
          priority: 10, // above catalogs, but below any manual config
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
}
