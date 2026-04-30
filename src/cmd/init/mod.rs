use anyhow::{anyhow, Result};
use std::path::Path;

mod detect;
mod render;
mod wizard;

use crate::hooks_repo::{self, RecipeMeta};

const TEMPLATE: &str = include_str!("../../../templates/sessionx.yaml.tmpl");

#[derive(Default, Debug, Clone)]
pub struct InitOpts {
    pub yes: bool,
    pub force: bool,
    pub theme: Option<String>,
    pub mode: Option<String>,
    pub worktree: Option<String>,
}

pub fn run(opts: InitOpts) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let path = cwd.join(crate::config::CONFIG_FILENAME);

    if path.exists() {
        handle_existing(&path, &opts)?;
    }

    let any_flag = opts.theme.is_some() || opts.mode.is_some() || opts.worktree.is_some();
    let interactive = !opts.yes && crate::picker::is_tty() && !any_flag;

    let det = detect::detect(&cwd);

    let mut installed_recipe: Option<hooks_repo::Installed> = None;

    let resolved = if interactive {
        let Some(choices) = wizard::run(&opts, &det)? else {
            eprintln!("cancelled — no file written");
            return Ok(());
        };
        let windows_yaml = if choices.use_detected_layout {
            detect::windows_yaml_for(&det, &cwd)
        } else {
            None
        };
        let hook_commands = if let Some(recipe) = &choices.recipe {
            let i = hooks_repo::install_recipe(&recipe.id)?;
            let cmds = build_hook_commands(&i, &det);
            installed_recipe = Some(i);
            Some(cmds)
        } else {
            None
        };
        render::Resolved {
            mode: choices.mode,
            theme: choices.theme,
            worktree_dir: choices.worktree_dir,
            windows_yaml,
            hook_commands,
        }
    } else {
        // Non-interactive: --yes (no flags) auto-applies detection + auto-installs
        // a recipe if exactly one matches the stack. Flag-driven runs leave
        // recipes alone — explicit flags imply "I know what I want".
        if let Some(t) = &opts.theme {
            crate::themes::load(t)?;
        }
        let mode = opts.mode.clone().unwrap_or_else(|| "session".into());
        if mode != "session" && mode != "window" {
            return Err(anyhow!(
                "invalid --mode '{mode}' (expected 'session' or 'window')"
            ));
        }
        let auto_apply_detection = opts.yes && !any_flag;
        let windows_yaml = if auto_apply_detection {
            detect::windows_yaml_for(&det, &cwd)
        } else {
            None
        };
        let hook_commands = if auto_apply_detection {
            try_auto_install_recipe(&det, opts.worktree.is_some())?.map(|i| {
                let cmds = build_hook_commands(&i, &det);
                installed_recipe = Some(i);
                cmds
            })
        } else {
            None
        };
        render::Resolved {
            mode,
            theme: opts.theme.clone().unwrap_or_else(|| "tokyo-night".into()),
            worktree_dir: opts.worktree.clone(),
            windows_yaml,
            hook_commands,
        }
    };

    let rendered = render::apply(TEMPLATE, &resolved);
    std::fs::write(&path, &rendered)?;
    print_summary(
        &path,
        &cwd,
        &resolved,
        &det,
        interactive,
        installed_recipe.as_ref(),
    );
    Ok(())
}

/// Build the bash command strings to drop into post_create / pre_remove. Adds
/// `SX_LARAVEL_DIR=<subdir>` prefix for PHP recipes when the project lives in
/// a subdir (e.g. Laravel-in-www).
fn build_hook_commands(i: &hooks_repo::Installed, det: &detect::Detected) -> (String, String) {
    let mut prefix = String::new();
    if det.kind == detect::ProjectKind::Php {
        if let Some(sub) = &det.subdir {
            prefix = format!("SX_LARAVEL_DIR={sub} ");
        }
    }
    (
        format!("{prefix}bash {}", i.post_create_tilde()),
        format!("{prefix}bash {}", i.pre_remove_tilde()),
    )
}

/// In non-interactive `--yes` mode, install a recipe iff exactly one matches
/// the detected stack AND its worktree requirement is satisfied. Returns
/// Ok(None) on offline / no-match / multi-match / worktree-mismatch.
fn try_auto_install_recipe(
    det: &detect::Detected,
    worktree_on: bool,
) -> Result<Option<hooks_repo::Installed>> {
    let stack = match det.kind {
        detect::ProjectKind::Php => "php",
        detect::ProjectKind::Node => "node",
        detect::ProjectKind::Rust => "rust",
        detect::ProjectKind::Python => "python",
        detect::ProjectKind::Generic => return Ok(None),
    };
    let cache = match hooks_repo::ensure_cloned() {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    let m = match hooks_repo::read_manifest(&cache) {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };
    let matches: Vec<&RecipeMeta> = m
        .recipes
        .iter()
        .filter(|r| r.stack == stack && (!r.requires_worktree || worktree_on))
        .collect();
    if matches.len() != 1 {
        return Ok(None);
    }
    Ok(Some(hooks_repo::install_recipe(&matches[0].id)?))
}

fn handle_existing(path: &Path, opts: &InitOpts) -> Result<()> {
    if opts.force {
        backup(path)?;
        return Ok(());
    }
    if !crate::picker::is_tty() {
        return Err(anyhow!(
            "{} already exists (use --force to overwrite)",
            path.display()
        ));
    }
    let items = vec![
        "Overwrite (back up existing to .sessionx.yaml.bak)".to_string(),
        "Edit existing file in $EDITOR".to_string(),
        "Cancel".to_string(),
    ];
    let Some(idx) = crate::picker::select(&format!("{} exists", path.display()), &items)? else {
        return Err(anyhow!("cancelled"));
    };
    match idx {
        0 => backup(path),
        1 => {
            crate::cmd::edit::run()?;
            std::process::exit(0);
        }
        _ => Err(anyhow!("cancelled")),
    }
}

fn backup(path: &Path) -> Result<()> {
    let bak = path.with_extension("yaml.bak");
    std::fs::rename(path, &bak)?;
    eprintln!("backed up existing config to {}", bak.display());
    Ok(())
}

fn display_rel(p: &Path, base: &Path) -> String {
    p.strip_prefix(base)
        .map(|r| r.display().to_string())
        .unwrap_or_else(|_| p.display().to_string())
}

fn print_summary(
    path: &Path,
    cwd: &Path,
    r: &render::Resolved,
    det: &detect::Detected,
    interactive: bool,
    installed: Option<&hooks_repo::Installed>,
) {
    if !interactive {
        println!("wrote {}", display_rel(path, cwd));
        if let Some(i) = installed {
            println!("installed recipe '{}':", i.recipe.id);
            println!("  post_create  {}", i.post_create.display());
            println!("  pre_remove   {}", i.pre_remove.display());
        }
        return;
    }

    let project = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");

    let kind_label = match &det.subdir {
        Some(s) => format!("{} in ./{}/", det.kind.label(), s),
        None => det.kind.label().to_string(),
    };

    println!();
    println!("  ✓ wrote {}", display_rel(path, cwd));
    println!();
    println!("  project    {project}  ({kind_label})");
    println!("  mode       {}", r.mode);
    match &r.worktree_dir {
        Some(d) => println!("  worktree   {d}"),
        None => println!("  worktree   off"),
    }
    println!("  theme      {}", r.theme);
    if let Some(body) = &r.windows_yaml {
        let names: Vec<&str> = body
            .lines()
            .filter_map(|l| l.trim_start().strip_prefix("- name: "))
            .collect();
        if !names.is_empty() {
            println!("  windows    {}", names.join(", "));
        }
    }
    if let Some(i) = installed {
        println!("  recipe     {} ({})", i.recipe.id, i.recipe.stack);
        println!("    post_create  {}", i.post_create.display());
        println!("    pre_remove   {}", i.pre_remove.display());
        let missing = hooks_repo::missing_bins(&i.recipe);
        if !missing.is_empty() {
            println!("    ⚠ missing bins: {}", missing.join(", "));
        }
        if !i.recipe.secrets.is_empty() {
            println!(
                "    secrets needed in ~/.sessionx/secrets.env: {}",
                i.recipe.secrets.join(", ")
            );
        }
    }
    println!();
    println!("  next:");
    println!("    sessionx add work        # create session \"{project}-work\"");
    println!("    sessionx edit            # tweak .sessionx.yaml in $EDITOR");
    if r.theme != "tokyo-night" || matches!(det.kind, detect::ProjectKind::Generic) {
        println!("    sessionx theme preview   # try a different theme live");
    }
    println!();
}
