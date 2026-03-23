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

use anyhow::{Context, Result};
use chrono::Local;
use clap::{Parser, Subcommand};
use inquire::{Confirm, Text};
use sanitize_filename::sanitize;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};
use walkdir::WalkDir;

// ---------------------------------------------------------------
// Templates
// ---------------------------------------------------------------

mod templates {
    pub const README: &str = r#"# {{ title }}

**Author:** {{ author }}
**Affiliation:** {{ affiliation }}
**Date:** {{ today }}

## Overview
This project was generated on {{ today }} ({{ year }}) via the template copier.

## Structure
- `src/`     — source code (Julia scripts, etc.)
- `figures/` — plots and figures
- `R/`       — R scripts
- `output/`  — generated outputs/results

## Notes
- The root `.here` file marks the project directory for the R `{here}` package.
- Julia project managed via `Project.toml`.
- LaTeX main file: `main.tex`
- Run `task --list` to see available development tasks.
"#;

    pub const MAIN_TEX: &str = r#"\documentclass[12pt,letterpaper]{article}
\usepackage{setspace}
% Packages
\usepackage[margin=1in]{geometry}
\usepackage{graphicx} % Required for inserting images
\usepackage{comment}
\usepackage[T1]{fontenc}
\usepackage[utf8]{inputenc}
\usepackage{lmodern}
\usepackage{natbib}

\title{ {{ title }} }
\author{ {{ author }} \\ {{ affiliation }} }
\date{ {{ today }} }

\begin{document}
\maketitle

\section{Introduction}
Write your introduction here.

\section{Methods}
Describe your methods here.

\section{Results}
Present results here. Figures can be stored in `figures/`.

\section{Discussion}
Discuss findings here.

\bibliographystyle{plain}
\bibliography{references}

\end{document}
"#;

    pub const REFERENCES_BIB: &str = r#"% Bibliography for {{ title }}
% Add BibTeX entries below
"#;

    pub const MAKEFILE: &str = r#"############################################################
# Project Makefile — generated via template copier
#
# Targets:
#   make pdf       - build the PDF with latexmk
#   make watch     - watch and rebuild on changes (latexmk -pvc)
#   make figures   - run all R/*.R scripts
#   make open      - open the generated PDF
#   make clean     - clean LaTeX aux files in output/
#   make distclean - clean plus remove output/* artifacts
#   make check     - print configuration
############################################################

# --- Configuration (Tera-injected) ---
MAIN       := {{ main_file }}
OUTDIR     := output
FIGDIR     := figures

# LaTeX engine selection: 'pdf', 'xelatex', or 'lualatex'
# (Collected via TUI; defaults to 'pdf')
ENGINE_FLAG := {% if latex_engine == "xelatex" %}-xelatex{% elif latex_engine == "lualatex" %}-lualatex{% else %}-pdf{% endif %}

LATEXMK   := latexmk
RSCRIPTS  := $(wildcard R/*.R)
PDF       := $(OUTDIR)/$(MAIN:.tex=.pdf)

# Detect platform for 'open'
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    OPEN_CMD := open
else
    OPEN_CMD := xdg-open
endif

.PHONY: all pdf watch figures open clean distclean check ensure_dirs

all: pdf

ensure_dirs:
    @mkdir -p $(OUTDIR) $(FIGDIR)

# Build PDF with latexmk
pdf: ensure_dirs $(PDF)

$(PDF): $(MAIN)
    $(LATEXMK) $(ENGINE_FLAG) -interaction=nonstopmode -halt-on-error -file-line-error -outdir=$(OUTDIR) $(MAIN)

# Continuous preview (rebuild on file changes)
watch: ensure_dirs
    $(LATEXMK) $(ENGINE_FLAG) -pvc -interaction=nonstopmode -file-line-error -outdir=$(OUTDIR) $(MAIN)

# Run all R scripts in R/
figures: ensure_dirs
    @for f in $(RSCRIPTS); do \
        echo "Running $$f"; \
        Rscript "$$f"; \
    done

# Open the built PDF (macOS: open; Linux: xdg-open)
open: pdf
    $(OPEN_CMD) "$(PDF)"

# Clean LaTeX intermediate files from output/
clean:
    $(LATEXMK) -C -outdir=$(OUTDIR)
    @rm -f "$(PDF)"

# Deep clean all artifacts in output/
distclean: clean
    @rm -rf $(OUTDIR)/*

check:
    @echo "Main file:      $(MAIN)"
    @echo "Output dir:     $(OUTDIR)"
    @echo "Figures dir:    $(FIGDIR)"
    @echo "Engine flag:    $(ENGINE_FLAG)"
    @echo "R scripts:      $(RSCRIPTS)"
"#;

    pub const TASKFILE: &str = r#"# Taskfile for research project: {{ title }}
# https://taskfile.dev
version: "3"

vars:
  MAIN: "{{ main_file }}"
  OUTDIR: output
  FIGDIR: figures
  ENGINE:
    sh: |
      case "{{ latex_engine }}" in
        xelatex)  echo "-xelatex" ;;
        lualatex) echo "-lualatex" ;;
        *)        echo "-pdf" ;;
      esac

{% raw %}
tasks:
  default:
    desc: Build the PDF
    deps: [latex]

  latex:
    desc: Build PDF with latexmk
    cmds:
      - mkdir -p {{.OUTDIR}}
      - >-
        latexmk {{.ENGINE}}
        -interaction=nonstopmode
        -halt-on-error
        -file-line-error
        -outdir={{.OUTDIR}}
        {{.MAIN}}

  watch:
    desc: Continuous rebuild on file changes
    cmds:
      - >-
        latexmk {{.ENGINE}}
        -pvc
        -interaction=nonstopmode
        -file-line-error
        -outdir={{.OUTDIR}}
        {{.MAIN}}

  tikz:
    desc: Render TikZ standalone figures
    cmds:
      - mkdir -p {{.FIGDIR}}
      - |
        for f in figures/*.tex; do
          [ -f "$f" ] || continue
          latexmk {{.ENGINE}} \
            -interaction=nonstopmode \
            -outdir={{.FIGDIR}} "$f"
        done

  r:
    desc: Run all R scripts in R/
    cmds:
      - |
        for f in R/*.R; do
          [ -f "$f" ] || continue
          echo "Running $f"
          Rscript "$f"
        done

  julia:
    desc: Run Julia scripts in src/
    cmds:
      - |
        for f in src/*.jl; do
          [ -f "$f" ] || continue
          echo "Running $f"
          julia --project=. "$f"
        done

  figures:
    desc: Generate all figures (R + Julia)
    deps: [r, julia]

  clean:
    desc: Clean LaTeX auxiliary files
    cmds:
      - latexmk -C -outdir={{.OUTDIR}}
      - rm -f {{.OUTDIR}}/{{.MAIN}}

  open:
    desc: Open the generated PDF
    deps: [latex]
    cmds:
      - open {{.OUTDIR}}/{{.MAIN}}
{% endraw %}
"#;

    pub const R_EXAMPLE: &str = r#"# Example R script
# Project: {{ title }}
# Author: {{ author }}
# Date: {{ today }}

library(tidyverse)
library(data.table)
library(here)
# library(gtsummary)
# library(flextable)
# library(officer)

message(paste("Project root is:", here::here()))
"#;

    pub const JULIA_EXAMPLE: &str = r#"# Example Julia script
# Project: {{ title }}
# Author: {{ author }}
# Date: {{ today }}

using CSV, DataFrames, Plots

println("Project root: ", @__DIR__)
"#;

    pub const PROJECT_TOML: &str = r#"name = "{{ title }}"
authors = ["{{ author }}"]

[deps]
CSV = "0.10"
DataFrames = "1.6"
Plots = "1.40"
"#;

    pub const FIELDS_TOML: &str = r#"
# Define fields collected via TUI and injected into Tera templates
[[fields]]
name = "title"
prompt = "Project title"
default = "My Analysis"
required = true

[[fields]]
name = "author"
prompt = "Author name"
default = ""
required = true

[[fields]]
name = "affiliation"
prompt = "Affiliation (e.g., Institution/Lab)"
default = ""
required = false


[[fields]]
name = "main_file"
prompt = "Main LaTeX file name"
default = "main.tex"
required = false

[[fields]]
name = "latex_engine"
prompt = "LaTeX engine (pdf/xelatex/lualatex)"
default = "pdf"
required = true

"#;
}

// ---------------------------------------------------------------
// CLI
// ---------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(name = "ptmp")]
#[command(
    about = "Copy a template directory and render Tera placeholders via a TUI",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a default template skeleton at the given path
    Init {
        /// Path where the template skeleton will be created
        #[arg(short, long)]
        template_path: PathBuf,
    },

    /// Create a new project from a template directory
    New {
        /// Path to the template directory
        #[arg(short, long)]
        template_path: PathBuf,

        /// Destination directory for the new project
        #[arg(short, long)]
        destination: PathBuf,

        /// Optional project name override
        /// (defaults to destination folder name)
        #[arg(short, long)]
        project_name: Option<String>,
    },

    /// Emit a Taskfile.yml into the current directory
    Taskfile,

    /// Emit a main.tex into the current directory
    Latex,

    /// Emit a Makefile into the current directory
    #[command(name = "makefile")]
    Makefile,

    /// Emit a README.md into the current directory
    Readme,

    /// Emit R/example.R into the current directory
    R,

    /// Emit src/example.jl into the current directory
    Julia,

    /// Emit a Project.toml into the current directory
    #[command(name = "project-toml")]
    ProjectToml,
}

// ---------------------------------------------------------------
// Component registry
// ---------------------------------------------------------------

struct FieldDef {
    name: &'static str,
    prompt: &'static str,
    default: &'static str,
    required: bool,
}

static KNOWN_FIELDS: &[FieldDef] = &[
    FieldDef {
        name: "title",
        prompt: "Project title",
        default: "My Analysis",
        required: true,
    },
    FieldDef {
        name: "author",
        prompt: "Author name",
        default: "",
        required: true,
    },
    FieldDef {
        name: "affiliation",
        prompt: "Affiliation (e.g., Institution/Lab)",
        default: "",
        required: false,
    },
    FieldDef {
        name: "main_file",
        prompt: "Main LaTeX file name",
        default: "main.tex",
        required: false,
    },
    FieldDef {
        name: "latex_engine",
        prompt: "LaTeX engine (pdf/xelatex/lualatex)",
        default: "pdf",
        required: true,
    },
];

struct Component {
    name: &'static str,
    output_path: &'static str,
    template: &'static str,
    fields: &'static [&'static str],
}

static COMPONENTS: &[Component] = &[
    Component {
        name: "taskfile",
        output_path: "Taskfile.yml",
        template: templates::TASKFILE,
        fields: &["title", "main_file", "latex_engine"],
    },
    Component {
        name: "latex",
        output_path: "main.tex",
        template: templates::MAIN_TEX,
        fields: &["title", "author", "affiliation"],
    },
    Component {
        name: "makefile",
        output_path: "Makefile",
        template: templates::MAKEFILE,
        fields: &["main_file", "latex_engine"],
    },
    Component {
        name: "readme",
        output_path: "README.md",
        template: templates::README,
        fields: &["title", "author", "affiliation"],
    },
    Component {
        name: "r",
        output_path: "R/example.R",
        template: templates::R_EXAMPLE,
        fields: &["title", "author"],
    },
    Component {
        name: "julia",
        output_path: "src/example.jl",
        template: templates::JULIA_EXAMPLE,
        fields: &["title", "author"],
    },
    Component {
        name: "project-toml",
        output_path: "Project.toml",
        template: templates::PROJECT_TOML,
        fields: &["title", "author"],
    },
];

fn component_for_command(cmd: &Commands) -> Option<&'static Component> {
    let name = match cmd {
        Commands::Taskfile => "taskfile",
        Commands::Latex => "latex",
        Commands::Makefile => "makefile",
        Commands::Readme => "readme",
        Commands::R => "r",
        Commands::Julia => "julia",
        Commands::ProjectToml => "project-toml",
        _ => return None,
    };
    COMPONENTS.iter().find(|c| c.name == name)
}

// ---------------------------------------------------------------
// Template field manifest (`fields.toml`)
// ---------------------------------------------------------------

/// Top-level structure of a `fields.toml` manifest.
///
/// Each entry in `fields` describes one variable that the TUI
/// will prompt the user for. The collected values are injected
/// into the Tera rendering context alongside the built-in
/// variables (`project_name`, `today`, `year`).
#[derive(Debug, Deserialize)]
struct FieldsManifest {
    fields: Vec<Field>,
}

/// A single field definition inside `fields.toml`.
///
/// Example TOML:
///
/// ```toml
/// [[fields]]
/// name = "author"
/// prompt = "Author name"
/// default = ""
/// required = true
/// ```
#[derive(Debug, Deserialize)]
struct Field {
    /// Variable name referenced in Tera templates as `{{ name }}`.
    name: String,
    /// Human-readable prompt displayed in the TUI.
    prompt: String,
    /// Optional default value pre-filled in the prompt.
    default: Option<String>,
    /// When true, an empty response is rejected.
    #[serde(default)]
    required: bool,
}

// ---------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------

/// Return `true` if the entire file at `path` is valid UTF-8.
fn is_text_utf8(path: &Path) -> bool {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = Vec::new();
    if f.read_to_end(&mut buf).is_err() {
        return false;
    }
    std::str::from_utf8(&buf).is_ok()
}

/// Process a single file from the template directory.
fn render_or_copy_file(
    src: &Path,
    dst: &Path,
    tera_ctx: &TeraContext,
    enable_rendering: bool,
) -> Result<()> {
    if src.file_name().and_then(|n| n.to_str()) == Some("fields.toml") {
        return Ok(());
    }

    fs::create_dir_all(dst.parent().unwrap())?;

    if enable_rendering && is_text_utf8(src) {
        let mut s = String::new();
        File::open(src)?.read_to_string(&mut s)?;
        let rendered = Tera::one_off(&s, tera_ctx, false)
            .with_context(|| format!("Failed to render template file: {}", src.display()))?;
        let mut out = File::create(dst)?;
        out.write_all(rendered.as_bytes())?;
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}

/// Load `fields.toml` from the template root, if it exists.
fn load_fields_manifest(template_root: &Path) -> Result<Option<FieldsManifest>> {
    let manifest_path = template_root.join("fields.toml");
    if manifest_path.exists() {
        let s = fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
        let manifest: FieldsManifest = toml::from_str(&s)
            .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

/// Prompt the user for each field defined in the manifest.
fn collect_field_values(manifest: &FieldsManifest) -> Result<HashMap<String, String>> {
    let mut values = HashMap::new();
    for f in &manifest.fields {
        let mut prompt = Text::new(&f.prompt);
        if let Some(default) = &f.default {
            prompt = prompt.with_default(default);
        }
        if f.required {
            prompt = prompt.with_help_message("Required");
        }
        let val = prompt
            .prompt()
            .with_context(|| format!("Prompt failed for '{}'", f.name))?;
        if f.required && val.trim().is_empty() {
            anyhow::bail!("Field '{}' is required and cannot be empty", f.name);
        }
        values.insert(f.name.clone(), val);
    }
    Ok(values)
}

/// Prompt only for the fields required by a component.
fn collect_fields_for_component(component: &Component) -> Result<HashMap<String, String>> {
    let mut values = HashMap::new();
    for &needed in component.fields {
        let def = KNOWN_FIELDS
            .iter()
            .find(|f| f.name == needed)
            .unwrap_or_else(|| panic!("Unknown field '{needed}' in component registry"));
        let mut prompt = Text::new(def.prompt);
        if !def.default.is_empty() {
            prompt = prompt.with_default(def.default);
        }
        if def.required {
            prompt = prompt.with_help_message("Required");
        }
        let val = prompt
            .prompt()
            .with_context(|| format!("Prompt failed for '{}'", def.name))?;
        if def.required && val.trim().is_empty() {
            anyhow::bail!("Field '{}' is required and cannot be empty", def.name);
        }
        values.insert(def.name.to_string(), val);
    }
    Ok(values)
}

/// Build a [`TeraContext`] from collected field values.
fn make_tera_context(mut values: HashMap<String, String>, project_name: &str) -> TeraContext {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let year = Local::now().format("%Y").to_string();

    values.insert("project_name".to_string(), project_name.to_string());
    values.insert("today".to_string(), today);
    values.insert("year".to_string(), year);

    TeraContext::from_serialize(values).expect("Serializable context")
}

/// Recursively copy a template directory to `destination`.
fn copy_template(
    template_root: &Path,
    destination: &Path,
    tera_ctx: &TeraContext,
    enable_rendering: bool,
) -> Result<()> {
    for entry in WalkDir::new(template_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let src_path = entry.path();
        if src_path == template_root {
            continue;
        }
        let rel = src_path.strip_prefix(template_root).unwrap();
        let dst_path = destination.join(rel);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
        } else {
            render_or_copy_file(src_path, &dst_path, tera_ctx, enable_rendering)?;
        }
    }
    Ok(())
}

/// Ensure `destination` exists and is safe to write into.
fn ensure_destination_ready(destination: &Path) -> Result<()> {
    if destination.exists() {
        let is_empty = destination
            .read_dir()
            .map(|mut it| it.next().is_none())
            .unwrap_or(true);

        if !is_empty {
            let proceed = Confirm::new(&format!(
                "Destination '{}' is not empty. \
                 Continue and potentially overwrite files?",
                destination.display()
            ))
            .with_default(false)
            .prompt()
            .context("Confirmation prompt failed")?;

            if !proceed {
                anyhow::bail!("Aborted: destination not empty.");
            }
        }
    } else {
        fs::create_dir_all(destination)
            .with_context(|| format!("Failed to create destination {}", destination.display()))?;
    }
    Ok(())
}

/// Emit a single component file into the current directory.
fn emit_component(component: &Component) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to determine current directory")?;

    let project_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .map(sanitize)
        .unwrap_or_else(|| "project".to_string());

    let output = cwd.join(component.output_path);

    if output.exists() {
        let proceed = Confirm::new(&format!(
            "'{}' already exists. Overwrite?",
            component.output_path
        ))
        .with_default(false)
        .prompt()
        .context("Confirmation prompt failed")?;

        if !proceed {
            anyhow::bail!("Aborted.");
        }
    }

    let values = collect_fields_for_component(component)?;
    let ctx = make_tera_context(values, &project_name);

    let rendered = Tera::one_off(component.template, &ctx, false)
        .with_context(|| format!("Failed to render template for '{}'", component.name))?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, rendered)?;

    println!("Wrote {}", component.output_path);
    Ok(())
}

/// Scaffold a default template directory at `path`.
fn init_default_template(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    fs::create_dir_all(path.join("src"))?;
    fs::create_dir_all(path.join("figures"))?;
    fs::create_dir_all(path.join("R"))?;
    fs::create_dir_all(path.join("output"))?;

    File::create(path.join(".here"))?.write_all(b"")?;
    fs::write(path.join("README.md"), templates::README)?;
    fs::write(path.join("main.tex"), templates::MAIN_TEX)?;
    fs::write(path.join("references.bib"), templates::REFERENCES_BIB)?;
    fs::write(path.join("R").join("example.R"), templates::R_EXAMPLE)?;
    fs::write(path.join("fields.toml"), templates::FIELDS_TOML)?;
    fs::write(path.join("Makefile"), templates::MAKEFILE)?;
    fs::write(path.join("Project.toml"), templates::PROJECT_TOML)?;
    fs::write(
        path.join("src").join("example.jl"),
        templates::JULIA_EXAMPLE,
    )?;
    fs::write(path.join("Taskfile.yml"), templates::TASKFILE)?;

    Ok(())
}

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
