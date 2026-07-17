// Rust guideline compliant 2026-02-21

use anyhow::{Result, bail};
use std::path::Path;

pub(crate) fn create_project(_target: &Path, _template_name: &str) -> Result<()> {
    bail!("Python project generation is not supported yet; managed uv support is planned")
}
