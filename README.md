# entre

`entre rust <target-path>` creates the built-in Rust project with MIT license, Rust `.gitignore`, and release optimizations.

`entre <language> <template> <target-path>` creates a project from `templates/<language>/<template>/project.toml`.

## Template architecture

- `src/templates.rs`: strict TOML model, loader, validation, rendering, and safe file writes.
- `src/langs/rust.rs`: Cargo initialization, Rust defaults, dependency installation, and formatting.
- `templates/<language>/<template>/files/*.toml`: deterministic custom files overriding defaults by target path.
- `assets/`: embedded default files, `.gitignore` presets, and license texts.

Manifests are package-manager managed. Templates cannot define commands. Python and custom manifests currently return explicit unsupported errors. Existing symlink path components are rejected; concurrent filesystem replacement attacks are outside the current local-CLI threat model.
