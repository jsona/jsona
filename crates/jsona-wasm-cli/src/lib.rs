use std::str::FromStr;

use environment::WasmEnvironment;
use gloo_utils::format::JsValueSerdeExt;
use jsona::{
    dom::Node,
    error::ErrorObject,
    formatter::{self, Options},
    util::mapper::Mapper,
};
use jsona_util::schema::Schemas;
use serde::Serialize;
use wasm_bindgen::prelude::*;

mod environment;
#[cfg(feature = "lsp")]
mod lsp;

#[derive(Serialize)]
struct Range {
    start: u32,
    end: u32,
}

#[derive(Serialize)]
struct LintError {
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<Range>,
    error: String,
}

#[derive(Serialize)]
struct LintResult {
    errors: Vec<LintError>,
}

#[wasm_bindgen]
pub fn format(input: &str, format_options: JsValue) -> Result<String, JsError> {
    let mut options: Options = Options::default();
    options.update(
        format_options
            .into_serde()
            .map_err(|_| JsError::new("invalid format options"))?,
    );
    Ok(formatter::format(input, options))
}

#[wasm_bindgen]
pub async fn lint(env: JsValue, input: String, schema_url: String) -> JsValue {
    let mapper = Mapper::new_utf16(&input, false);
    let env = WasmEnvironment::from(env);
    let node = match Node::from_str(&input) {
        Ok(v) => v,
        Err(err) => return JsValue::from_serde(&err.to_error_objects(&mapper)).unwrap(),
    };

    let schemas = Schemas::new(env);

    let mut errors = vec![];
    if let Ok(url) = schema_url.parse() {
        if let Some(schema) = schemas.associations().query_for(&url) {
            match schemas.validate(&schema.url, &node).await {
                Ok(errs) => {
                    errors.extend(errs.into_iter().map(|v| v.to_error_object(&node, &mapper)));
                }
                Err(err) => {
                    errors.push(ErrorObject {
                        source: "unknown".to_string(),
                        kind: "Unknown".to_string(),
                        message: err.to_string(),
                        range: None,
                    });
                }
            };
        }
    }
    JsValue::from_serde(&errors).unwrap()
}

#[cfg(feature = "cli")]
#[wasm_bindgen(js_name = runCli)]
pub async fn run_cli(env: JsValue, args: JsValue) -> Result<(), JsError> {
    use clap::Parser;
    use environment::WasmEnvironment;
    use jsona_cli::{App, AppArgs, Colors};
    use jsona_util::{environment::Environment, log::setup_stderr_logging};
    use tokio::io::AsyncWriteExt;
    use tracing::Instrument;

    console_error_panic_hook::set_once();
    let env = WasmEnvironment::from(env);
    let args: Vec<String> = args.into_serde()?;

    let cli = match AppArgs::try_parse_from(args) {
        Ok(v) => v,
        Err(error) => {
            env.stdout().write_all(error.to_string().as_bytes()).await?;
            return Err(JsError::new("operation failed"));
        }
    };

    setup_stderr_logging(
        env.clone(),
        cli.log_spans,
        cli.verbose,
        match cli.colors {
            Colors::Auto => None,
            Colors::Always => Some(true),
            Colors::Never => Some(false),
        },
    );

    match App::new(env.clone())
        .execute(cli)
        .instrument(tracing::info_span!("jsona"))
        .await
    {
        Ok(_) => Ok(()),
        Err(error) => {
            tracing::error!(error = %format!("{error:#}"), "operation failed");
            Err(JsError::new("operation failed"))
        }
    }
}

#[cfg(feature = "lsp")]
#[wasm_bindgen(js_name = createLsp)]
pub fn create_lsp(env: JsValue, lsp_interface: JsValue) -> lsp::JsonaWasmLsp {
    use jsona_util::log::setup_stderr_logging;

    console_error_panic_hook::set_once();
    let env = WasmEnvironment::from(env);
    setup_stderr_logging(env.clone(), false, false, None);

    lsp::JsonaWasmLsp {
        server: jsona_lsp::create_server(),
        world: jsona_lsp::create_world(env),
        lsp_interface: lsp::WasmLspInterface::from(lsp_interface),
    }
}
