use clap::StructOpt;
use jsona_cli::{
    commands::{AppArgs, Colors},
    App,
};
use jsona_common::{environment::native::NativeEnvironment, log::setup_stderr_logging};
use std::process::exit;
use tracing::Instrument;

#[tokio::main]
async fn main() {
    let cli = AppArgs::parse();
    setup_stderr_logging(
        NativeEnvironment::new(),
        cli.log_spans,
        cli.verbose,
        match cli.colors {
            Colors::Auto => None,
            Colors::Always => Some(true),
            Colors::Never => Some(false),
        },
    );

    match App::new(NativeEnvironment::new())
        .execute(cli)
        .instrument(tracing::info_span!("jsona"))
        .await
    {
        Ok(_) => {
            exit(0);
        }
        Err(error) => {
            tracing::error!(error = %format!("{error:#}"), "operation failed");
            exit(1);
        }
    }
}
