use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use tera::{Context as TeraContext, Tera};
use walkdir::WalkDir;

/// Return `true` if the entire file at `path` is valid UTF-8.
pub fn is_text_utf8(path: &Path) -> bool {
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

/// Build a [`TeraContext`] from collected field values.
///
/// Injects the built-in variables `project_name`, `today` (YYYY-MM-DD),
/// and `year` in addition to any user-supplied values.
pub fn make_tera_context(mut values: HashMap<String, String>, project_name: &str) -> TeraContext {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let year = Local::now().format("%Y").to_string();

    values.insert("project_name".to_string(), project_name.to_string());
    values.insert("today".to_string(), today);
    values.insert("year".to_string(), year);

    TeraContext::from_serialize(values).expect("Serializable context")
}

/// Process a single file from the template directory.
///
/// - Skips `fields.toml` entirely.
/// - Renders UTF-8 files through Tera when `enable_rendering` is true.
/// - Binary-copies non-UTF-8 files or when rendering is disabled.
pub fn render_or_copy_file(
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

/// Recursively copy a template directory to `destination`.
///
/// Applies Tera rendering to UTF-8 files when `enable_rendering` is true,
/// and skips `fields.toml`.
pub fn copy_template(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn simple_ctx() -> TeraContext {
        let mut vals = HashMap::new();
        vals.insert("title".to_string(), "Test Title".to_string());
        make_tera_context(vals, "test_project")
    }

    // --- is_text_utf8 ---

    #[test]
    fn is_text_utf8_valid() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("text.txt");
        fs::write(&p, "hello world").unwrap();
        assert!(is_text_utf8(&p));
    }

    #[test]
    fn is_text_utf8_binary() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("bin.dat");
        fs::write(&p, b"\xff\xfe\x00\x01").unwrap();
        assert!(!is_text_utf8(&p));
    }

    #[test]
    fn is_text_utf8_missing() {
        let p = PathBuf::from("/tmp/ptmp_test_nonexistent_abc123.txt");
        assert!(!is_text_utf8(&p));
    }

    // --- make_tera_context ---

    #[test]
    fn make_tera_context_injects_builtins() {
        let ctx = make_tera_context(HashMap::new(), "myproject");
        let json = ctx.clone().into_json();
        assert_eq!(json["project_name"], "myproject");
        let today = json["today"].as_str().unwrap();
        assert!(today.len() == 10, "today should be YYYY-MM-DD, got {today}");
        assert!(today.chars().nth(4) == Some('-'));
        let year = json["year"].as_str().unwrap();
        assert_eq!(year.len(), 4);
    }

    #[test]
    fn make_tera_context_preserves_user_values() {
        let mut vals = HashMap::new();
        vals.insert("author".to_string(), "Jane Doe".to_string());
        let ctx = make_tera_context(vals, "proj");
        let json = ctx.into_json();
        assert_eq!(json["author"], "Jane Doe");
    }

    // --- render_or_copy_file ---

    #[test]
    fn render_or_copy_file_renders_template() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("file.md");
        let dst = dst_dir.path().join("file.md");
        fs::write(&src, "# {{ title }}").unwrap();

        render_or_copy_file(&src, &dst, &simple_ctx(), true).unwrap();

        let content = fs::read_to_string(&dst).unwrap();
        assert_eq!(content, "# Test Title");
    }

    #[test]
    fn render_or_copy_file_skips_fields_toml() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("fields.toml");
        let dst = dst_dir.path().join("fields.toml");
        fs::write(&src, "[[fields]]").unwrap();

        render_or_copy_file(&src, &dst, &simple_ctx(), true).unwrap();

        assert!(!dst.exists(), "fields.toml should not be copied to destination");
    }

    #[test]
    fn render_or_copy_file_copies_without_rendering() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("file.txt");
        let dst = dst_dir.path().join("file.txt");
        let raw = "{{ title }} is not rendered";
        fs::write(&src, raw).unwrap();

        render_or_copy_file(&src, &dst, &simple_ctx(), false).unwrap();

        let content = fs::read_to_string(&dst).unwrap();
        assert_eq!(content, raw);
    }

    #[test]
    fn render_or_copy_file_binary_copy() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();
        let src = src_dir.path().join("image.png");
        let dst = dst_dir.path().join("image.png");
        let bytes: Vec<u8> = vec![0x89, 0x50, 0x4e, 0x47, 0xff, 0xfe];
        fs::write(&src, &bytes).unwrap();

        render_or_copy_file(&src, &dst, &simple_ctx(), true).unwrap();

        let copied = fs::read(&dst).unwrap();
        assert_eq!(copied, bytes);
    }

    // --- copy_template ---

    #[test]
    fn copy_template_preserves_structure() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let sub = src_dir.path().join("subdir");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("note.md"), "# {{ title }}").unwrap();
        fs::write(src_dir.path().join("README.md"), "# {{ title }}").unwrap();
        fs::write(src_dir.path().join("fields.toml"), "[[fields]]").unwrap();

        copy_template(src_dir.path(), dst_dir.path(), &simple_ctx(), true).unwrap();

        assert!(dst_dir.path().join("subdir").is_dir());
        assert!(dst_dir.path().join("subdir/note.md").exists());
        assert!(dst_dir.path().join("README.md").exists());
        assert!(
            !dst_dir.path().join("fields.toml").exists(),
            "fields.toml must be excluded"
        );

        let readme = fs::read_to_string(dst_dir.path().join("README.md")).unwrap();
        assert_eq!(readme, "# Test Title");
    }
}
