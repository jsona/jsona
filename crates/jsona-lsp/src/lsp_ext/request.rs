use lsp_types::{request::Request, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub enum ListSchemasRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSchemasParams {
    pub document_uri: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSchemasResponse {
    pub schemas: Vec<SchemaInfo>,
}

impl Request for ListSchemasRequest {
    type Params = ListSchemasParams;
    type Result = ListSchemasResponse;
    const METHOD: &'static str = "jsona/listSchemas";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaInfo {
    pub url: Url,
    pub meta: Value,
}

pub enum AssociatedSchemaRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssociatedSchemaParams {
    pub document_uri: Url,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssociatedSchemaResponse {
    pub schema: Option<SchemaInfo>,
}

impl Request for AssociatedSchemaRequest {
    type Params = AssociatedSchemaParams;
    type Result = AssociatedSchemaResponse;
    const METHOD: &'static str = "jsona/associatedSchema";
}
