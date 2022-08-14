import * as vscode from "vscode";
import { BaseLanguageClient } from "vscode-languageclient";

export function register(ctx: vscode.ExtensionContext, c: BaseLanguageClient) {
  ctx.subscriptions.push(
    vscode.commands.registerTextEditorCommand(
      "jsona.selectSchema",
      async editor => {
        if (!editor) {
          return;
        }
        let documentUri = editor.document.uri.toString();

        const schemasResp: { schemas: { url: string; meta?: any }[] } =
          await c.sendRequest("jsona/listSchemas", { documentUri });

        interface SchemaItem extends vscode.QuickPickItem {
          url: string;
          meta?: Record<string, any>;
        }

        const selectedSchema: { schema?: { url: string } } =
          await c.sendRequest("jsona/associatedSchema", { documentUri });

        const selection = await vscode.window.showQuickPick<SchemaItem>(
          schemasResp.schemas.map(s => ({
            label: s.meta?.name ?? s.url,
            description: schemaDescription(s.meta),
            detail: schemaDetails(s.url, s.meta),
            picked: selectedSchema.schema?.url === s.url,
            url: s.url,
            meta: s.meta,
          }))
        );

        if (!selection) {
          return;
        }
        writeSchemaAssociations(selection.url, documentUri);
      }
    )
  );
}

function writeSchemaAssociations(schemaUrl: string, fileUrl: string) {
    const associations: Record<string, string[]> = vscode.workspace.getConfiguration("jsona").get("schema.associations");
    const newAssociations = Object.assign({}, associations);
    deleteExistingAssociationFile(newAssociations, fileUrl);
    let schemaAssociations = newAssociations[schemaUrl];
    if (!schemaAssociations) {
      newAssociations[schemaUrl] = [fileUrl];
    } else {
      newAssociations[schemaUrl] = [...schemaAssociations, fileUrl];
    }
   vscode.workspace.getConfiguration("jsona").update("schema.associations", newAssociations);
}

function deleteExistingAssociationFile(associations: Record<string, string[]>, fileUri: string) {
  for (const key in associations) {
    if (Object.prototype.hasOwnProperty.call(associations, key)) {
      const element = associations[key];
      if (Array.isArray(element)) {
        const filePatterns = element.filter((val) => val !== fileUri);
        associations[key] = filePatterns;
      } else {
        delete associations[key];
      }
    }
  }
}

function schemaDescription(meta: any | undefined): string | undefined {
  if (typeof meta?.description === "string") {
    return meta.description;
  } else {
    return undefined;
  }
}

function schemaDetails(url: string, _meta: any): string {
  let s = `${url}`;
  return s;
}
