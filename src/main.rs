// Rust guideline compliant 2026-02-21

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod langs;
mod templates;

const DEFAULT_RUST_TEMPLATE: &str = "default";

#[derive(Parser, Debug)]
#[command(
    name = "entre",
    about = "Create projects from language-specific TOML templates.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Language,
}

#[derive(Subcommand, Debug)]
enum Language {
    /// Initialize a Rust project, optionally from a named template.
    Rust {
        /// Target path, or template name when TARGET_PATH follows.
        #[arg(value_name = "TEMPLATE_OR_TARGET")]
        template_or_target: PathBuf,
        /// Target path when a template name was supplied.
        #[arg(value_name = "TARGET_PATH")]
        target_path: Option<PathBuf>,
    },
    /// Initialize a Python project.
    Python {
        /// Template name below templates/python/.
        template: String,
        /// New project directory.
        target_path: PathBuf,
    },
}

fn rust_arguments(
    template_or_target: PathBuf,
    target_path: Option<PathBuf>,
) -> Result<(String, PathBuf)> {
    match target_path {
        Some(target_path) => {
            let template = template_or_target
                .to_str()
                .context("Rust template name must be valid UTF-8")?;
            Ok((template.to_owned(), target_path))
        }
        None => Ok((DEFAULT_RUST_TEMPLATE.to_owned(), template_or_target)),
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Language::Rust {
            template_or_target,
            target_path,
        } => {
            let (template, target_path) = rust_arguments(template_or_target, target_path)?;
            langs::rust::create_project(&target_path, &template)
        }
        Language::Python {
            template,
            target_path,
        } => langs::python::create_project(&target_path, &template),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parsed_rust(args: &[&str]) -> Result<(String, PathBuf)> {
        let cli = Cli::try_parse_from(args.iter().copied())?;
        let Language::Rust {
            template_or_target,
            target_path,
        } = cli.command
        else {
            anyhow::bail!("expected Rust command");
        };
        rust_arguments(template_or_target, target_path)
    }

    #[test]
    fn rust_target_uses_default_template() -> Result<()> {
        let (template, target) = parsed_rust(&["entre", "rust", "project"])?;
        assert_eq!(template, DEFAULT_RUST_TEMPLATE);
        assert_eq!(target, Path::new("project"));
        Ok(())
    }

    #[test]
    fn rust_template_and_target_are_preserved() -> Result<()> {
        let (template, target) = parsed_rust(&["entre", "rust", "cli_basic", "project"])?;
        assert_eq!(template, "cli_basic");
        assert_eq!(target, Path::new("project"));
        Ok(())
    }
}
