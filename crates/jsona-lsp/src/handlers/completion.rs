use jsona::{
    dom::{DomNode, Key, Keys},
    util::quote,
};
use jsona_schema::Schema;
use jsona_util::environment::Environment;
use lsp_async_stub::{
    rpc::Error,
    util::{LspExt, Position},
    Context, Params,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, CompletionTextEdit,
    Documentation, InsertTextFormat, MarkupContent, TextEdit,
};
use serde_json::Value;

use crate::World;
use crate::{
    query::{Query, ScopeKind},
    world::DocumentState,
};

#[tracing::instrument(skip_all)]
pub async fn completion<E: Environment>(
    context: Context<World<E>>,
    params: Params<CompletionParams>,
) -> Result<Option<CompletionResponse>, Error> {
    let p = params.required()?;

    let document_uri = p.text_document_position.text_document.uri;

    let workspaces = context.workspaces.read().await;
    let ws = workspaces.by_document(&document_uri);

    if !ws.config.schema.enabled {
        return Ok(None);
    }

    let doc = ws.document(&document_uri)?;

    let schema_association = match ws.schemas.associations().association_for(&document_uri) {
        Some(ass) => ass,
        None => return Ok(None),
    };

    let position = p.text_document_position.position;
    let offset = match doc.mapper.offset(Position::from_lsp(position)) {
        Some(ofs) => ofs,
        None => {
            tracing::error!(?position, "document position not found");
            return Ok(None);
        }
    };

    let query = Query::at(&doc.dom, offset);
    if query.scope == ScopeKind::Unknown {
        return Ok(None);
    }

    let (mut keys, node) = match query.node_at(&doc.dom) {
        Some(v) => v,
        None => return Ok(None),
    };

    if query.scope == ScopeKind::AnnotationKey {
        keys = Keys::single(Key::annotation(query.key.as_ref().unwrap().text()))
    }

    let schemas = match ws
        .schemas
        .pointer_schemas(&schema_association.url, &keys)
        .await
    {
        Ok(v) => v,
        Err(error) => {
            tracing::error!(?error, "failed to query schemas");
            return Ok(None);
        }
    };

    tracing::info!(?query, "debug completion keys={}", keys);

    match &query.scope {
        ScopeKind::AnnotationKey => {
            let props = node
                .annotations()
                .map(|v| v.members_keys())
                .unwrap_or_default();
            return Ok(complete_properties(doc, &schemas, &props, &query));
        }
        ScopeKind::PropertyKey | ScopeKind::Object => {
            let props = node
                .as_object()
                .map(|v| v.properties_keys())
                .unwrap_or_default();
            return Ok(complete_properties(doc, &schemas, &props, &query));
        }
        _ => {}
    }

    Ok(None)
}

fn complete_properties(
    doc: &DocumentState,
    schemas: &[Schema],
    exist_props: &[String],
    query: &Query,
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    let current_key = query.key.as_ref().map(|v| v.text()).unwrap_or_default();
    let is_annotation = query.scope == ScopeKind::AnnotationKey;
    for schema in schemas.iter() {
        tracing::info!(
            "complete properties schema={}",
            serde_json::to_string(schema).unwrap()
        );
        if !schema.is_object() {
            continue;
        }
        match schema.properties.as_ref() {
            None => continue,
            Some(properties) => {
                for (prop_key, prop_value) in properties {
                    if current_key != prop_key && exist_props.contains(prop_key) {
                        continue;
                    }

                    let insert_text =
                        insert_text_for_property(query, prop_key, prop_value, is_annotation);

                    let text_edit = query.key.as_ref().map(|r| {
                        CompletionTextEdit::Edit(TextEdit {
                            range: doc.mapper.range(r.text_range()).unwrap().into_lsp(),
                            new_text: insert_text.to_string(),
                        })
                    });

                    comp_items.push(CompletionItem {
                        label: prop_key.to_string(),
                        kind: Some(CompletionItemKind::PROPERTY),
                        insert_text: Some(insert_text),
                        text_edit,
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        documentation: make_doc(prop_value),
                        ..Default::default()
                    });
                }
            }
        }
    }
    if comp_items.is_empty() {
        None
    } else {
        Some(CompletionResponse::Array(comp_items))
    }
}

fn complete_value(
    doc: &DocumentState,
    schemas: &[Schema],
    query: &Query,
) -> Option<CompletionResponse> {
    todo!()
}

fn make_doc(schema: &Schema) -> Option<Documentation> {
    if let Some(docs) = schema.description.as_ref() {
        return Some(Documentation::MarkupContent(MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: docs.into(),
        }));
    }
    None
}

fn insert_text_for_property(
    query: &Query,
    prop_key: &str,
    schema: &Schema,
    is_annotation: bool,
) -> String {
    let prop_key = if is_annotation {
        prop_key.to_string()
    } else {
        quote(prop_key, false)
    };
    if !query.add_value {
        return prop_key;
    }
    let mut value = String::new();
    let mut num_proposals = 0;
    if let Some(enum_value) = schema.enum_value.as_ref() {
        if value.is_empty() && enum_value.len() == 1 {
            value = insert_text_for_guess_value(&enum_value[0]);
        }
        num_proposals += enum_value.len();
    }
    if let Some(const_value) = schema.const_value.as_ref() {
        if value.is_empty() {
            value = insert_text_for_guess_value(const_value);
        }
        num_proposals += 1
    }
    if let Some(default) = schema.default.as_ref() {
        if value.is_empty() {
            value = insert_text_for_guess_value(default);
        }
        num_proposals += 1
    }
    if let Some(examples) = schema.examples.as_ref() {
        if value.is_empty() && examples.len() == 1 {
            value = insert_text_for_guess_value(&examples[0]);
        }
        num_proposals += examples.len();
    }
    if num_proposals == 0 {
        match schema.schema_type.as_deref() {
            Some("boolean") => value = "$1".into(),
            Some("string") => value = r#""$1""#.into(),
            Some("object") => value = "{$1}".into(),
            Some("array") => value = "[$1]".into(),
            Some("number") | Some("integer") => value = "{$1:0}".into(),
            Some("null") => {
                if is_annotation {
                    return prop_key;
                }
                value = "{$1:null}".into();
            }
            _ => return prop_key,
        }
    }
    if value.is_empty() || num_proposals > 1 {
        value = "$1".to_string();
    }
    if is_annotation {
        format!("{}({})", prop_key, value)
    } else {
        format!("{}: {},", prop_key, value)
    }
}

fn insert_text_for_guess_value(value: &Value) -> String {
    match value {
        Value::Null | Value::Number(_) | Value::Bool(_) | Value::String(_) => {
            format!("${{1:{}}}", value)
        }
        Value::Array(_) | Value::Object(_) => insert_text_for_value(value),
    }
}

fn insert_text_for_value(value: &Value) -> String {
    let text = serde_json::to_string_pretty(value).unwrap();
    if text == "{}" {
        return "{$1}".into();
    } else if text == "[]" {
        return "[$l]".into();
    }
    insert_text_for_plain_text(text)
}

fn insert_text_for_plain_text(text: String) -> String {
    text
}
