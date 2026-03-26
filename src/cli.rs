use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ptmp")]
#[command(
    about = "Copy a template directory and render Tera placeholders via a TUI",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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

        /// Optional project name override (defaults to destination folder name)
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

    /// Emit an example.slurm job script into the current directory
    Slurm,

    /// Emit a .gitignore into the current directory
    Gitignore,

    /// Emit an .editorconfig into the current directory
    Editorconfig,

    /// Emit a CITATION.cff into the current directory
    Citation,

    /// Emit an MIT LICENSE into the current directory
    License,

    /// Emit a Quarto report (report.qmd) into the current directory
    Quarto,

    /// Emit a Quarto revealjs slide deck (slides.qmd) into the current directory
    Slides,
}
