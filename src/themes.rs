use anyhow::{anyhow, Result};
use std::collections::BTreeMap;

/// A preset theme. Fields mirror StatusSpec; None means "don't impose a default".
pub struct Theme {
    pub style: BTreeMap<String, String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub window_format: Option<String>,
    pub current_window_format: Option<String>,
    pub status_interval: Option<u32>,
}

pub fn list() -> &'static [&'static str] {
    &[
        "tokyo-night",
        "catppuccin",
        "dracula",
        "gruvbox",
        "nord",
        "rose-pine",
        "minimal",
    ]
}

pub fn load(name: &str) -> Result<Theme> {
    match name {
        "tokyo-night" => Ok(tokyo_night()),
        "catppuccin" => Ok(catppuccin()),
        "dracula" => Ok(dracula()),
        "gruvbox" => Ok(gruvbox()),
        "nord" => Ok(nord()),
        "rose-pine" => Ok(rose_pine()),
        "minimal" => Ok(minimal()),
        other => Err(anyhow!(
            "unknown theme '{other}'. Available: {}",
            list().join(", ")
        )),
    }
}

fn s(map: &mut BTreeMap<String, String>, k: &str, v: &str) {
    map.insert(k.to_string(), v.to_string());
}

/// Tokyo Night — purple icon block, host badge, prefix-aware session label, date on right.
fn tokyo_night() -> Theme {
    let mut style = BTreeMap::new();
    s(&mut style, "status-style", "bg=#1a1b26,fg=#c0caf5");
    s(&mut style, "window-status-style", "bg=#1a1b26,fg=#7aa2f7");
    s(
        &mut style,
        "window-status-current-style",
        "bg=#1a1b26,fg=#bb9af7,bold",
    );
    s(&mut style, "pane-active-border-style", "fg=#7aa2f7");
    s(&mut style, "pane-border-style", "fg=#414868");
    s(&mut style, "message-style", "bg=#bb9af7,fg=#1a1b26,bold");

    Theme {
        style,
        left: Some(
            "#[fg=#c0caf5,bg=#9d7cd8,bold] ⏳ \
             #[fg=#a9b1d6,bg=#3d59a1,nobold] #h \
             #[fg=#c0caf5,bg=#1a2a4a,bold] #{?client_prefix,󰠠 ,#[dim]󰤂 }#[bold,nodim]#S \
             #[fg=#1a2a4a,bg=#1a1b26,nobold]"
                .into(),
        ),
        right: Some(
            "#[fg=#414868,bg=#1a1b26]\
             #[fg=#c0caf5,bg=#414868] %m-%d-%Y \
             #[fg=#7aa2f7]< \
             #[fg=#c0caf5]%I:%M %p "
                .into(),
        ),
        window_format: Some(" #[fg=#7aa2f7]#I#[fg=#414868]:#[fg=#c0caf5]#W ".into()),
        current_window_format: Some(
            " #[fg=#bb9af7,bold]#I#[fg=#414868]:#[fg=#c0caf5,bold]#W ".into(),
        ),
        status_interval: Some(5),
    }
}

/// Catppuccin Mocha
fn catppuccin() -> Theme {
    let mut style = BTreeMap::new();
    s(&mut style, "status-style", "bg=#1e1e2e,fg=#cdd6f4");
    s(&mut style, "window-status-style", "bg=#1e1e2e,fg=#9399b2");
    s(
        &mut style,
        "window-status-current-style",
        "bg=#1e1e2e,fg=#89b4fa,bold",
    );
    s(&mut style, "pane-active-border-style", "fg=#89b4fa");
    s(&mut style, "pane-border-style", "fg=#45475a");
    s(&mut style, "message-style", "bg=#89b4fa,fg=#1e1e2e,bold");

    Theme {
        style,
        left: Some(
            "#[fg=#1e1e2e,bg=#cba6f7,bold] ◆ \
             #[fg=#cdd6f4,bg=#45475a,nobold] #h \
             #[fg=#cdd6f4,bg=#313244] #{?client_prefix,● ,○ }#S \
             #[fg=#313244,bg=#1e1e2e]"
                .into(),
        ),
        right: Some(
            "#[fg=#45475a,bg=#1e1e2e]\
             #[fg=#cdd6f4,bg=#45475a] %m-%d-%Y \
             #[fg=#1e1e2e,bg=#89b4fa,bold] %I:%M %p "
                .into(),
        ),
        window_format: Some(" #I:#W ".into()),
        current_window_format: Some(" #[bold]#I:#W#[nobold] ".into()),
        status_interval: Some(5),
    }
}

/// Dracula
fn dracula() -> Theme {
    let mut style = BTreeMap::new();
    s(&mut style, "status-style", "bg=#282a36,fg=#f8f8f2");
    s(&mut style, "window-status-style", "bg=#282a36,fg=#6272a4");
    s(
        &mut style,
        "window-status-current-style",
        "bg=#282a36,fg=#bd93f9,bold",
    );
    s(&mut style, "pane-active-border-style", "fg=#bd93f9");
    s(&mut style, "pane-border-style", "fg=#44475a");
    s(&mut style, "message-style", "bg=#ff79c6,fg=#282a36,bold");

    Theme {
        style,
        left: Some(
            "#[fg=#282a36,bg=#bd93f9,bold] ☾ \
             #[fg=#f8f8f2,bg=#44475a,nobold] #h \
             #[fg=#f8f8f2,bg=#282a36] #{?client_prefix,● ,○ }#S "
                .into(),
        ),
        right: Some(
            "#[fg=#44475a,bg=#282a36]\
             #[fg=#f8f8f2,bg=#44475a] %m-%d-%Y \
             #[fg=#282a36,bg=#ff79c6,bold] %I:%M %p "
                .into(),
        ),
        window_format: Some(" #I:#W ".into()),
        current_window_format: Some(" #[bold]#I:#W#[nobold] ".into()),
        status_interval: Some(5),
    }
}

/// Gruvbox Dark
fn gruvbox() -> Theme {
    let mut style = BTreeMap::new();
    s(&mut style, "status-style", "bg=#282828,fg=#ebdbb2");
    s(&mut style, "window-status-style", "bg=#282828,fg=#928374");
    s(
        &mut style,
        "window-status-current-style",
        "bg=#282828,fg=#fabd2f,bold",
    );
    s(&mut style, "pane-active-border-style", "fg=#fabd2f");
    s(&mut style, "pane-border-style", "fg=#504945");
    s(&mut style, "message-style", "bg=#fabd2f,fg=#282828,bold");

    Theme {
        style,
        left: Some(
            "#[fg=#282828,bg=#fabd2f,bold]  \
             #[fg=#ebdbb2,bg=#504945,nobold] #h \
             #[fg=#ebdbb2,bg=#282828] #{?client_prefix,● ,○ }#S "
                .into(),
        ),
        right: Some(
            "#[fg=#504945,bg=#282828]\
             #[fg=#ebdbb2,bg=#504945] %m-%d-%Y \
             #[fg=#282828,bg=#fabd2f,bold] %I:%M %p "
                .into(),
        ),
        window_format: Some(" #I:#W ".into()),
        current_window_format: Some(" #[bold]#I:#W#[nobold] ".into()),
        status_interval: Some(5),
    }
}

/// Nord
fn nord() -> Theme {
    let mut style = BTreeMap::new();
    s(&mut style, "status-style", "bg=#2e3440,fg=#d8dee9");
    s(&mut style, "window-status-style", "bg=#2e3440,fg=#4c566a");
    s(
        &mut style,
        "window-status-current-style",
        "bg=#2e3440,fg=#88c0d0,bold",
    );
    s(&mut style, "pane-active-border-style", "fg=#88c0d0");
    s(&mut style, "pane-border-style", "fg=#3b4252");
    s(&mut style, "message-style", "bg=#88c0d0,fg=#2e3440,bold");

    Theme {
        style,
        left: Some(
            "#[fg=#2e3440,bg=#88c0d0,bold] ❄ \
             #[fg=#d8dee9,bg=#3b4252,nobold] #h \
             #[fg=#d8dee9,bg=#2e3440] #{?client_prefix,● ,○ }#S "
                .into(),
        ),
        right: Some(
            "#[fg=#3b4252,bg=#2e3440]\
             #[fg=#d8dee9,bg=#3b4252] %m-%d-%Y \
             #[fg=#2e3440,bg=#88c0d0,bold] %I:%M %p "
                .into(),
        ),
        window_format: Some(" #I:#W ".into()),
        current_window_format: Some(" #[bold]#I:#W#[nobold] ".into()),
        status_interval: Some(5),
    }
}

/// Rose Pine
fn rose_pine() -> Theme {
    let mut style = BTreeMap::new();
    s(&mut style, "status-style", "bg=#191724,fg=#e0def4");
    s(&mut style, "window-status-style", "bg=#191724,fg=#6e6a86");
    s(
        &mut style,
        "window-status-current-style",
        "bg=#191724,fg=#ebbcba,bold",
    );
    s(&mut style, "pane-active-border-style", "fg=#ebbcba");
    s(&mut style, "pane-border-style", "fg=#26233a");
    s(&mut style, "message-style", "bg=#ebbcba,fg=#191724,bold");

    Theme {
        style,
        left: Some(
            "#[fg=#191724,bg=#ebbcba,bold] ✦ \
             #[fg=#e0def4,bg=#26233a,nobold] #h \
             #[fg=#e0def4,bg=#191724] #{?client_prefix,● ,○ }#S "
                .into(),
        ),
        right: Some(
            "#[fg=#26233a,bg=#191724]\
             #[fg=#e0def4,bg=#26233a] %m-%d-%Y \
             #[fg=#191724,bg=#ebbcba,bold] %I:%M %p "
                .into(),
        ),
        window_format: Some(" #I:#W ".into()),
        current_window_format: Some(" #[bold]#I:#W#[nobold] ".into()),
        status_interval: Some(5),
    }
}

/// Minimal — uncluttered, neutral colors. Good base to override.
fn minimal() -> Theme {
    let mut style = BTreeMap::new();
    s(&mut style, "status-style", "bg=default,fg=default");
    s(&mut style, "window-status-current-style", "fg=default,bold");
    s(&mut style, "pane-active-border-style", "fg=blue");
    s(&mut style, "pane-border-style", "fg=default");

    Theme {
        style,
        left: Some(" #{?client_prefix,● ,○ }#S ".into()),
        right: Some(" %m-%d %H:%M ".into()),
        window_format: Some(" #I:#W ".into()),
        current_window_format: Some(" #[bold]#I:#W#[nobold] ".into()),
        status_interval: Some(15),
    }
}
