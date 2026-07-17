// Rust guideline compliant 2026-02-21

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectTemplate {
    pub(crate) schema_version: u32,
    pub(crate) language: String,
    pub(crate) template: TemplateMetadata,
    #[serde(default)]
    pub(crate) project: ProjectConfig,
    #[serde(default)]
    pub(crate) manifest: ManifestConfig,
    #[serde(default)]
    pub(crate) defaults: DefaultsConfig,
    pub(crate) license: Option<LicenseConfig>,
    #[serde(default)]
    pub(crate) dependencies: Vec<Dependency>,
    #[serde(default)]
    pub(crate) dev_dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateMetadata {
    pub(crate) name: String,
    #[expect(dead_code, reason = "Metadata is retained for future template listing")]
    pub(crate) description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ProjectConfig {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) edition: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            kind: "binary".to_owned(),
            edition: "2024".to_owned(),
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ManifestMode {
    #[default]
    Managed,
    Custom,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ManifestConfig {
    pub(crate) mode: ManifestMode,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DefaultsConfig {
    pub(crate) source_files: bool,
    pub(crate) readme: bool,
    pub(crate) gitignore: bool,
    pub(crate) license: bool,
    pub(crate) tests: bool,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            source_files: true,
            readme: true,
            gitignore: true,
            license: false,
            tests: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LicenseConfig {
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Dependency {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) features: Vec<String>,
    pub(crate) default_features: Option<bool>,
    pub(crate) optional: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FileMode {
    Create,
    #[default]
    Replace,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileTemplate {
    pub(crate) path: PathBuf,
    #[serde(default)]
    pub(crate) mode: FileMode,
    #[serde(default = "default_true")]
    pub(crate) render: bool,
    pub(crate) content: String,
}

#[derive(Debug)]
pub(crate) struct LoadedTemplate {
    pub(crate) project: ProjectTemplate,
    pub(crate) files: Vec<FileTemplate>,
}

#[derive(Debug)]
pub(crate) struct TemplateContext {
    project_name: String,
    language: String,
    template_name: String,
}

impl TemplateContext {
    pub(crate) fn from_target(target: &Path, language: &str, template_name: &str) -> Result<Self> {
        let project_name = target
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .with_context(|| format!("target path '{}' has no project name", target.display()))?;

        Ok(Self {
            project_name: project_name.to_owned(),
            language: language.to_owned(),
            template_name: template_name.to_owned(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct GeneratedFile {
    pub(crate) path: PathBuf,
    pub(crate) content: String,
}

pub(crate) fn load_template(language: &str, template_name: &str) -> Result<LoadedTemplate> {
    validate_selector("language", language)?;
    validate_selector("template", template_name)?;

    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("templates")
        .join(language)
        .join(template_name);

    load_template_directory(&directory, language, template_name).with_context(|| {
        format!("failed to load template '{template_name}' for language '{language}'")
    })
}

fn load_template_directory(
    directory: &Path,
    language: &str,
    template_name: &str,
) -> Result<LoadedTemplate> {
    let project_path = directory.join("project.toml");
    let source = fs::read_to_string(&project_path)
        .with_context(|| format!("could not read {}", project_path.display()))?;
    let project: ProjectTemplate = toml::from_str(&source)
        .with_context(|| format!("could not parse {}", project_path.display()))?;
    validate_project(&project, language, template_name)?;

    let files_directory = directory.join("files");
    let mut paths = if files_directory.exists() {
        fs::read_dir(&files_directory)
            .with_context(|| format!("could not read {}", files_directory.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "toml")
    });
    paths.sort();

    let mut target_paths = HashSet::new();
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let source = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        let file: FileTemplate = toml::from_str(&source)
            .with_context(|| format!("could not parse {}", path.display()))?;
        validate_file_path(&file.path).with_context(|| {
            format!(
                "{} contains invalid target path '{}'",
                path.strip_prefix(directory).unwrap_or(&path).display(),
                file.path.display()
            )
        })?;
        if !target_paths.insert(file.path.clone()) {
            bail!(
                "{} duplicates target path '{}'",
                path.strip_prefix(directory).unwrap_or(&path).display(),
                file.path.display()
            );
        }
        files.push(file);
    }

    Ok(LoadedTemplate { project, files })
}

fn validate_project(project: &ProjectTemplate, language: &str, template_name: &str) -> Result<()> {
    if project.schema_version != 1 {
        bail!(
            "unsupported schema_version {}; expected 1",
            project.schema_version
        );
    }
    if !matches!(project.language.as_str(), "rust" | "python") {
        bail!("unsupported language '{}'", project.language);
    }
    if project.language != language {
        bail!(
            "template language '{}' does not match CLI language '{language}'",
            project.language
        );
    }
    if project.template.name.trim().is_empty() {
        bail!("template name must not be empty");
    }
    if project.template.name != template_name {
        bail!(
            "template declares name '{}' instead of '{template_name}'",
            project.template.name
        );
    }
    if project.manifest.mode != ManifestMode::Managed {
        bail!("manifest mode 'custom' is not supported yet");
    }
    if project.defaults.license && project.license.is_none() {
        bail!("defaults.license is true but [license] is missing");
    }
    if let Some(license) = &project.license {
        license_text(&license.name)?;
    }
    for dependency in project.dependencies.iter().chain(&project.dev_dependencies) {
        if dependency.name.trim().is_empty() {
            bail!("dependency name must not be empty");
        }
    }
    Ok(())
}

pub(crate) fn validate_target(target: &Path) -> Result<()> {
    if target.as_os_str().is_empty() {
        bail!("target path must not be empty");
    }
    reject_symlinks(target)?;
    if target.is_file() {
        bail!("target '{}' is a file", target.display());
    }
    if target.is_dir()
        && fs::read_dir(target)
            .with_context(|| format!("could not read target '{}'", target.display()))?
            .next()
            .is_some()
    {
        bail!("target directory '{}' is not empty", target.display());
    }
    Ok(())
}

pub(crate) fn write_files(
    target: &Path,
    defaults: Vec<GeneratedFile>,
    custom: &[FileTemplate],
    context: &TemplateContext,
) -> Result<()> {
    let files = merge_files(defaults, custom, context)?;
    for (file, mode) in files {
        let destination = target.join(&file.path);
        if let Some(parent) = destination.parent() {
            reject_symlinks(parent)?;
            fs::create_dir_all(parent)
                .with_context(|| format!("could not create {}", parent.display()))?;
        }
        reject_symlinks(&destination)?;

        let mut options = OpenOptions::new();
        options.write(true);
        match mode {
            FileMode::Create => {
                options.create_new(true);
            }
            FileMode::Replace => {
                options.create(true).truncate(true);
            }
        }
        let mut output = options.open(&destination).with_context(|| match mode {
            FileMode::Create => format!(
                "could not create '{}': target already exists or is unavailable",
                destination.display()
            ),
            FileMode::Replace => format!("could not replace '{}'", destination.display()),
        })?;
        output
            .write_all(file.content.as_bytes())
            .with_context(|| format!("could not write '{}'", destination.display()))?;
    }
    Ok(())
}

fn merge_files(
    defaults: Vec<GeneratedFile>,
    custom: &[FileTemplate],
    context: &TemplateContext,
) -> Result<Vec<(GeneratedFile, FileMode)>> {
    let overrides: HashSet<&Path> = custom.iter().map(|file| file.path.as_path()).collect();
    let mut merged = Vec::with_capacity(defaults.len() + custom.len());

    for mut file in defaults
        .into_iter()
        .filter(|file| !overrides.contains(file.path.as_path()))
    {
        file.content = render(&file.content, context)
            .with_context(|| format!("could not render default file '{}'", file.path.display()))?;
        merged.push((file, FileMode::Replace));
    }
    for file in custom {
        let content = if file.render {
            render(&file.content, context).with_context(|| {
                format!("could not render custom file '{}'", file.path.display())
            })?
        } else {
            file.content.clone()
        };
        merged.push((
            GeneratedFile {
                path: file.path.clone(),
                content,
            },
            file.mode,
        ));
    }
    Ok(merged)
}

pub(crate) fn render(input: &str, context: &TemplateContext) -> Result<String> {
    let mut remaining = input;
    let mut output = String::with_capacity(input.len());
    while let Some(start) = remaining.find("{{") {
        if remaining[..start].contains("}}") {
            bail!("unexpected closing placeholder delimiter");
        }
        output.push_str(&remaining[..start]);
        let placeholder = &remaining[start + 2..];
        let end = placeholder
            .find("}}")
            .context("unterminated template placeholder")?;
        let name = placeholder[..end].trim();
        let value = match name {
            "project_name" => &context.project_name,
            "language" => &context.language,
            "template_name" => &context.template_name,
            _ => bail!("unknown template placeholder '{{{{ {name} }}}}'"),
        };
        output.push_str(value);
        remaining = &placeholder[end + 2..];
    }
    if remaining.contains("}}") {
        bail!("unexpected closing placeholder delimiter");
    }
    output.push_str(remaining);
    Ok(output)
}

pub(crate) fn license_text(name: &str) -> Result<&'static str> {
    match name {
        "MIT" => Ok(include_str!("../assets/licenses/MIT.txt")),
        "Apache-2.0" => Ok(include_str!("../assets/licenses/Apache-2.0.txt")),
        _ => bail!("unknown license '{name}'"),
    }
}

fn validate_selector(kind: &str, value: &str) -> Result<()> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        bail!("invalid {kind} name '{value}'");
    }
    Ok(())
}

fn validate_file_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        bail!("path must be relative and contain only normal components");
    }
    Ok(())
}

fn reject_symlinks(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!(
                    "symlink path component '{}' is not allowed",
                    current.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("could not inspect '{}'", current.display()));
            }
        }
    }
    Ok(())
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Result<Self> {
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
            let path = std::env::temp_dir().join(format!(
                "entre-test-{}-{nonce}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path)?;
            Ok(Self(path))
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            drop(fs::remove_dir_all(&self.0));
        }
    }

    const PROJECT: &str = r#"
schema_version = 1
language = "rust"

[template]
name = "demo"

[project]
type = "binary"
edition = "2024"

[manifest]
mode = "managed"
"#;

    fn fixture(project: &str, files: &[(&str, &str)]) -> Result<TempDirectory> {
        let directory = TempDirectory::new()?;
        fs::write(directory.0.join("project.toml"), project)?;
        if !files.is_empty() {
            fs::create_dir(directory.0.join("files"))?;
        }
        for (name, content) in files {
            fs::write(directory.0.join("files").join(name), content)?;
        }
        Ok(directory)
    }

    fn expected_error<T>(result: Result<T>) -> Result<anyhow::Error> {
        match result {
            Ok(_) => bail!("expected operation to fail"),
            Err(error) => Ok(error),
        }
    }

    #[test]
    fn loads_valid_project() -> Result<()> {
        let directory = fixture(PROJECT, &[])?;
        let loaded = load_template_directory(&directory.0, "rust", "demo")?;
        assert_eq!(loaded.project.schema_version, 1);
        assert_eq!(loaded.project.template.name, "demo");
        Ok(())
    }

    #[test]
    fn missing_dependency_lists_are_empty() -> Result<()> {
        let project: ProjectTemplate = toml::from_str(PROJECT)?;
        assert!(project.dependencies.is_empty());
        assert!(project.dev_dependencies.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_unknown_schema_version() -> Result<()> {
        let directory = fixture(
            &PROJECT.replace("schema_version = 1", "schema_version = 2"),
            &[],
        )?;
        let error = expected_error(load_template_directory(&directory.0, "rust", "demo"))?;
        assert!(error.to_string().contains("unsupported schema_version 2"));
        Ok(())
    }

    #[test]
    fn rejects_language_mismatch() -> Result<()> {
        let directory = fixture(PROJECT, &[])?;
        let error = expected_error(load_template_directory(&directory.0, "python", "demo"))?;
        assert!(error.to_string().contains("does not match CLI language"));
        Ok(())
    }

    #[test]
    fn loads_file_templates_alphabetically() -> Result<()> {
        let directory = fixture(
            PROJECT,
            &[
                ("z.toml", "path = 'z.txt'\ncontent = 'z'"),
                ("a.toml", "path = 'a.txt'\ncontent = 'a'"),
            ],
        )?;
        let loaded = load_template_directory(&directory.0, "rust", "demo")?;
        assert_eq!(loaded.files[0].path, Path::new("a.txt"));
        assert_eq!(loaded.files[1].path, Path::new("z.txt"));
        Ok(())
    }

    #[test]
    fn rejects_duplicate_target_path() -> Result<()> {
        let directory = fixture(
            PROJECT,
            &[
                ("a.toml", "path = 'same.txt'\ncontent = 'a'"),
                ("b.toml", "path = 'same.txt'\ncontent = 'b'"),
            ],
        )?;
        let error = expected_error(load_template_directory(&directory.0, "rust", "demo"))?;
        assert!(
            error
                .to_string()
                .contains("duplicates target path 'same.txt'")
        );
        Ok(())
    }

    #[test]
    fn rejects_absolute_file_path() -> Result<()> {
        let directory = fixture(
            PROJECT,
            &[("file.toml", "path = '/tmp/outside'\ncontent = 'bad'")],
        )?;
        let error = expected_error(load_template_directory(&directory.0, "rust", "demo"))?;
        assert!(
            error
                .to_string()
                .contains("invalid target path '/tmp/outside'")
        );
        Ok(())
    }

    #[test]
    fn rejects_parent_file_path() -> Result<()> {
        let directory = fixture(
            PROJECT,
            &[("file.toml", "path = '../outside'\ncontent = 'bad'")],
        )?;
        let error = expected_error(load_template_directory(&directory.0, "rust", "demo"))?;
        assert!(
            error
                .to_string()
                .contains("invalid target path '../outside'")
        );
        Ok(())
    }

    #[test]
    fn custom_file_overrides_default() -> Result<()> {
        let context = TemplateContext {
            project_name: "demo".to_owned(),
            language: "rust".to_owned(),
            template_name: "basic".to_owned(),
        };
        let merged = merge_files(
            vec![GeneratedFile {
                path: "src/main.rs".into(),
                content: "default".to_owned(),
            }],
            &[FileTemplate {
                path: "src/main.rs".into(),
                mode: FileMode::Replace,
                render: true,
                content: "custom {{ project_name }}".to_owned(),
            }],
            &context,
        )?;

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].0.content, "custom demo");
        Ok(())
    }

    #[test]
    fn rejects_unknown_placeholder() -> Result<()> {
        let context = TemplateContext {
            project_name: "demo".to_owned(),
            language: "rust".to_owned(),
            template_name: "basic".to_owned(),
        };
        let error = expected_error(render("{{ unknown }}", &context))?;
        assert!(error.to_string().contains("unknown template placeholder"));
        Ok(())
    }
}
