use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

pub fn directory_relative_to_manifest(given_path: &str) -> Result<PathBuf, Box<dyn Error>> {
    let cargo_toml_directory = env::var("CARGO_MANIFEST_DIR")?;
    let cargo_manifest_path = Path::new(&cargo_toml_directory);
    let path = Path::new(given_path);
    Ok(cargo_manifest_path.join(path))
}
