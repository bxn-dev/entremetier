// Rust guideline compliant 2026-02-21

use crate::templates::{
    Dependency, GeneratedFile, LoadedTemplate, ProjectTemplate, TemplateContext, license_text,
    load_template, validate_target, write_files,
};
use anyhow::{Context, Result, bail};
use std::{fs, path::Path, process::Command};

pub(crate) fn create_project(target: &Path, template_name: &str) -> Result<()> {
    let loaded = load_template("rust", template_name)?;
    validate_rust_project(&loaded.project)?;
    validate_target(target)?;
    let context = TemplateContext::from_target(target, "rust", template_name)?;

    initialize_project(&loaded.project, target)?;
    remove_cargo_default(target.join("src/main.rs"))?;
    remove_cargo_default(target.join(".gitignore"))?;
    write_files(target, default_files(&loaded)?, &loaded.files, &context)?;
    install_dependencies(&loaded.project, target)?;
    run_cargo(target, &["fmt".to_owned()])?;

    println!(
        "Created Rust project '{}' from template '{}'",
        target.display(),
        template_name
    );
    Ok(())
}

fn validate_rust_project(template: &ProjectTemplate) -> Result<()> {
    if template.project.kind != "binary" {
        bail!(
            "Rust project type '{}' is not supported; expected 'binary'",
            template.project.kind
        );
    }
    if !matches!(
        template.project.edition.as_str(),
        "2015" | "2018" | "2021" | "2024"
    ) {
        bail!("unsupported Rust edition '{}'", template.project.edition);
    }
    Ok(())
}

fn initialize_project(template: &ProjectTemplate, target: &Path) -> Result<()> {
    let output = Command::new("cargo")
        .args(["init", "--bin", "--edition"])
        .arg(&template.project.edition)
        .arg(target)
        .output()
        .context("failed to start cargo init")?;
    if !output.status.success() {
        bail!(
            "cargo init failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn default_files(loaded: &LoadedTemplate) -> Result<Vec<GeneratedFile>> {
    let template = &loaded.project;
    let mut files = Vec::new();
    if template.defaults.source_files {
        files.push(generated(
            "src/main.rs",
            include_str!("../../assets/defaults/rust/main.rs"),
        ));
    }
    if template.defaults.readme {
        files.push(generated(
            "README.md",
            include_str!("../../assets/defaults/rust/README.md"),
        ));
    }
    if template.defaults.gitignore {
        files.push(generated(
            ".gitignore",
            include_str!("../../assets/gitignore/rust.gitignore"),
        ));
    }
    if template.defaults.license {
        let license = template
            .license
            .as_ref()
            .context("defaults.license is true but [license] is missing")?;
        files.push(generated("LICENSE", license_text(&license.name)?));
    }
    if template.defaults.tests {
        files.push(generated(
            "tests/cli.rs",
            include_str!("../../assets/defaults/rust/test.rs"),
        ));
    }
    Ok(files)
}

fn generated(path: &str, content: &str) -> GeneratedFile {
    GeneratedFile {
        path: path.into(),
        content: content.to_owned(),
    }
}

fn install_dependencies(template: &ProjectTemplate, target: &Path) -> Result<()> {
    for dependency in &template.dependencies {
        run_cargo(target, &cargo_add_args(dependency, false))?;
    }
    for dependency in &template.dev_dependencies {
        run_cargo(target, &cargo_add_args(dependency, true))?;
    }
    Ok(())
}

pub(crate) fn cargo_add_args(dependency: &Dependency, development: bool) -> Vec<String> {
    let mut args = vec!["add".to_owned(), dependency.name.clone()];
    if development {
        args.push("--dev".to_owned());
    }
    if !dependency.features.is_empty() {
        args.extend(["--features".to_owned(), dependency.features.join(",")]);
    }
    if dependency.optional == Some(true) {
        args.push("--optional".to_owned());
    }
    if dependency.default_features == Some(false) {
        args.push("--no-default-features".to_owned());
    }
    args
}

fn run_cargo(target: &Path, args: &[String]) -> Result<()> {
    let output = Command::new("cargo")
        .args(args)
        .current_dir(target)
        .output()
        .with_context(|| format!("failed to start cargo {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "cargo {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn remove_cargo_default(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("could not remove '{}'", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dependency() -> Dependency {
        Dependency {
            name: "serde".to_owned(),
            features: Vec::new(),
            default_features: None,
            optional: None,
        }
    }

    #[test]
    fn cargo_args_for_normal_dependency() {
        assert_eq!(cargo_add_args(&dependency(), false), ["add", "serde"]);
    }

    #[test]
    fn cargo_args_for_dev_dependency() {
        assert_eq!(
            cargo_add_args(&dependency(), true),
            ["add", "serde", "--dev"]
        );
    }

    #[test]
    fn cargo_args_include_features() {
        let mut dependency = dependency();
        dependency.features = vec!["derive".to_owned(), "std".to_owned()];
        assert_eq!(
            cargo_add_args(&dependency, false),
            ["add", "serde", "--features", "derive,std"]
        );
    }

    #[test]
    fn cargo_args_include_optional() {
        let mut dependency = dependency();
        dependency.optional = Some(true);
        assert_eq!(
            cargo_add_args(&dependency, false),
            ["add", "serde", "--optional"]
        );
    }

    #[test]
    fn cargo_args_disable_default_features() {
        let mut dependency = dependency();
        dependency.default_features = Some(false);
        assert_eq!(
            cargo_add_args(&dependency, false),
            ["add", "serde", "--no-default-features"]
        );
    }
}
