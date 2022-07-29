use std::collections::HashSet;

use jsona::{
    dom::{visit_annotations, DomNode, Key, Keys, Node, VisitControl, Visitor},
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
    Command, CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
    CompletionTextEdit, Documentation, InsertTextFormat, MarkupContent, TextEdit,
};
use serde_json::{json, Value};

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

    let (mut keys, node) = match Query::node_at(&doc.dom, query.node_at_offset) {
        Some(v) => v,
        None => return Ok(None),
    };

    if query.scope == ScopeKind::AnnotationKey {
        keys = Keys::single(Key::annotation(query.key.as_ref().unwrap().text()))
    }

    let schemas = ws.schemas_at_path(&document_uri, &keys).await;
    tracing::info!(
        ?query,
        "debug completion keys={} schemas={}",
        keys,
        schemas.is_some()
    );

    let result = match &query.scope {
        ScopeKind::AnnotationKey => {
            let props = node.annotations().map(|v| v.map_keys()).unwrap_or_default();
            match schemas.as_ref() {
                Some(schemas) => complete_properties(doc, &query, &props, schemas),
                None => complete_annotations_schemaless(doc, &query, &props),
            }
        }
        ScopeKind::PropertyKey | ScopeKind::Object => {
            let props = node
                .as_object()
                .map(|v| v.properties_keys())
                .unwrap_or_default();
            match schemas.as_ref() {
                Some(schemas) => complete_properties(doc, &query, &props, schemas),
                None => complete_properties_schemaless(doc, &query, &props, &keys),
            }
        }
        ScopeKind::Array => match schemas.as_ref() {
            Some(schemas) => complete_array(doc, &query, schemas),
            None => complete_array_schemaless(doc, &query, &keys),
        },
        ScopeKind::Value => match schemas.as_ref() {
            Some(schemas) => complete_value(doc, &query, schemas),
            None => complete_value_schemaless(doc, &query, &keys),
        },
        _ => return Ok(None),
    };

    Ok(result)
}

fn complete_properties(
    doc: &DocumentState,
    query: &Query,
    exist_props: &[String],
    schemas: &[Schema],
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    let is_annotation = query.scope == ScopeKind::AnnotationKey;
    for schema in schemas.iter() {
        if !schema.is_object() {
            continue;
        }
        match schema.properties.as_ref() {
            None => continue,
            Some(properties) => {
                for (prop_key, prop_value) in properties {
                    if exist_props.contains(prop_key) {
                        continue;
                    }
                    comp_items.push(completeion_item_from_prop(
                        doc,
                        query,
                        prop_key,
                        prop_value,
                        is_annotation,
                    ));
                }
            }
        }
    }
    Some(CompletionResponse::Array(comp_items))
}

fn complete_array(
    doc: &DocumentState,
    query: &Query,
    schemas: &[Schema],
) -> Option<CompletionResponse> {
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
    completion_item_set_text_edit(doc, query, &mut comp_items);
    Some(CompletionResponse::Array(comp_items))
}

fn complete_value(
    doc: &DocumentState,
    query: &Query,
    schemas: &[Schema],
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    let mut types: HashSet<String> = HashSet::default();
    let seperator = if query.add_seperator { "," } else { "" };
    complete_value_impl(&mut comp_items, &mut types, seperator, schemas);
    if comp_items.is_empty() && query.value.is_none() {
        complete_value_type(&mut comp_items, &types, seperator);
    }
    completion_item_set_text_edit(doc, query, &mut comp_items);
    Some(CompletionResponse::Array(comp_items))
}

fn complete_annotations_schemaless(
    doc: &DocumentState,
    query: &Query,
    exist_props: &[String],
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    let mut exist_anno_keys: HashSet<String> = HashSet::default();
    exist_anno_keys.insert("@".to_string());
    for (anno_keys, anno_value) in visit_annotations(&doc.dom) {
        if let Some(anno_key) = anno_keys.last_annotation_key() {
            let anno_key = anno_key.value().to_string();
            if exist_anno_keys.contains(&anno_key) {
                continue;
            }
            exist_anno_keys.insert(anno_key.clone());
            if exist_props.contains(&anno_key) {
                continue;
            }
            let anno_schema = schema_from_node(&anno_value);
            comp_items.push(completeion_item_from_prop(
                doc,
                query,
                &anno_key,
                &anno_schema,
                true,
            ))
        }
    }
    if comp_items.is_empty() {
        return None;
    }
    Some(CompletionResponse::Array(comp_items))
}

fn complete_properties_schemaless(
    doc: &DocumentState,
    query: &Query,
    exist_props: &[String],
    keys: &Keys,
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    let parent_key = match keys.last_property_key() {
        None => return None,
        Some(v) => v,
    };
    let mut exist_keys: HashSet<String> = HashSet::default();
    let visitor = Visitor::new(&doc.dom, parent_key, |keys, node, parent_key| {
        match keys.last() {
            Some(key) => match key.as_property_key() {
                Some(key) => {
                    if key.value() == parent_key.value() && node.is_object() {
                        VisitControl::AddIter
                    } else {
                        VisitControl::NotAddIter
                    }
                }
                _ => VisitControl::NotAddNotIter,
            },
            _ => VisitControl::NotAddIter,
        }
    });
    for (_, value) in visitor.into_iter() {
        let obj = match value.as_object() {
            Some(obj) => obj,
            None => continue,
        };
        for (key, value) in obj.value().read().kv_iter() {
            let key = key.value().to_string();
            if exist_keys.contains(&key) {
                continue;
            }
            exist_keys.insert(key.clone());
            if exist_props.contains(&key) {
                continue;
            }

            tracing::info!(
                "completeion_item_from_prop key={} value={}",
                key.to_string(),
                value.to_string()
            );
            let value_schema = schema_from_node(value);
            comp_items.push(completeion_item_from_prop(
                doc,
                query,
                &key,
                &value_schema,
                false,
            ))
        }
    }
    if comp_items.is_empty() {
        return None;
    }
    Some(CompletionResponse::Array(comp_items))
}

fn complete_array_schemaless(
    doc: &DocumentState,
    query: &Query,
    keys: &Keys,
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    let parent_key = match keys.last_property_key() {
        None => return None,
        Some(v) => v,
    };
    let seperator = if query.add_seperator { "," } else { "" };
    let mut exist_labels: HashSet<String> = HashSet::default();
    let visitor = Visitor::new(&doc.dom, parent_key, |keys, node, parent_key| {
        match keys.last() {
            Some(key) => match key.as_property_key() {
                Some(key) => {
                    if key.value() == parent_key.value() && node.is_array() {
                        VisitControl::AddIter
                    } else {
                        VisitControl::NotAddIter
                    }
                }
                _ => VisitControl::NotAddNotIter,
            },
            _ => VisitControl::NotAddIter,
        }
    });
    for (_, value) in visitor.into_iter() {
        let arr = match value.as_array() {
            Some(v) => v,
            None => continue,
        };
        for value in arr.value().read().iter() {
            if let Some(comp_item) = completion_item_from_node(value, seperator) {
                if exist_labels.contains(&comp_item.label) {
                    continue;
                }
                exist_labels.insert(comp_item.label.clone());
                comp_items.push(comp_item);
            }
        }
    }
    completion_item_set_text_edit(doc, query, &mut comp_items);
    Some(CompletionResponse::Array(comp_items))
}

fn complete_value_schemaless(
    doc: &DocumentState,
    query: &Query,
    keys: &Keys,
) -> Option<CompletionResponse> {
    let mut comp_items = vec![];
    let parent_key = match keys.last_property_key() {
        None => return None,
        Some(v) => v,
    };
    let seperator = if query.add_seperator { "," } else { "" };
    let mut exist_labels: HashSet<String> = HashSet::default();
    let visitor = Visitor::new(&doc.dom, parent_key, |keys, _, parent_key| {
        match keys.last() {
            Some(key) => match key.as_property_key() {
                Some(key) => {
                    if key.value() == parent_key.value() {
                        VisitControl::AddIter
                    } else {
                        VisitControl::NotAddIter
                    }
                }
                _ => VisitControl::NotAddNotIter,
            },
            _ => VisitControl::NotAddIter,
        }
    });
    for (_, value) in visitor.into_iter() {
        if let Some(comp_item) = completion_item_from_node(&value, seperator) {
            if exist_labels.contains(&comp_item.label) {
                continue;
            }
            exist_labels.insert(comp_item.label.clone());
            comp_items.push(comp_item);
        }
    }
    completion_item_set_text_edit(doc, query, &mut comp_items);
    Some(CompletionResponse::Array(comp_items))
}

fn completion_item_from_node(node: &Node, seperator: &str) -> Option<CompletionItem> {
    if !node.is_valid_node() {
        return None;
    }
    let (label, value) = match node {
        Node::Null(_) => ("null".to_string(), "null".to_string()),
        Node::Bool(v) => (v.value().to_string(), v.value().to_string()),
        Node::Number(v) => (v.value().to_string(), v.value().to_string()),
        Node::String(v) => {
            let value = match v.syntax() {
                Some(s) => s.to_string(),
                None => r#""""#.into(),
            };
            (value.clone(), value)
        }
        Node::Array(_) => ("[]".to_string(), insert_text_for_value(&json!({}))),
        Node::Object(_) => ("{}".to_string(), insert_text_for_value(&json!([]))),
    };
    let schema = schema_from_node(node);
    Some(CompletionItem {
        label,
        kind: Some(suggestion_kind(&schema.schema_type)),
        insert_text: Some(format!("{}{}", value, seperator)),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    })
}

fn complete_value_impl(
    comp_items: &mut Vec<CompletionItem>,
    types: &mut HashSet<String>,
    seperator: &str,
    schemas: &[Schema],
) {
    for schema in schemas.iter() {
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

fn completeion_item_from_prop(
    doc: &DocumentState,
    query: &Query,
    prop_key: &str,
    prop_value: &Schema,
    is_annotation: bool,
) -> CompletionItem {
    let (insert_text, suggest) =
        insert_text_for_property(query, prop_key, prop_value, is_annotation);

    let text_edit = query.key.as_ref().map(|r| {
        CompletionTextEdit::Edit(TextEdit {
            range: doc.mapper.range(r.text_range()).unwrap().into_lsp(),
            new_text: insert_text.to_string(),
        })
    });

    let command = if suggest {
        Some(Command::new(
            "Suggest".into(),
            "editor.action.triggerSuggest".into(),
            None,
        ))
    } else {
        None
    };
    CompletionItem {
        label: sanitize_label(prop_key.to_string()),
        kind: Some(CompletionItemKind::PROPERTY),
        insert_text: Some(insert_text),
        text_edit,
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        command,
        documentation: make_doc(prop_value),
        ..Default::default()
    }
}

fn completion_item_from_value(schema: &Schema, value: &Value, seperator: &str) -> CompletionItem {
    CompletionItem {
        label: sanitize_label(stringify_value(value)),
        kind: Some(suggestion_kind(&schema.schema_type)),
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

fn completion_item_set_text_edit(
    doc: &DocumentState,
    query: &Query,
    items: &mut Vec<CompletionItem>,
) {
    for item in items {
        if let Some(insert_text) = item.insert_text.as_ref() {
            item.text_edit = query.value.as_ref().map(|r| {
                CompletionTextEdit::Edit(TextEdit {
                    range: doc.mapper.range(r.text_range()).unwrap().into_lsp(),
                    new_text: insert_text.to_string(),
                })
            });
        }
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
) -> (String, bool) {
    let prop_key = if is_annotation {
        prop_key.to_string()
    } else {
        quote(prop_key, false)
    };
    if !query.add_value {
        return (prop_key, false);
    }
    let value = match insert_text_for_schema(schema) {
        Some(value) => value,
        None => return (prop_key, false),
    };
    let suggest = value == "$1";
    if is_annotation {
        if value == "${1:null}" {
            return (prop_key, false);
        }
        (format!("{}({})", prop_key, value), suggest)
    } else {
        (format!("{}: {},", prop_key, value), suggest)
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
    text
}

fn schema_from_node(node: &Node) -> Schema {
    let mut schema = Schema::default();
    let schema_type = match node {
        Node::Null(v) => {
            if v.is_valid_node() {
                "null"
            } else {
                "any"
            }
        }
        Node::Bool(_) => "boolean",
        Node::Number(_) => "number",
        Node::String(_) => "string",
        Node::Array(_) => "array",
        Node::Object(_) => "object",
    };
    schema.schema_type = Some(schema_type.to_string());
    schema
}

fn suggestion_kind(schema_type: &Option<String>) -> CompletionItemKind {
    match schema_type.as_deref() {
        Some("object") => CompletionItemKind::MODULE,
        _ => CompletionItemKind::VALUE,
    }
}

fn sanitize_label(mut value: String) -> String {
    if value.contains('\n') {
        value = value.replace('\n', "↵");
    }
    if value.len() > 60 {
        value = format!("{}...", &value[0..57])
    }
    value
}

fn stringify_value(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}
