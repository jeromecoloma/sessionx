use anyhow::{anyhow, Result};
use std::path::Path;

const TEMPLATE: &str = include_str!("../../templates/sessionx.yaml.tmpl");

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let path = cwd.join(crate::config::CONFIG_FILENAME);
    if path.exists() {
        return Err(anyhow!("{} already exists", path.display()));
    }
    std::fs::write(&path, TEMPLATE)?;
    println!("wrote {}", display_rel(&path, &cwd));
    Ok(())
}

fn display_rel(p: &Path, base: &Path) -> String {
    p.strip_prefix(base)
        .map(|r| r.display().to_string())
        .unwrap_or_else(|_| p.display().to_string())
}
