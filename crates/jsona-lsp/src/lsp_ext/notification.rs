use lsp_types::{notification::Notification, Url};
use serde::{Deserialize, Serialize};

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
