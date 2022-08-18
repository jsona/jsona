use crate::{App, GeneralArgs};

use anyhow::{anyhow, Context};
use clap::Args;
use codespan_reporting::files::SimpleFile;
use jsona::parser;
use jsona_util::{
    environment::Environment,
    schema::associations::{AssociationRule, SchemaAssociation},
    util::to_file_uri,
};
use serde_json::json;
use tokio::io::AsyncReadExt;
use url::Url;

impl<E: Environment> App<E> {
    pub async fn execute_lint(&mut self, cmd: LintCommand) -> Result<(), anyhow::Error> {
        if !cmd.no_schema {
            if let Some(schema_uri) = &cmd.schema {
                let url: Url = self
                    .env
                    .to_file_uri(schema_uri)
                    .ok_or_else(|| anyhow!("invalid schema path `{}`", schema_uri))?;
                self.schemas.associations().add(
                    AssociationRule::glob("**")?,
                    SchemaAssociation {
                        meta: json!({"source": "command-line"}),
                        url,
                        priority: 999,
                    },
                );
            } else if let Some(store) = &cmd.schemastore {
                if !store.is_empty() {
                    let url = to_file_uri(store, &self.env.root())
                        .ok_or_else(|| anyhow!("invalid schemastore {store}"))?;

                    self.schemas
                        .associations()
                        .add_from_schemastore(&Some(url), &self.env.root())
                        .await
                        .with_context(|| "failed to load schema store")?;
                };
            } else {
                self.schemas
                    .associations()
                    .add_from_schemastore(&None, &self.env.root())
                    .await
                    .with_context(|| "failed to load schema store")?;
            }
        }

        if cmd.files.is_empty() {
            self.lint_stdin(cmd).await
        } else {
            self.lint_files(cmd).await
        }
    }

    #[tracing::instrument(skip_all)]
    async fn lint_stdin(&self, _cmd: LintCommand) -> Result<(), anyhow::Error> {
        self.lint_file("-", true).await
    }

    #[tracing::instrument(skip_all)]
    async fn lint_files(&mut self, cmd: LintCommand) -> Result<(), anyhow::Error> {
        let mut result = Ok(());

        for file in &cmd.files {
            if let Err(error) = self.lint_file(file, false).await {
                tracing::error!(%error, path = ?file, "invalid file");
                result = Err(anyhow!("some files were not valid"));
            }
        }

        result
    }

    #[tracing::instrument(skip_all, fields(%file_path))]
    async fn lint_file(&self, file_path: &str, stdin: bool) -> Result<(), anyhow::Error> {
        let (file_uri, source) = if stdin {
            let mut source = String::new();
            self.env
                .stdin()
                .read_to_string(&mut source)
                .await
                .map_err(|err| anyhow!("failed to read stdin, {err}"))?;
            ("file:///_".parse().unwrap(), source)
        } else {
            self.load_file(file_path)
                .await
                .map_err(|err| anyhow!("failed to read {file_path}, {err}"))?
        };
        let parse = parser::parse(&source);
        self.print_parse_errors(&SimpleFile::new(file_path, &source), &parse.errors)
            .await?;

        if !parse.errors.is_empty() {
            return Err(anyhow!("syntax errors found"));
        }

        let dom = parse.into_dom();

        if let Err(errors) = dom.validate() {
            self.print_semantic_errors(&SimpleFile::new(file_path, &source), errors)
                .await?;

            return Err(anyhow!("semantic errors found"));
        }

        self.schemas
            .associations()
            .add_from_document(&file_uri, &dom);

        if let Some(schema_association) = self.schemas.associations().query_for(&file_uri) {
            tracing::debug!(
                schema.url = %schema_association.url,
                schema.name = schema_association.meta["name"].as_str().unwrap_or(""),
                schema.source = schema_association.meta["source"].as_str().unwrap_or(""),
                "using schema"
            );

            let errors = self.schemas.validate(&schema_association.url, &dom).await?;

            if !errors.is_empty() {
                self.print_schema_errors(&SimpleFile::new(file_path, &source), &errors)
                    .await?;

                return Err(anyhow!("schema validation failed"));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Args)]
pub struct LintCommand {
    #[clap(flatten)]
    pub general: GeneralArgs,

    /// URL to the schema to be used for validation.
    #[clap(long)]
    pub schema: Option<String>,

    /// URL to a schema store (index).
    #[clap(long)]
    pub schemastore: Option<String>,

    /// Disable all schema validations.
    #[clap(long)]
    pub no_schema: bool,

    /// Paths or glob patterns to JSONA documents.
    pub files: Vec<String>,
}
