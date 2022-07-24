use crate::App;

use anyhow::anyhow;
use clap::Args;
use codespan_reporting::files::SimpleFile;
use jsona::{dom::Keys, parser};
use jsona_util::environment::Environment;
use serde_json::Value;
use std::{borrow::Cow, path::PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl<E: Environment> App<E> {
    pub async fn execute_get(&self, cmd: GetCommand) -> Result<(), anyhow::Error> {
        let mut stdout = self.env.stdout();

        let source = match &cmd.file_path {
            Some(p) => String::from_utf8(self.env.read_file(p).await?)?,
            None => {
                let mut stdin = self.env.stdin();
                let mut s = String::new();
                stdin.read_to_string(&mut s).await?;
                s
            }
        };

        let parse = parser::parse(&source);

        let file_path = cmd
            .file_path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or(Cow::Borrowed("-"));

        self.print_parse_errors(&SimpleFile::new(&file_path, &source), &parse.errors)
            .await?;

        if !parse.errors.is_empty() {
            return Err(anyhow!("syntax errors found"));
        }

        let node = parse.into_dom();

        if let Err(errors) = node.validate() {
            self.print_semantic_errors(&SimpleFile::new(&file_path, &source), errors)
                .await?;

            return Err(anyhow!("semantic errors found"));
        }

        let nodes = match cmd.pattern {
            Some(p) => {
                let p = p.trim_start_matches('.');

                let keys = p
                    .parse::<Keys>()
                    .map_err(|err| anyhow!("invalid pattern: {err}"))?;

                node.matches_all(keys, false)
                    .map_err(|err| anyhow!("invalid pattern: {err}"))?
                    .map(|(_, v)| v)
                    .collect()
            }
            None => vec![node],
        };
        let buf = {
            let items: Vec<Value> = nodes.iter().map(|v| v.to_plain_json()).collect();
            let value = if items.len() == 1 {
                items[0].clone()
            } else {
                Value::Array(items)
            };
            if let Some(value) = value.as_str() {
                value.as_bytes().to_vec()
            } else {
                serde_json::to_vec_pretty(&value).unwrap()
            }
        };
        stdout.write_all(&buf).await?;
        stdout.flush().await?;
        Ok(())
    }
}

#[derive(Clone, Args)]
pub struct GetCommand {
    /// Path to the JSONA document, if omitted the standard input will be used.
    #[clap(short, long)]
    pub file_path: Option<PathBuf>,

    /// A dotted key pattern to the value within the JSONA document.
    ///
    /// If omitted, the entire document will be printed.
    ///
    /// If the pattern yielded no values, the operation will fail.
    ///
    /// The pattern supports `jq`-like syntax and glob patterns as well:
    ///
    /// Examples:
    ///
    /// - table.array[1].foo
    /// - table.array.1.foo
    /// - table.array[*].foo
    /// - table.array.*.foo
    /// - dependencies.tokio-*.version
    ///
    pub pattern: Option<String>,
}
