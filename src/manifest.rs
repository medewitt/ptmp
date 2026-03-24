use anyhow::{Context, Result};
use inquire::Text;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Top-level structure of a `fields.toml` manifest.
///
/// Each entry in `fields` describes one variable that the TUI
/// will prompt the user for. The collected values are injected
/// into the Tera rendering context alongside the built-in
/// variables (`project_name`, `today`, `year`).
#[derive(Debug, Deserialize)]
pub struct FieldsManifest {
    pub fields: Vec<Field>,
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
pub struct Field {
    /// Variable name referenced in Tera templates as `{{ name }}`.
    pub name: String,
    /// Human-readable prompt displayed in the TUI.
    pub prompt: String,
    /// Optional default value pre-filled in the prompt.
    pub default: Option<String>,
    /// When true, an empty response is rejected.
    #[serde(default)]
    pub required: bool,
}

/// Load `fields.toml` from the template root, if it exists.
pub fn load_fields_manifest(template_root: &Path) -> Result<Option<FieldsManifest>> {
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
pub fn collect_field_values(manifest: &FieldsManifest) -> Result<HashMap<String, String>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn load_fields_manifest_valid() {
        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("fields.toml");
        let mut f = std::fs::File::create(&manifest_path).unwrap();
        writeln!(
            f,
            r#"
[[fields]]
name = "title"
prompt = "Project title"
default = "My Analysis"
required = true

[[fields]]
name = "author"
prompt = "Author name"
required = false
"#
        )
        .unwrap();

        let result = load_fields_manifest(dir.path()).unwrap();
        assert!(result.is_some());
        let manifest = result.unwrap();
        assert_eq!(manifest.fields.len(), 2);
        assert_eq!(manifest.fields[0].name, "title");
        assert_eq!(manifest.fields[0].default.as_deref(), Some("My Analysis"));
        assert!(manifest.fields[0].required);
        assert_eq!(manifest.fields[1].name, "author");
        assert!(!manifest.fields[1].required);
    }

    #[test]
    fn load_fields_manifest_missing() {
        let dir = tempdir().unwrap();
        let result = load_fields_manifest(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_fields_manifest_invalid_toml() {
        let dir = tempdir().unwrap();
        let manifest_path = dir.path().join("fields.toml");
        std::fs::write(&manifest_path, b"not valid toml [[[").unwrap();
        let result = load_fields_manifest(dir.path());
        assert!(result.is_err());
    }
}
