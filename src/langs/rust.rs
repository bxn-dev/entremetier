use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    process::Command,
};

pub fn create_project(path: &Path, template: Option<&str>) -> Result<(), String> {
    let templates: HashMap<&str, String> = HashMap::from([
        (
            "base_cli",
            String::from("https://github.com/bxn-dev/rust_cli_basic_template.git"),
        ),
        ("base_lib", String::from("")),
    ]);

    if let Some(template) = template {
        match templates.get(template) {
            Some(template_url) => create_template(&path, template_url),
            None => return Err("Template not found".to_string()),
        }
    } else {
        base(&path)
    }
}

fn base(path: &Path) -> Result<(), String> {
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
        Err(err) => Err(err.to_string()),
        Ok(_) => Ok(()),
    }
}

fn create_template(path: &Path, template: &str) -> Result<(), String> {
    match Command::new("git")
        .arg("clone")
        .arg(template)
        .arg(&path)
        .arg("--depth")
        .arg("1")
        .status()
    {
        Err(err) => return Err(err.to_string()),
        Ok(_) => println!("Template cloned"),
    }

    match personalize(path) {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

fn personalize(path: &Path) -> Result<(), String> {
    let toml_path = path.join("Cargo.toml");
    let project_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Could not extract project name from path".to_string())?;

    let toml_content =
        fs::read_to_string(&toml_path).expect("Toml File is cloned, so it should just open.");
    let mut toml_file =
        File::create(toml_path).expect("If above did not panic, this shouldnt aswell.");
    let mut writer = BufWriter::new(&mut toml_file);

    for line in toml_content.lines() {
        if line.contains("name =") {
            writeln!(writer, "name = \"{}\"", project_name).map_err(|e| e.to_string())?;
        } else {
            writeln!(writer, "{}", line).map_err(|e| e.to_string())?;
        }
    }

    match writer.flush() {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
