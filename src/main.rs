//! ptmp -- Project Template Copier
//!
//! A single-binary CLI that scaffolds research project directories
//! from a template, rendering Tera placeholders via interactive TUI
//! prompts. Targets academic workflows with LaTeX, R, Julia, and
//! Taskfile/Makefile scaffolding.
//!
//! # Subcommands
//!
//! - `init` -- create a default template directory with example
//!   files and a `fields.toml` manifest.
//! - `new`  -- copy a template to a destination, collecting field
//!   values via TUI and rendering Tera placeholders in all UTF-8
//!   files. Binary files are copied without rendering.
//! - `taskfile`, `latex`, `makefile`, `readme`, `r`, `julia`,
//!   `project-toml` -- emit a single component file into the
//!   current working directory.
//!
//! # Template rendering
//!
//! Templates use the [Tera](https://keats.github.io/tera/) engine
//! with autoescape disabled (templates are LaTeX/Markdown, not
//! HTML). The manifest file `fields.toml` defines which variables
//! are prompted for; built-in variables (`project_name`, `today`,
//! `year`) are injected automatically.

use anyhow::Result;
use clap::Parser;
use sanitize_filename::sanitize;
use std::collections::HashMap;

mod cli;
mod commands;
mod components;
mod manifest;
mod render;
mod templates;

use cli::{Cli, Commands};
use commands::{emit_component, ensure_destination_ready, init_default_template};
use components::component_for_command;
use manifest::{collect_field_values, load_fields_manifest};
use render::{copy_template, make_tera_context};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { template_path } => {
            println!(
                "Initializing default template at '{}'",
                template_path.display()
            );
            init_default_template(template_path)?;
            println!("Default template created.");
        }
        Commands::New {
            template_path,
            destination,
            project_name,
        } => {
            if !template_path.exists() || !template_path.is_dir() {
                anyhow::bail!(
                    "Template path '{}' does not exist \
                     or is not a directory.",
                    template_path.display()
                );
            }

            ensure_destination_ready(destination)?;

            let proj_name = project_name.clone().unwrap_or_else(|| {
                destination
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "new_project".to_string())
            });
            let proj_name_sanitized = sanitize(&proj_name);

            let manifest_opt = load_fields_manifest(template_path)?;
            let mut values = HashMap::new();
            let enable_rendering = manifest_opt.is_some();

            if let Some(manifest) = manifest_opt {
                println!("Collecting template fields via TUI…");
                values = collect_field_values(&manifest)?;
            } else {
                println!(
                    "No 'fields.toml' found. \
                     Copying without rendering (binary-safe)."
                );
            }

            let tera_ctx = make_tera_context(values, &proj_name_sanitized);

            copy_template(template_path, destination, &tera_ctx, enable_rendering)?;

            println!(
                "Project '{}' created at '{}'",
                proj_name_sanitized,
                destination.display()
            );
        }
        cmd => {
            let component = component_for_command(cmd).expect("Unhandled command variant");
            emit_component(component)?;
        }
    }

    Ok(())
}
