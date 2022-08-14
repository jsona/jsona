use environment::WasmEnvironment;
use jsona::{formatter, parser::parse};
use jsona_util::{config::Config, schema::Schemas};
use serde::Serialize;
use url::Url;
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
pub fn initialize() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn format(
    _env: JsValue,
    jsona: &str,
    options: JsValue,
    config: JsValue,
) -> Result<String, JsError> {
    let mut config = if config.is_undefined() {
        Config::default()
    } else {
        config.into_serde()?
    };

    // let env = WasmEnvironment::from(env);
    config
        .prepare(None)
        .map_err(|err| JsError::new(&err.to_string()))?;

    let camel_opts: formatter::OptionsIncompleteCamel = options.into_serde()?;
    let mut options = formatter::Options::default();
    if let Some(cfg_opts) = config.formatting.clone() {
        options.update(cfg_opts);
    }
    options.update_camel(camel_opts);

    let syntax = parse(jsona);

    Ok(formatter::format_syntax(syntax.into_syntax(), options))
}

#[wasm_bindgen]
pub async fn lint(env: JsValue, jsona: String, config: JsValue) -> Result<JsValue, JsError> {
    let mut config = if config.is_undefined() {
        Config::default()
    } else {
        config.into_serde()?
    };
    let env = WasmEnvironment::from(env);
    config
        .prepare(None)
        .map_err(|err| JsError::new(&err.to_string()))?;

    let syntax = parse(&jsona);

    if !syntax.errors.is_empty() {
        return Ok(JsValue::from_serde(&LintResult {
            errors: syntax
                .errors
                .into_iter()
                .map(|err| LintError {
                    range: Range {
                        start: err.range.start().into(),
                        end: err.range.end().into(),
                    }
                    .into(),
                    error: err.to_string(),
                })
                .collect(),
        })?);
    }

    let dom = syntax.into_dom();

    if let Err(errors) = dom.validate() {
        return Ok(JsValue::from_serde(&LintResult {
            errors: errors
                .map(|err| LintError {
                    range: None,
                    error: err.to_string(),
                })
                .collect(),
        })?);
    }

    let schemas = Schemas::new(env);
    schemas.associations().add_from_config(&config);

    if let Some(schema) = schemas
        .associations()
        .query_for(&Url::parse("file:///__.jsona").unwrap())
    {
        let schema_errors = schemas
            .validate(&schema.url, &dom)
            .await
            .map_err(|err| JsError::new(&err.to_string()))?;

        return Ok(JsValue::from_serde(&LintResult {
            errors: schema_errors
                .into_iter()
                .map(|err| LintError {
                    range: None,
                    error: err.to_string(),
                })
                .collect(),
        })?);
    }

    todo!()
}

#[cfg(feature = "cli")]
#[wasm_bindgen]
pub async fn run_cli(env: JsValue, args: JsValue) -> Result<(), JsError> {
    use clap::Parser;
    use environment::WasmEnvironment;
    use jsona_cli::{App, AppArgs, Colors};
    use jsona_util::{environment::Environment, log::setup_stderr_logging};
    use tokio::io::AsyncWriteExt;
    use tracing::Instrument;

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
#[wasm_bindgen]
pub fn create_lsp(env: JsValue, lsp_interface: JsValue) -> lsp::JsonaWasmLsp {
    use jsona_util::log::setup_stderr_logging;

    let env = WasmEnvironment::from(env);
    setup_stderr_logging(env.clone(), false, false, None);

    lsp::JsonaWasmLsp {
        server: jsona_lsp::create_server(),
        world: jsona_lsp::create_world(env),
        lsp_interface: lsp::WasmLspInterface::from(lsp_interface),
    }
}
