use std::collections::HashSet;

use indexmap::{IndexMap, IndexSet};
use jsona::{
    dom::{visit_annotations, DomNode, Key, KeyOrIndex, Keys, Node, VisitControl, Visitor},
    util::quote,
};
use jsona_schema::{Schema, SchemaType};
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

    let workspaces = context.workspaces.read().await;
    let document_uri = p.text_document_position.text_document.uri;
    let ws = workspaces.by_document(&document_uri);
    if !ws.lsp_config.schema.enabled {
        return Ok(None);
    }

    let doc = match ws.document(&document_uri) {
        Ok(d) => d,
        Err(error) => {
            tracing::debug!(%error, "failed to get document from workspace");
            return Ok(None);
        }
    };

    let position = p.text_document_position.position;
    let offset = match doc.mapper.offset(Position::from_lsp(position)) {
        Some(ofs) => ofs,
        None => {
            tracing::error!(?position, "document position not found");
            return Ok(None);
        }
    };

    let query = Query::at(&doc.dom, offset, true);
    if query.scope == ScopeKind::Unknown {
        return Ok(None);
    }

    let (keys, node) = match Query::node_at(&doc.dom, query.node_at_offset) {
        Some(v) => v,
        None => return Ok(None),
    };
    let query_keys = match &query.scope {
        ScopeKind::AnnotationKey => {
            Keys::single(Key::annotation(query.key.as_ref().unwrap().text()))
        }
        ScopeKind::Array => {
            let idx = query.index_at().unwrap_or_default();
            keys.join(KeyOrIndex::Index(idx))
        }
        _ => keys.clone(),
    };

    let schemas = ws.query_schemas(&document_uri, &query_keys).await;
    tracing::debug!(
        ?query,
        "completion keys={} schemas={:?}",
        keys,
        schemas.as_ref().map(|v| v.len())
    );

    let result = match &query.scope {
        ScopeKind::AnnotationKey => {
            let mut props = node.annotations().map(|v| v.map_keys()).unwrap_or_default();
            if let Some(key) = query.key.as_ref() {
                props.push(key.to_string());
            }
            match schemas.as_ref() {
                Some(schemas) => complete_key(doc, &query, &props, schemas),
                None => complete_annotations_schemaless(doc, &query, &props),
            }
        }
        ScopeKind::PropertyKey | ScopeKind::Object => {
            let props: Vec<String> = node
                .as_object()
                .map(|v| {
                    v.value()
                        .read()
                        .kv_iter()
                        .map(|(k, _)| k.value().to_string())
                        .collect()
                })
                .unwrap_or_default();
            match schemas.as_ref() {
                Some(schemas) => complete_key(doc, &query, &props, schemas),
                None => complete_properties_schemaless(doc, &query, &props, &keys),
            }
        }
        ScopeKind::Array => match schemas.as_ref() {
            Some(schemas) => complete_value(doc, &query, schemas),
            None => complete_value_schemaless(doc, &query, &keys, true),
        },
        ScopeKind::Value => match schemas.as_ref() {
            Some(schemas) => complete_value(doc, &query, schemas),
            None => complete_value_schemaless(doc, &query, &keys, false),
        },
        _ => return Ok(None),
    };

    Ok(result)
}

fn complete_key(
    doc: &DocumentState,
    query: &Query,
    exist_props: &[String],
    schemas: &[Schema],
) -> Option<CompletionResponse> {
    let mut comps = CompletionMap::default();
    for schema in schemas.iter() {
        if !schema.maybe_type(&SchemaType::Object) {
            continue;
        }
        match schema.properties.as_ref() {
            None => continue,
            Some(properties) => {
                for (prop_key, prop_value) in properties {
                    if exist_props.contains(prop_key) {
                        continue;
                    }
                    comps.add_key(prop_key, prop_value);
                }
            }
        }
    }
    comps.into_key_completions(doc, query)
}

fn complete_value(
    doc: &DocumentState,
    query: &Query,
    schemas: &[Schema],
) -> Option<CompletionResponse> {
    let mut comps = CompletionMap::default();
    for schema in schemas.iter() {
        comps.add_schema(schema);
    }
    comps.into_schema_completions(doc, query)
}

fn complete_annotations_schemaless(
    doc: &DocumentState,
    query: &Query,
    exist_props: &[String],
) -> Option<CompletionResponse> {
    let mut comps = CompletionMap::default();
    for (anno_keys, anno_value) in visit_annotations(&doc.dom) {
        if let Some(anno_key) = anno_keys.last_annotation_key() {
            let anno_key = anno_key.value().to_string();
            if &anno_key == "@" || exist_props.contains(&anno_key) {
                continue;
            }
            let anno_schema = node_to_schema(&anno_value);
            comps.add_key(&anno_key, &anno_schema)
        }
    }
    comps.into_key_completions(doc, query)
}

fn complete_properties_schemaless(
    doc: &DocumentState,
    query: &Query,
    exist_props: &[String],
    keys: &Keys,
) -> Option<CompletionResponse> {
    let mut comps = CompletionMap::default();
    let parent_key = match keys.last_property_key() {
        None => return None,
        Some(v) => v,
    };
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
            if exist_props.contains(&key) {
                continue;
            }
            let schema = node_to_schema(value);
            comps.add_key(&key, &schema);
        }
    }
    comps.into_key_completions(doc, query)
}

fn complete_value_schemaless(
    doc: &DocumentState,
    query: &Query,
    keys: &Keys,
    is_array: bool,
) -> Option<CompletionResponse> {
    let mut comps = CompletionMap::default();
    let parent_key = match keys.last_property_key() {
        None => return None,
        Some(v) => v,
    };
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
    if is_array {
        for (visitor_keys, value) in visitor.into_iter() {
            if &visitor_keys == keys {
                continue;
            }
            let arr = match value.as_array() {
                Some(v) => v,
                None => continue,
            };
            for value in arr.value().read().iter() {
                comps.add_node(value);
            }
        }
    } else {
        for (visitor_keys, value) in visitor.into_iter() {
            if &visitor_keys == keys {
                continue;
            }
            comps.add_node(&value);
        }
    }
    comps.into_node_completions(doc, query)
}

#[derive(Debug, Default)]
struct CompletionMap {
    keys: IndexMap<String, CompletionKeyData>,
    values: IndexMap<String, CompletionValueData>,
    types_add: IndexSet<SchemaType>,
    types_all: HashSet<SchemaType>,
}

#[derive(Debug)]
struct CompletionKeyData {
    types: HashSet<SchemaType>,
    document: Option<String>,
}

#[derive(Debug)]
struct CompletionValueData {
    plain: bool,
    value: String,
    kind: CompletionItemKind,
}

impl CompletionMap {
    fn add_key(&mut self, key: &str, schema: &Schema) {
        if let Some(key_data) = self.keys.get_mut(key) {
            key_data.types.extend(schema.types());
            if let (None, Some(description)) =
                (key_data.document.as_ref(), schema.description.as_ref())
            {
                key_data.document = Some(description.clone());
            }
        } else {
            self.keys.insert(
                key.to_string(),
                CompletionKeyData {
                    types: schema.types(),
                    document: schema.description.clone(),
                },
            );
        }
    }

    fn add_node(&mut self, node: &Node) {
        match node {
            Node::Null(_) => {
                self.add_null_value();
            }
            Node::Bool(_) => {
                self.add_bool_value();
            }
            Node::Number(v) => {
                let value = v.value().to_string();
                self.add_number_value(value);
            }
            Node::String(v) => {
                let value = match v.syntax() {
                    Some(s) => s.to_string(),
                    None => quote(v.value(), true),
                };
                self.add_string_value(value);
            }
            Node::Array(_) => {
                self.add_array_value();
            }
            Node::Object(_) => {
                self.add_object_value();
            }
        }
    }

    fn add_schema(&mut self, schema: &Schema) {
        let values = collect_schema_values(schema);
        self.types_all.extend(schema.types().into_iter());
        for value in values {
            match value {
                Value::Null => {
                    self.add_null_value();
                }
                Value::Bool(_) => {
                    self.add_bool_value();
                }
                Value::Number(v) => self.add_number_value(v.to_string()),
                Value::String(v) => self.add_string_value(quote(v.as_str(), true)),
                Value::Array(_) => {
                    self.add_array_value();
                }
                Value::Object(_) => {
                    self.add_object_value();
                }
            }
        }
    }

    fn into_key_completions(
        mut self,
        doc: &DocumentState,
        query: &Query,
    ) -> Option<CompletionResponse> {
        if self.keys.is_empty() {
            return None;
        }
        let mut output = vec![];
        let is_annotation = query.scope == ScopeKind::AnnotationKey;
        let (space, comma) = query.space_and_comma();
        for (key, data) in self.keys.drain(..) {
            let CompletionKeyData {
                document: description,
                types,
            } = data;
            let key = if is_annotation {
                key.to_string()
            } else {
                quote(&key, false)
            };
            let mut value = "$1".into();
            let mut command = None;
            let mut is_null = false;
            if query.add_value && types.len() == 1 {
                let schema_type = types.iter().next().unwrap();
                is_null = schema_type == &SchemaType::Null;
                value = insert_text_from_type(schema_type).to_string();
            }
            if value == "$1" {
                command = Some(Command::new(
                    "Suggest".into(),
                    "editor.action.triggerSuggest".into(),
                    None,
                ));
            }
            let label = sanitize_label(key.clone());
            let insert_text = if !query.add_value || (is_annotation && is_null) {
                key
            } else if is_annotation {
                format!("{}({})", key, value)
            } else {
                format!("{}:{}{}{}", key, space, value, comma)
            };
            let text_edit = query.key.as_ref().map(|r| {
                CompletionTextEdit::Edit(TextEdit {
                    range: doc.mapper.range(r.text_range()).unwrap().into_lsp(),
                    new_text: insert_text.to_string(),
                })
            });
            output.push(CompletionItem {
                label,
                kind: Some(CompletionItemKind::PROPERTY),
                insert_text: Some(insert_text),
                text_edit,
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                command,
                documentation: to_document(&description),
                ..Default::default()
            });
        }
        if output.is_empty() {
            return None;
        }
        Some(CompletionResponse::Array(output))
    }

    fn into_node_completions(
        mut self,
        doc: &DocumentState,
        query: &Query,
    ) -> Option<CompletionResponse> {
        if self.values.is_empty() {
            return None;
        }
        let mut output = vec![];
        let (space, comma) = query.space_and_comma();
        for (label, data) in self.values.drain(..) {
            let CompletionValueData { plain, kind, value } = data;
            let format = if plain {
                InsertTextFormat::PLAIN_TEXT
            } else {
                InsertTextFormat::SNIPPET
            };
            let insert_text = format!("{}{}{}", space, value, comma);
            let text_edit = query.value.as_ref().map(|r| {
                CompletionTextEdit::Edit(TextEdit {
                    range: doc.mapper.range(r.text_range()).unwrap().into_lsp(),
                    new_text: insert_text.to_string(),
                })
            });
            output.push(CompletionItem {
                label,
                kind: Some(kind),
                insert_text: Some(insert_text),
                insert_text_format: Some(format),
                text_edit,
                ..Default::default()
            })
        }
        if output.is_empty() {
            return None;
        }
        Some(CompletionResponse::Array(output))
    }

    fn into_schema_completions(
        mut self,
        doc: &DocumentState,
        query: &Query,
    ) -> Option<CompletionResponse> {
        let mut types_miss = vec![];
        for schema_type in self.types_all.iter() {
            if self.types_add.contains(schema_type) {
                continue;
            }
            types_miss.push(schema_type.clone());
        }
        for schema_type in types_miss.into_iter() {
            match schema_type {
                SchemaType::String => {
                    self.add_string_value(r#""""#.into());
                }
                SchemaType::Number => {
                    self.add_number_value("0.0".into());
                }
                SchemaType::Integer => {
                    self.add_number_value("0".into());
                }
                SchemaType::Boolean => {
                    self.add_bool_value();
                }
                SchemaType::Null => {
                    self.add_null_value();
                }
                SchemaType::Object => {
                    self.add_object_value();
                }
                SchemaType::Array => {
                    self.add_array_value();
                }
            }
        }
        self.into_node_completions(doc, query)
    }

    fn add_null_value(&mut self) {
        self.types_add.insert(SchemaType::Null);
        self.values.insert(
            "null".into(),
            CompletionValueData {
                plain: true,
                value: "null".into(),
                kind: CompletionItemKind::VALUE,
            },
        );
    }
    fn add_bool_value(&mut self) {
        self.types_add.insert(SchemaType::Boolean);
        self.values.insert(
            "true".into(),
            CompletionValueData {
                plain: true,
                value: "true".into(),
                kind: CompletionItemKind::VALUE,
            },
        );
        self.values.insert(
            "false".into(),
            CompletionValueData {
                plain: true,
                value: "false".into(),
                kind: CompletionItemKind::VALUE,
            },
        );
    }
    fn add_number_value(&mut self, value: String) {
        self.types_add.insert(SchemaType::Number);
        let label = sanitize_label(value.clone());
        self.values.insert(
            label,
            CompletionValueData {
                plain: true,
                value,
                kind: CompletionItemKind::VALUE,
            },
        );
    }
    fn add_string_value(&mut self, value: String) {
        self.types_add.insert(SchemaType::String);
        let label = sanitize_label(value.clone());
        self.values.insert(
            label,
            CompletionValueData {
                plain: true,
                value,
                kind: CompletionItemKind::VALUE,
            },
        );
    }
    fn add_array_value(&mut self) {
        self.types_add.insert(SchemaType::Array);
        self.values.insert(
            "[]".into(),
            CompletionValueData {
                plain: false,
                value: insert_text_from_type(&SchemaType::Array).to_string(),
                kind: CompletionItemKind::VALUE,
            },
        );
    }
    fn add_object_value(&mut self) {
        self.types_add.insert(SchemaType::Array);
        self.values.insert(
            "{}".into(),
            CompletionValueData {
                plain: false,
                value: insert_text_from_type(&SchemaType::Object).to_string(),
                kind: CompletionItemKind::MODULE,
            },
        );
    }
}

fn insert_text_from_type(schema_type: &SchemaType) -> &'static str {
    match schema_type {
        SchemaType::String => r#""$1""#,
        SchemaType::Number => "$1",
        SchemaType::Integer => "$1",
        SchemaType::Boolean => "$1",
        SchemaType::Null => "${1:null}",
        SchemaType::Object => "{$1}",
        SchemaType::Array => "[$1]",
    }
}

fn node_to_schema(node: &Node) -> Schema {
    Schema {
        schema_type: SchemaType::from_node(node).map(|v| v.into()),
        ..Default::default()
    }
}

fn collect_schema_values(schema: &Schema) -> Vec<&Value> {
    let mut values = vec![];
    if let Some(value) = schema.const_value.as_ref() {
        values.push(value);
    }
    if let Some(value) = schema.default.as_ref() {
        values.push(value);
    }
    if let Some(value) = schema.enum_value.as_ref() {
        values.extend(value);
    }
    if let Some(value) = schema.examples.as_ref() {
        values.extend(value);
    }
    values
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

fn to_document(data: &Option<String>) -> Option<Documentation> {
    if let Some(doc) = data.as_ref() {
        return Some(Documentation::MarkupContent(MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: doc.into(),
        }));
    }
    None
}
