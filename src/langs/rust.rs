use std::{fs, path::Path, process::Command};

pub fn create_project(path: &Path) -> Result<(), String> {
    if !path.exists() {
        if let Err(fehler) = fs::create_dir_all(&path) {
            println!("Error creating the project dir: {}", fehler);
        }
    }
    match Command::new("cargo")
        .arg("init")
        .arg("--bin")
        .current_dir(&path)
        .status()
    {
        Err(fehler) => Err(fehler.to_string()),
        Ok(_) => Ok(()),
    }
}
