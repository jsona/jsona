#[cfg(feature = "lsp")]
use self::lsp::LspCommand;
use self::{format::FormatCommand, lint::LintCommand, queries::GetCommand};
use crate::App;

mod format;
mod lint;
#[cfg(feature = "lsp")]
mod lsp;
mod queries;

use clap::{crate_version, ArgEnum, Args, Parser, Subcommand};
use jsona_util::environment::Environment;
use std::path::PathBuf;

impl<E: Environment> App<E> {
    pub async fn execute(&mut self, args: AppArgs) -> Result<(), anyhow::Error> {
        self.colors = match args.colors {
            Colors::Auto => self.env.atty_stderr(),
            Colors::Always => true,
            Colors::Never => false,
        };

        match args.cmd {
            JsonaCommand::Format(cmd) => self.execute_format(cmd).await,
            #[cfg(feature = "lsp")]
            JsonaCommand::Lsp { cmd } => {
                #[cfg(feature = "lsp")]
                {
                    self.execute_lsp(cmd).await
                }
                #[cfg(not(feature = "lsp"))]
                {
                    let _ = cmd;
                    return Err(anyhow::anyhow!("the LSP is not part of this build, please consult the documentation about enabling the functionality"));
                }
            }
            JsonaCommand::Lint(cmd) => self.execute_lint(cmd).await,
            JsonaCommand::Get(cmd) => self.execute_get(cmd).await,
        }
    }
}

#[derive(Clone, Copy, ArgEnum)]
pub enum Colors {
    /// Determine whether to colorize output automatically.
    Auto,
    /// Always colorize output.
    Always,
    /// Never colorize output.
    Never,
}

#[derive(Clone, Parser)]
#[clap(name = "jsona")]
#[clap(bin_name = "jsona")]
#[clap(version = crate_version!())]
pub struct AppArgs {
    #[clap(long, arg_enum, global = true, default_value = "auto")]
    pub colors: Colors,
    /// Enable a verbose logging format.
    #[clap(long, global = true)]
    pub verbose: bool,
    /// Enable logging spans.
    #[clap(long, global = true)]
    pub log_spans: bool,
    #[clap(subcommand)]
    pub cmd: JsonaCommand,
}

#[derive(Clone, Args)]
pub struct GeneralArgs {
    /// Path to the Jsona configuration file.
    #[clap(long, short)]
    pub config: Option<PathBuf>,

    /// Do not search for a configuration file.
    #[clap(long)]
    pub no_auto_config: bool,
}

#[derive(Clone, Subcommand)]
pub enum JsonaCommand {
    /// Lint JSONA documents.
    #[clap(visible_aliases = &["check", "validate"])]
    Lint(LintCommand),
    /// Format JSONA documents.
    ///
    /// Files are modified in-place unless the input comes from the standard input, in which case the formatted result is printed to the standard output.
    #[clap(visible_aliases = &["fmt"])]
    Format(FormatCommand),
    /// Language server operations.
    #[cfg(feature = "lsp")]
    Lsp {
        #[clap(subcommand)]
        cmd: LspCommand,
    },
    /// Extract a value from the given JSONA document.
    Get(GetCommand),
}
