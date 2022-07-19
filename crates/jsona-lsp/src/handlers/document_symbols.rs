use crate::world::{DocumentState, World};
use jsona::dom::{self, DomNode, Node};
use jsona_util::environment::Environment;
use lsp_async_stub::{
    rpc::Error,
    util::{LspExt, Mapper},
    Context, Params,
};
use lsp_types::{DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, SymbolKind};

#[tracing::instrument(skip_all)]
pub(crate) async fn document_symbols<E: Environment>(
    context: Context<World<E>>,
    params: Params<DocumentSymbolParams>,
) -> Result<Option<DocumentSymbolResponse>, Error> {
    let p = params.required()?;

    let workspaces = context.workspaces.read().await;
    let ws = workspaces.by_document(&p.text_document.uri);
    let doc = ws.document(&p.text_document.uri)?;

    Ok(Some(DocumentSymbolResponse::Nested(create_symbols(doc))))
}

pub(crate) fn create_symbols(doc: &DocumentState) -> Vec<DocumentSymbol> {
    let mapper = &doc.mapper;
    let mut symbols: Vec<DocumentSymbol> = Vec::new();

    let dom = doc.dom.clone();
    symbols_for_annotaions(&dom, mapper, &mut symbols);
    match dom {
        Node::Object(obj) => {
            symbols.extend(symbols_for_object(&obj, mapper));
        }
        Node::Array(arr) => {
            symbols.extend(symbols_for_array(&arr, mapper));
        }
        _ => {}
    }

    symbols
}

#[allow(deprecated)]
fn symbols_for_value(
    name: String,
    node: &Node,
    mapper: &Mapper,
    symbols: &mut Vec<DocumentSymbol>,
) {
    let range = node
        .node_text_range()
        .and_then(|v| mapper.range(v))
        .unwrap();
    let selection_range = node.text_range().and_then(|v| mapper.range(v)).unwrap();

    match node {
        Node::Null(_) => {}
        Node::Bool(_) => symbols.push(DocumentSymbol {
            name,
            kind: SymbolKind::BOOLEAN,
            range: range.into_lsp(),
            selection_range: selection_range.into_lsp(),
            detail: None,
            deprecated: None,
            tags: Default::default(),
            children: None,
        }),
        Node::String(_) => symbols.push(DocumentSymbol {
            name,
            kind: SymbolKind::STRING,
            range: range.into_lsp(),
            selection_range: selection_range.into_lsp(),
            detail: None,
            deprecated: None,
            tags: Default::default(),
            children: None,
        }),
        Node::Number(_) => symbols.push(DocumentSymbol {
            name,
            kind: SymbolKind::NUMBER,
            range: range.into_lsp(),
            selection_range: selection_range.into_lsp(),
            detail: None,
            deprecated: None,
            tags: Default::default(),
            children: None,
        }),
        Node::Array(arr) => symbols.push(DocumentSymbol {
            name,
            kind: SymbolKind::ARRAY,
            range: range.into_lsp(),
            selection_range: selection_range.into_lsp(),
            detail: None,
            deprecated: None,
            tags: Default::default(),
            children: Some(symbols_for_array(arr, mapper)),
        }),
        Node::Object(obj) => {
            symbols.push(DocumentSymbol {
                name,
                kind: SymbolKind::OBJECT,
                range: range.into_lsp(),
                selection_range: selection_range.into_lsp(),
                detail: None,
                deprecated: None,
                tags: Default::default(),
                children: Some(symbols_for_object(obj, mapper)),
            });
        }
    }
}

fn symbols_for_annotaions(node: &Node, mapper: &Mapper, symbols: &mut Vec<DocumentSymbol>) {
    if let Some(annotations) = node.annotations() {
        for (k, v) in annotations.value().read().iter() {
            symbols_for_value(k.annotation_name(), v, mapper, symbols);
        }
    }
}

fn symbols_for_array(arr: &dom::Array, mapper: &Mapper) -> Vec<DocumentSymbol> {
    let items = arr.value().read();
    let mut symbols = vec![];

    for (index, value) in items.iter().enumerate() {
        symbols_for_annotaions(value, mapper, &mut symbols);
        symbols_for_value(index.to_string(), value, mapper, &mut symbols);
    }

    symbols
}

fn symbols_for_object(obj: &dom::Object, mapper: &Mapper) -> Vec<DocumentSymbol> {
    let properties = obj.value().read();
    let mut symbols = vec![];
    for (key, value) in properties.iter() {
        symbols_for_annotaions(value, mapper, &mut symbols);
        symbols_for_value(key.to_string(), value, mapper, &mut symbols);
    }
    symbols
}
