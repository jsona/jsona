use crate::{App, GeneralArgs};

use anyhow::anyhow;
use clap::Args;
use codespan_reporting::files::SimpleFile;
use jsona::{formatter, parser};
use jsona_util::environment::Environment;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl<E: Environment> App<E> {
    pub async fn execute_format(&mut self, cmd: FormatCommand) -> Result<(), anyhow::Error> {
        if cmd.files.is_empty() {
            self.format_stdin(cmd).await
        } else {
            self.format_files(cmd).await
        }
    }

    #[tracing::instrument(skip_all)]
    async fn format_stdin(&mut self, cmd: FormatCommand) -> Result<(), anyhow::Error> {
        let mut source = String::new();
        self.env.stdin().read_to_string(&mut source).await?;

        let display_path = "-";

        let p = parser::parse(&source);

        if !p.errors.is_empty() {
            self.print_parse_errors(&SimpleFile::new(display_path, source.as_str()), &p.errors)
                .await?;

            if !cmd.force {
                return Err(anyhow!("no formatting was done due to syntax errors"));
            }
        }
        let format_opts = self.format_options(&cmd)?;

        let formatted = formatter::format_syntax(p.into_syntax(), format_opts);

        if cmd.check {
            if source != formatted {
                return Err(anyhow!("the input was not properly formatted"));
            }
        } else {
            let mut stdout = self.env.stdout();
            stdout.write_all(formatted.as_bytes()).await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    async fn format_files(&mut self, cmd: FormatCommand) -> Result<(), anyhow::Error> {
        let mut result = Ok(());

        let format_opts = self.format_options(&cmd)?;

        for path in &cmd.files {
            let (url, source) = self
                .load_file(path)
                .await
                .map_err(|err| anyhow!("failed to read {path}, {err}"))?;

            let p = parser::parse(&source);

            if !p.errors.is_empty() {
                self.print_parse_errors(&SimpleFile::new(path, source.as_str()), &p.errors)
                    .await?;

                if !cmd.force {
                    result = Err(anyhow!(
                        "some files were not formatted due to syntax errors"
                    ));
                    continue;
                }
            }

            let formatted = formatter::format_syntax(p.into_syntax(), format_opts.clone());

            if cmd.check {
                if source != formatted {
                    tracing::error!(?path, "the file is not properly formatted");
                    result = Err(anyhow!("some files were not properly formatted"));
                }
            } else if source != formatted {
                self.env.write_file(&url, formatted.as_bytes()).await?;
            }
        }

        result
    }

    fn format_options(&self, cmd: &FormatCommand) -> Result<formatter::Options, anyhow::Error> {
        let mut format_opts = formatter::Options::default();
        format_opts.update_from_str(cmd.options.iter().filter_map(|s| {
            let mut split = s.split('=');
            let k = split.next();
            let v = split.next();

            if let (Some(k), Some(v)) = (k, v) {
                Some((k, v))
            } else {
                tracing::error!(option = %s, "malformed formatter option");
                None
            }
        }))?;

        Ok(format_opts)
    }
}

#[derive(Debug, Clone, Args)]
pub struct FormatCommand {
    #[clap(flatten)]
    pub general: GeneralArgs,

    /// A formatter option given as a "key=value", can be set multiple times.
    #[clap(long = "option", short)]
    pub options: Vec<String>,

    /// Ignore syntax errors and force formatting.
    #[clap(long, short)]
    pub force: bool,

    /// Dry-run and report any files that are not correctly formatted.
    #[clap(long)]
    pub check: bool,

    /// JSONA files to format.
    ///
    /// If the only argument is "-", the standard input will be used.
    pub files: Vec<String>,
}
