use std::collections::HashSet;

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

    let (mut keys, node) = match Query::node_at(&doc.dom, query.node_at_offset, false) {
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

    tracing::debug!(?query, "debug completion keys={}", keys);

    let comp_items = match &query.scope {
        ScopeKind::AnnotationKey => {
            let props = node
                .annotations()
                .map(|v| v.members_keys())
                .unwrap_or_default();
            complete_properties(doc, &query, &schemas, &props)
        }
        ScopeKind::PropertyKey | ScopeKind::Object => {
            let props = node
                .as_object()
                .map(|v| v.properties_keys())
                .unwrap_or_default();
            complete_properties(doc, &query, &schemas, &props)
        }
        ScopeKind::Array => complete_array(&query, &schemas),
        ScopeKind::Value => complete_value(&query, &schemas),
        _ => return Ok(None),
    };
    Ok(Some(CompletionResponse::Array(comp_items)))
}

fn complete_properties(
    doc: &DocumentState,
    query: &Query,
    schemas: &[Schema],
    exist_props: &[String],
) -> Vec<CompletionItem> {
    let mut comp_items = vec![];
    let current_key = query.key.as_ref().map(|v| v.text()).unwrap_or_default();
    let is_annotation = query.scope == ScopeKind::AnnotationKey;
    for schema in schemas.iter() {
        if !schema.is_object() {
            continue;
        }
        tracing::debug!(
            "complete property key={} schema={}",
            current_key,
            serde_json::to_string(schema).unwrap()
        );
        match schema.properties.as_ref() {
            None => continue,
            Some(properties) => {
                for (prop_key, prop_value) in properties {
                    if exist_props.contains(prop_key) {
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
    comp_items
}

fn complete_array(query: &Query, schemas: &[Schema]) -> Vec<CompletionItem> {
    let mut comp_items = vec![];
    let index = query.index_at().unwrap_or_default();
    let mut types: HashSet<String> = HashSet::default();
    let mut new_schemas = vec![];
    let seperator = if query.add_seperator { "," } else { "" };
    for schema in schemas.iter() {
        if let Some(items) = schema.items.as_ref() {
            let items = items.to_vec();
            match items.len() {
                1 => {
                    new_schemas.push(items[0].clone());
                }
                len if len > index => {
                    new_schemas.push(items[index].clone());
                }
                _ => continue,
            }
        }
    }
    complete_value_impl(&mut comp_items, &mut types, seperator, &new_schemas);
    if comp_items.is_empty() && query.value.is_none() {
        complete_value_type(&mut comp_items, &types, seperator);
    }
    comp_items
}

fn complete_value(query: &Query, schemas: &[Schema]) -> Vec<CompletionItem> {
    let mut comp_items = vec![];
    let mut types: HashSet<String> = HashSet::default();
    let seperator = if query.add_seperator { "," } else { "" };
    complete_value_impl(&mut comp_items, &mut types, seperator, schemas);
    if comp_items.is_empty() && query.value.is_none() {
        complete_value_type(&mut comp_items, &types, seperator);
    }
    comp_items
}

fn complete_value_impl(
    comp_items: &mut Vec<CompletionItem>,
    types: &mut HashSet<String>,
    seperator: &str,
    schemas: &[Schema],
) {
    for schema in schemas.iter() {
        tracing::debug!(
            "complete value schema={}",
            serde_json::to_string(schema).unwrap()
        );
        if let Some(value) = schema.const_value.as_ref() {
            comp_items.push(completion_item_from_value(schema, value, seperator));
        }
        if let Some(enum_value) = schema.enum_value.as_ref() {
            for value in enum_value {
                comp_items.push(completion_item_from_value(schema, value, seperator));
            }
        }
        if let Some(value) = schema.default.as_ref() {
            comp_items.push(completion_item_from_value(schema, value, seperator));
        }
        if let Some(examples) = schema.examples.as_ref() {
            for value in examples {
                comp_items.push(completion_item_from_value(schema, value, seperator));
            }
        }
        if let Some(schema_type) = schema.schema_type.as_deref() {
            types.insert(schema_type.to_string());
        }
    }
}

fn complete_value_type(
    comp_items: &mut Vec<CompletionItem>,
    types: &HashSet<String>,
    seperator: &str,
) {
    let mut items = vec![];
    for schema_type in types {
        match schema_type.as_str() {
            "boolean" => {
                items.push(("true", "true"));
                items.push(("false", "false"));
            }
            "null" => {
                items.push(("null", "null"));
            }
            "string" => {
                items.push((r#""""#, r#""$1""#));
            }
            "number" => {
                items.push(("0", "${1:0}"));
            }
            "object" => {
                items.push(("{}", "{$1}"));
            }
            "array" => {
                items.push(("[]", "[$1]"));
            }
            _ => {}
        }
    }
    for (label, text) in items {
        comp_items.push(completion_item_from_literal(label, text, seperator));
    }
}

fn completion_item_from_value(schema: &Schema, value: &Value, seperator: &str) -> CompletionItem {
    CompletionItem {
        label: stringify_value(value),
        kind: Some(suggestion_kind(schema)),
        insert_text: Some(format!("{}{}", insert_text_for_value(value), seperator)),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        documentation: make_doc(schema),
        ..Default::default()
    }
}

fn completion_item_from_literal(label: &str, text: &str, seperator: &str) -> CompletionItem {
    CompletionItem {
        label: label.into(),
        kind: Some(CompletionItemKind::VALUE),
        insert_text: Some(format!("{}{}", text, seperator)),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
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
    let value = match insert_text_for_schema(schema) {
        Some(value) => value,
        None => return prop_key,
    };
    if is_annotation {
        if value == "${1:null}" {
            return prop_key;
        }
        format!("{}({})", prop_key, value)
    } else {
        format!("{}: {},", prop_key, value)
    }
}

fn insert_text_for_schema(schema: &Schema) -> Option<String> {
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
        value = match schema.schema_type.as_deref() {
            Some("boolean") => "$1",
            Some("string") => r#""$1""#,
            Some("object") => "{$1}",
            Some("array") => "[$1]",
            Some("number") | Some("integer") => "${1:0}",
            Some("null") => "${1:null}",
            _ => "",
        }
        .to_string()
    }
    if value.is_empty() || num_proposals > 1 {
        value = "$1".to_string();
    }
    Some(value)
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
    let text = stringify_value(value);
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

fn suggestion_kind(schema: &Schema) -> CompletionItemKind {
    match schema.schema_type.as_deref() {
        Some("object") => CompletionItemKind::MODULE,
        _ => CompletionItemKind::VALUE,
    }
}

fn stringify_value(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}
