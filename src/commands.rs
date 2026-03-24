use crate::components::{collect_fields_for_component, Component};
use crate::render::make_tera_context;
use crate::templates;
use anyhow::{Context, Result};
use inquire::Confirm;
use sanitize_filename::sanitize;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tera::Tera;

/// Scaffold a default template directory at `path`.
pub fn init_default_template(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    fs::create_dir_all(path.join("src"))?;
    fs::create_dir_all(path.join("figures"))?;
    fs::create_dir_all(path.join("R"))?;
    fs::create_dir_all(path.join("output"))?;
    fs::create_dir_all(path.join("notes"))?;

    File::create(path.join(".here"))?.write_all(b"")?;
    fs::write(path.join("README.md"), templates::README)?;
    fs::write(path.join("main.tex"), templates::MAIN_TEX)?;
    fs::write(path.join("references.bib"), templates::REFERENCES_BIB)?;
    fs::write(path.join("R").join("example.R"), templates::R_EXAMPLE)?;
    fs::write(path.join("fields.toml"), templates::FIELDS_TOML)?;
    fs::write(path.join("Makefile"), templates::MAKEFILE)?;
    fs::write(path.join("Project.toml"), templates::PROJECT_TOML)?;
    fs::write(path.join("src").join("example.jl"), templates::JULIA_EXAMPLE)?;
    fs::write(path.join("Taskfile.yml"), templates::TASKFILE)?;

    Ok(())
}

/// Ensure `destination` exists and is safe to write into.
///
/// - Creates the directory if it does not exist.
/// - Prompts for confirmation if the directory is non-empty.
pub fn ensure_destination_ready(destination: &Path) -> Result<()> {
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
pub fn emit_component(component: &Component) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // --- init_default_template ---

    #[test]
    fn init_default_template_creates_dirs() {
        let dir = tempdir().unwrap();
        init_default_template(dir.path()).unwrap();

        for subdir in &["src", "figures", "R", "output"] {
            assert!(
                dir.path().join(subdir).is_dir(),
                "missing subdirectory: {subdir}"
            );
        }
    }

    #[test]
    fn init_default_template_creates_files() {
        let dir = tempdir().unwrap();
        init_default_template(dir.path()).unwrap();

        let expected = &[
            "README.md",
            "main.tex",
            "Makefile",
            "Taskfile.yml",
            "fields.toml",
            "Project.toml",
            "references.bib",
            ".here",
            "R/example.R",
            "src/example.jl",
        ];
        for name in expected {
            assert!(
                dir.path().join(name).exists(),
                "missing file: {name}"
            );
        }
    }

    // --- ensure_destination_ready ---

    #[test]
    fn ensure_destination_ready_creates_missing_dir() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("new_project");
        assert!(!target.exists());
        ensure_destination_ready(&target).unwrap();
        assert!(target.is_dir());
    }

    #[test]
    fn ensure_destination_ready_accepts_empty_dir() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("empty");
        fs::create_dir_all(&target).unwrap();
        // Should succeed without prompting because the dir is empty.
        ensure_destination_ready(&target).unwrap();
        assert!(target.is_dir());
    }

    // Non-empty destination requires TUI confirmation — not unit-testable without
    // mocking `inquire::Confirm`. Covered manually / via integration testing.
}
