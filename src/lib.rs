// Recursion limit bump is to support Ritz, a JSX-like proc macro used for
// Rojo's web UI currently.
#![recursion_limit = "1024"]

pub mod cli;

#[cfg(test)]
mod tree_view;

mod auth_cookie;
mod change_processor;
mod glob;
mod json;
mod lua_ast;
mod message_queue;
mod multimap;
mod path_serializer;
mod project;
mod resolution;
mod rojo_ref;
mod serve_session;
mod session_id;
mod snapshot;
mod snapshot_middleware;
mod syncback;
mod variant_eq;
mod web;

// TODO: Work out what we should expose publicly

pub use project::*;
pub use rojo_ref::*;
pub use session_id::SessionId;
pub use snapshot::{
    InstanceContext, InstanceMetadata, InstanceSnapshot, InstanceWithMeta, InstanceWithMetaMut,
    RojoDescendants, RojoTree,
};
pub use snapshot_middleware::{snapshot_from_vfs, Middleware, ScriptType};
pub use syncback::{syncback_loop, FsSnapshot, SyncbackData, SyncbackSnapshot};
pub use web::interface as web_api;

// napi deviation: expose all internals

use napi_derive::napi;

#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[napi]
pub fn build(
    project: String,
    output: Option<String>,
    plugin: Option<String>,
    watch: bool,
) -> Result<(), napi::Error> {
    use crate::cli::BuildCommand;

    let command = BuildCommand {
        project: project.into(),
        output: output.map(Into::into),
        plugin: plugin.map(Into::into),
        watch,
    };

    command
        .run()
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn doc() -> Result<(), napi::Error> {
    use crate::cli::DocCommand;

    DocCommand {}
        .run()
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn fmt_project(project: String) -> Result<(), napi::Error> {
    use crate::cli::FmtProjectCommand;

    FmtProjectCommand {
        project: project.into(),
    }
    .run()
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn init(path: String, kind: Option<String>, skip_git: bool) -> Result<(), napi::Error> {
    use crate::cli::{InitCommand, InitKind};

    let kind = match kind.as_deref() {
        Some("place") | None => InitKind::Place,
        Some("plugin") => InitKind::Plugin,
        Some("model") => InitKind::Model,
        Some(other) => {
            return Err(napi::Error::from_reason(format!(
                "Invalid init kind '{}'. Valid values are: place, plugin, model",
                other
            )))
        }
    };

    InitCommand {
        path: path.into(),
        kind,
        skip_git,
    }
    .run()
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn plugin(subcommand: String) -> Result<(), napi::Error> {
    use crate::cli::{PluginCommand, PluginSubcommand};

    let subcommand = match subcommand.as_str() {
        "install" => PluginSubcommand::Install,
        "uninstall" => PluginSubcommand::Uninstall,
        other => {
            return Err(napi::Error::from_reason(format!(
                "Invalid plugin subcommand '{}'. Valid values are: install, uninstall",
                other
            )))
        }
    };

    PluginCommand { subcommand }
        .run()
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn serve(
    project: String,
    address: Option<String>,
    port: Option<u16>,
) -> Result<(), napi::Error> {
    use crate::cli::{ColorChoice, GlobalOptions, ServeCommand};
    use std::net::IpAddr;
    use std::str::FromStr;

    let address = if let Some(address_str) = address {
        Some(IpAddr::from_str(&address_str).map_err(|e| napi::Error::from_reason(e.to_string()))?)
    } else {
        None
    };

    let command = ServeCommand {
        project: project.into(),
        address,
        port,
    };

    let global = GlobalOptions {
        verbosity: 0,
        color: ColorChoice::Auto,
    };

    command
        .run(global)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn sourcemap(
    project: String,
    output: Option<String>,
    include_non_scripts: bool,
    watch: bool,
    absolute: bool,
) -> Result<(), napi::Error> {
    use crate::cli::SourcemapCommand;

    SourcemapCommand {
        project: project.into(),
        output: output.map(Into::into),
        include_non_scripts,
        watch,
        absolute,
    }
    .run()
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn sync(project: String, input: String, output: String) -> Result<(), napi::Error> {
    use crate::cli::SyncCommand;

    SyncCommand {
        project: project.into(),
        input: input.into(),
        output: output.into(),
    }
    .run()
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn syncback(
    project: String,
    input: String,
    list: bool,
    dry_run: bool,
    non_interactive: bool,
) -> Result<(), napi::Error> {
    use crate::cli::{ColorChoice, GlobalOptions, SyncbackCommand};

    let command = SyncbackCommand {
        project: project.into(),
        input: input.into(),
        list,
        dry_run,
        non_interactive,
    };

    let global = GlobalOptions {
        verbosity: 0,
        color: ColorChoice::Auto,
    };

    command
        .run(global)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn upload(
    project: String,
    asset_id: i64,
    cookie: Option<String>,
    api_key: Option<String>,
    universe_id: Option<i64>,
) -> Result<(), napi::Error> {
    use crate::cli::UploadCommand;

    UploadCommand {
        project: project.into(),
        cookie,
        api_key,
        universe_id: universe_id.map(|id| id as u64),
        asset_id: asset_id as u64,
    }
    .run()
    .map_err(|e| napi::Error::from_reason(e.to_string()))
}

#[napi]
pub fn help() -> Result<(), napi::Error> {
    use crate::cli::Options;
    use clap::CommandFactory;

    let mut command = Options::command();
    command
        .print_help()
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    println!();

    Ok(())
}

#[napi]
pub fn run_cli(args: Vec<String>) -> Result<(), napi::Error> {
    use crate::cli::Options;
    use clap::Parser;

    let options = match Options::try_parse_from(args) {
        Ok(options) => options,
        Err(e) => e.exit(),
    };

    let log_filter = match options.global.verbosity {
        0 => "info",
        1 => "info,librojo=debug",
        2 => "info,librojo=trace",
        _ => "trace",
    };

    let log_env = env_logger::Env::default().default_filter_or(log_filter);

    let _ = env_logger::Builder::from_env(log_env)
        .format_module_path(false)
        .format_timestamp(None)
        // Indent following lines equal to the log level label, like `[ERROR] `
        .format_indent(Some(8))
        .write_style(options.global.color.into())
        .try_init();

    if let Err(err) = options.run() {
        log::error!("{:?}", err);
        std::process::exit(1);
    }

    Ok(())
}
