use clap::Parser;
use std::path::{PathBuf};

mod langs;

#[derive(Parser, Debug)]
#[command(about, version)]
struct Args {
    /// The language of the project to initialize.
    lang: String,

    /// The Path where to put the projekt
    path: PathBuf,
}

fn main() {
    let args: Args = Args::parse();
    match args.lang.to_lowercase().as_str() {
        "python" => langs::python::create_project(),
        "rust" => match langs::rust::create_project(args.path.as_path()) {
            Ok(_) => {}
            Err(fehler) => eprintln!("Error creating Rust project: {}", fehler),
        },
        _ => {
            eprintln!("Invalid language: {}", args.lang);
            return;
        }
    };
}
