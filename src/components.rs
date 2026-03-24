use crate::cli::Commands;
use crate::templates;
use anyhow::{Context, Result};
use inquire::Text;
use std::collections::HashMap;

pub struct FieldDef {
    pub name: &'static str,
    pub prompt: &'static str,
    pub default: &'static str,
    pub required: bool,
}

pub static KNOWN_FIELDS: &[FieldDef] = &[
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

pub struct Component {
    pub name: &'static str,
    pub output_path: &'static str,
    pub template: &'static str,
    pub fields: &'static [&'static str],
}

pub static COMPONENTS: &[Component] = &[
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

/// Map a CLI command variant to its registered [`Component`], if applicable.
///
/// Returns `None` for `Init` and `New`, which are handled separately.
pub fn component_for_command(cmd: &Commands) -> Option<&'static Component> {
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

/// Prompt only for the fields required by a component.
pub fn collect_fields_for_component(component: &Component) -> Result<HashMap<String, String>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Commands;
    use std::path::PathBuf;

    #[test]
    fn component_for_command_taskfile() {
        let cmd = Commands::Taskfile;
        let comp = component_for_command(&cmd).expect("Taskfile should resolve");
        assert_eq!(comp.name, "taskfile");
        assert_eq!(comp.output_path, "Taskfile.yml");
    }

    #[test]
    fn component_for_command_all_emit_variants() {
        let cases: &[(Commands, &str)] = &[
            (Commands::Taskfile, "taskfile"),
            (Commands::Latex, "latex"),
            (Commands::Makefile, "makefile"),
            (Commands::Readme, "readme"),
            (Commands::R, "r"),
            (Commands::Julia, "julia"),
            (Commands::ProjectToml, "project-toml"),
        ];
        for (cmd, expected_name) in cases {
            let comp = component_for_command(cmd)
                .unwrap_or_else(|| panic!("Expected component for {expected_name}"));
            assert_eq!(comp.name, *expected_name);
        }
    }

    #[test]
    fn component_for_command_init_returns_none() {
        let cmd = Commands::Init {
            template_path: PathBuf::from("/tmp"),
        };
        assert!(component_for_command(&cmd).is_none());
    }

    #[test]
    fn component_for_command_new_returns_none() {
        let cmd = Commands::New {
            template_path: PathBuf::from("/tmp"),
            destination: PathBuf::from("/tmp/out"),
            project_name: None,
        };
        assert!(component_for_command(&cmd).is_none());
    }

    #[test]
    fn known_fields_covers_all_component_fields() {
        for comp in COMPONENTS {
            for &field_name in comp.fields {
                let found = KNOWN_FIELDS.iter().any(|f| f.name == field_name);
                assert!(
                    found,
                    "Component '{}' references field '{}' not in KNOWN_FIELDS",
                    comp.name, field_name
                );
            }
        }
    }
}
