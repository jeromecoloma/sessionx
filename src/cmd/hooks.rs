use anyhow::Result;

use crate::hooks_repo;

pub fn run_list() -> Result<()> {
    let cache = hooks_repo::ensure_cloned()?;
    let m = hooks_repo::read_manifest(&cache)?;
    println!("repo:  {}", hooks_repo::repo_url());
    println!("ref:   {}", hooks_repo::pinned_ref());
    println!("cache: {}", cache.display());
    println!();
    println!("recipes:");
    for r in &m.recipes {
        println!("  {:<16} [{}]  {}", r.id, r.stack, r.description);
    }
    Ok(())
}

pub fn run_info(id: &str) -> Result<()> {
    let cache = hooks_repo::ensure_cloned()?;
    let m = hooks_repo::read_manifest(&cache)?;
    let r = hooks_repo::find_recipe(&m, id)?;
    println!("id:           {}", r.id);
    println!("stack:        {}", r.stack);
    println!("description:  {}", r.description);
    println!("post_create:  {}", r.post_create);
    println!("pre_remove:   {}", r.pre_remove);
    if !r.required_env.is_empty() {
        println!("required_env: {}", r.required_env.join(", "));
    }
    if !r.optional_env.is_empty() {
        println!("optional_env: {}", r.optional_env.join(", "));
    }
    if !r.secrets.is_empty() {
        println!("secrets:      {}", r.secrets.join(", "));
    }
    if !r.requires_bins.is_empty() {
        println!("requires:     {}", r.requires_bins.join(", "));
        let missing = hooks_repo::missing_bins(r);
        if !missing.is_empty() {
            println!("missing bins: {}", missing.join(", "));
        }
    }
    Ok(())
}

pub fn run_install(id: &str) -> Result<()> {
    let installed = hooks_repo::install_recipe(id)?;
    println!("installed {}:", installed.recipe.id);
    println!("  post_create  {}", installed.post_create.display());
    println!("  pre_remove   {}", installed.pre_remove.display());
    let missing = hooks_repo::missing_bins(&installed.recipe);
    if !missing.is_empty() {
        eprintln!(
            "warning: missing required bins: {} — install before running hooks",
            missing.join(", ")
        );
    }
    Ok(())
}

pub fn run_update() -> Result<()> {
    let dir = hooks_repo::ensure_cloned()?;
    println!(
        "up to date: {} @ {}",
        dir.display(),
        hooks_repo::pinned_ref()
    );
    Ok(())
}

pub fn run_repo() -> Result<()> {
    println!("repo:  {}", hooks_repo::repo_url());
    println!("ref:   {}", hooks_repo::pinned_ref());
    println!("cache: {}", hooks_repo::cache_dir()?.display());
    Ok(())
}
