use jsona_util::environment::Environment;
use lsp_async_stub::Server;
use lsp_types::{notification, request};
use std::sync::Arc;
use world::{World, WorldState};

mod diagnostics;
mod handlers;

pub mod config;
pub mod lsp_ext;
pub mod query;
pub mod world;

#[must_use]
pub fn create_server<E: Environment>() -> Server<World<E>> {
    Server::new()
        .on_request::<request::Initialize, _>(handlers::initialize)
        .on_request::<request::FoldingRangeRequest, _>(handlers::folding_ranges)
        .on_request::<request::DocumentSymbolRequest, _>(handlers::document_symbols)
        .on_request::<request::Completion, _>(handlers::completion)
        .on_request::<request::HoverRequest, _>(handlers::hover)
        .on_request::<request::Formatting, _>(handlers::format)
        .on_notification::<notification::Initialized, _>(handlers::initialized)
        .on_notification::<notification::DidOpenTextDocument, _>(handlers::document_open)
        .on_notification::<notification::DidChangeTextDocument, _>(handlers::document_change)
        .on_notification::<notification::DidSaveTextDocument, _>(handlers::document_save)
        .on_notification::<notification::DidCloseTextDocument, _>(handlers::document_close)
        .on_notification::<notification::DidChangeConfiguration, _>(handlers::configuration_change)
        .on_notification::<notification::DidChangeWorkspaceFolders, _>(handlers::workspace_change)
        .on_request::<lsp_ext::request::ListSchemasRequest, _>(handlers::list_schemas)
        .on_request::<lsp_ext::request::AssociatedSchemaRequest, _>(handlers::associated_schema)
        .on_notification::<lsp_ext::notification::AssociateSchema, _>(handlers::associate_schema)
        .build()
}

pub fn create_world<E: Environment>(env: E) -> World<E> {
    Arc::new(WorldState::new(env))
}
