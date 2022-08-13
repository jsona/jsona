use crate::App;

use clap::Subcommand;
use jsona_util::{
    config::Config,
    environment::{native::NativeEnvironment, Environment},
};
use std::sync::Arc;

impl<E: Environment> App<E> {
    pub async fn execute_lsp(&self, cmd: LspCommand) -> Result<(), anyhow::Error> {
        let server = jsona_lsp::create_server();
        let world = jsona_lsp::create_world(NativeEnvironment::new());
        world.set_default_config(Arc::new(Config::default()));

        match cmd {
            LspCommand::Tcp { address } => {
                server
                    .listen_tcp(world, &address, async_ctrlc::CtrlC::new().unwrap())
                    .await
            }
            LspCommand::Stdio {} => {
                server
                    .listen_stdio(world, async_ctrlc::CtrlC::new().unwrap())
                    .await
            }
        }
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum LspCommand {
    /// Run the language server and listen on a TCP address.
    Tcp {
        /// The address to listen on.
        #[clap(long, default_value = "0.0.0.0:9182")]
        address: String,
    },
    /// Run the language server over the standard input and output.
    Stdio {},
}
