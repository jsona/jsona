use lsp_types::{notification::Notification, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub enum MessageWithOutput {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageKind {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageWithOutputParams {
    pub kind: MessageKind,
    pub message: String,
}

impl Notification for MessageWithOutput {
    type Params = MessageWithOutputParams;
    const METHOD: &'static str = "jsona/messageWithOutput";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssociationRule {
    Glob(String),
    Regex(String),
    Url(Url),
}

pub enum AssociateSchemas {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssociateSchemasParams {
    pub associations: Vec<AssociateSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssociateSchema {
    pub schema_uri: Url,
    pub rule: AssociationRule,
    pub meta: Option<Value>,
}

impl Notification for AssociateSchemas {
    type Params = AssociateSchemasParams;
    const METHOD: &'static str = "jsona/associateSchemas";
}

pub enum InitializeWorkspace {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeWorkspaceParams {
    pub root_uri: Url,
}

impl Notification for InitializeWorkspace {
    type Params = InitializeWorkspaceParams;
    const METHOD: &'static str = "jsona/initializeWorkspace";
}
