use anyhow::Result;

pub fn run() -> Result<()> {
    let loaded = crate::config::find_and_load()?;
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new(&editor)
        .arg(&loaded.config_path)
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("editor {editor} exited non-zero"));
    }
    Ok(())
}
