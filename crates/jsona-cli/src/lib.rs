use jsona_util::{environment::Environment, schema::Schemas};

pub use crate::commands::{AppArgs, Colors, GeneralArgs};

pub mod commands;
pub mod printing;

pub struct App<E: Environment> {
    env: E,
    colors: bool,
    schemas: Schemas<E>,
}

impl<E: Environment> App<E> {
    pub fn new(env: E) -> Self {
        Self {
            schemas: Schemas::new(env.clone()),
            colors: env.atty_stderr(),
            env,
        }
    }
}
