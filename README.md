# entre

## Projekt

`entre` ist ein Rust-CLI zum Erzeugen neuer Projekte aus sprachspezifischen TOML-Templates. Das Standard-Rust-Projekt enthält Quellcode, README, MIT-Lizenz, `.gitignore` und Release-Optimierungen.

## Verwendung

```bash
# Standard-Rust-Projekt
entre rust ./mein-projekt

# Benanntes Template
entre rust cli_basic ./mein-projekt
```

Der Zielordner muss neu oder leer sein.

## Installation

Rust und Cargo müssen installiert sein. Im Repository ausführen:

```bash
cargo install --path .
```

Danach steht `entre` im `PATH` bereit.
