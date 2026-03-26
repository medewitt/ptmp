# ptmp

A single-binary Rust CLI that copies a template directory and renders [Tera](https://keats.github.io/tera/) placeholders via an interactive TUI.
Targets research and academic projects with LaTeX, R, Julia, Quarto, and Taskfile/Makefile scaffolding.

## Installation

```bash
cargo install --path .
```

## Usage

### Create a default template

Generate a starter template directory with example files and a `fields.toml` manifest.

```bash
ptmp init --template-path ~/templates/my-template
```

### Create a new project from a template

Copy a template directory to a new location, prompting for field values defined in `fields.toml`.

```bash
ptmp new \
  --template-path ~/templates/latex_analytics \
  --destination ~/projects/WNV-modeling
```

### Emit a single component

Write one file into the current working directory, prompting only for the fields that file needs.

```bash
ptmp taskfile       # Taskfile.yml
ptmp latex          # main.tex
ptmp makefile       # Makefile
ptmp readme         # README.md
ptmp r              # R/example.R
ptmp julia          # src/example.jl
ptmp project-toml   # Project.toml
ptmp slurm          # example.slurm
ptmp gitignore      # .gitignore
ptmp editorconfig   # .editorconfig
ptmp citation       # CITATION.cff
ptmp license        # LICENSE (MIT)
ptmp quarto         # report.qmd
ptmp slides         # slides.qmd
```

## Template rendering

Templates use Tera syntax with autoescape disabled (safe for LaTeX and Markdown).
Any UTF-8 file in the template directory can contain placeholders like `{{ title }}` or `{{ author }}`.
Binary files are detected automatically and copied without rendering.

Built-in variables injected into every render context:

| Variable | Value |
|---|---|
| `project_name` | Sanitised destination folder name |
| `today` | Current date (`YYYY-MM-DD`) |
| `year` | Current four-digit year |

## `fields.toml`

Place a `fields.toml` file in the template root to define which variables the TUI prompts for.
Each `[[fields]]` entry maps to one interactive prompt.

```toml
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
```

Fields are referenced in templates as `{{ name }}`, e.g. `{{ title }}` or `{{ author }}`.
When `required` is true, the prompt rejects empty input.
The `default` value is pre-filled in the prompt and used if the user presses enter without typing.

## License

MIT
