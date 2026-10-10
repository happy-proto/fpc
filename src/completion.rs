use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use clap_complete::engine::{
    ArgValueCompleter, CompletionCandidate, PathCompleter, ValueCompleter,
};
use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Shell {
    Auto,
    Fish,
    Bash,
    Zsh,
}

const START: &str = "# >>> fpc completion >>>";
const END: &str = "# <<< fpc completion <<<";

pub fn command() -> clap::Command {
    clap::Command::new("fpc")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            clap::Arg::new("path")
                .num_args(0..)
                .allow_hyphen_values(true)
                .add(ArgValueCompleter::new(path_candidates)),
        )
}

fn path_candidates(current: &OsStr) -> Vec<CompletionCandidate> {
    let explicit_dot = current == OsStr::new(".");
    PathCompleter::any()
        .complete(if explicit_dot {
            OsStr::new("")
        } else {
            current
        })
        .into_iter()
        .filter(|candidate| candidate.get_value() != OsStr::new("."))
        .filter(|candidate| {
            !explicit_dot || candidate.get_value().to_string_lossy().starts_with('.')
        })
        .collect()
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn bash_completion_available() -> bool {
    if let Ok(output) = Command::new("pkg-config")
        .args(["--variable=datadir", "bash-completion"])
        .output()
        && output.status.success()
        && let Ok(dir) = std::str::from_utf8(&output.stdout)
        && !dir.trim().is_empty()
        && Path::new(dir.trim())
            .join("bash-completion/bash_completion")
            .is_file()
    {
        return true;
    }
    let dirs = env::var_os("XDG_DATA_DIRS")
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share:/opt/homebrew/share".into());
    env::split_paths(&dirs).any(|dir| dir.join("bash-completion/bash_completion").is_file())
}

pub fn install(shell: Shell) -> Result<()> {
    let shell = if matches!(shell, Shell::Auto) {
        match env::var_os("SHELL")
            .as_deref()
            .and_then(|value| Path::new(value).file_name())
            .and_then(|s| s.to_str())
        {
            Some("fish") => Shell::Fish,
            Some("bash") => Shell::Bash,
            Some("zsh") => Shell::Zsh,
            _ => bail!("Cannot detect shell; use --install-completion fish, bash, or zsh"),
        }
    } else {
        shell
    };
    let home = env_path("HOME").context("HOME is missing")?;
    let data = env_path("XDG_DATA_HOME").unwrap_or_else(|| home.join(".local/share"));
    let (path, hook) = match shell {
        Shell::Fish => (
            env_path("XDG_CONFIG_HOME")
                .unwrap_or_else(|| home.join(".config"))
                .join("fish/completions/fpc.fish"),
            "if command -q fpc\n    COMPLETE=fish fpc | source\nend",
        ),
        Shell::Bash => {
            if !bash_completion_available() {
                bail!(
                    "Cannot detect bash-completion. Install and enable bash-completion first; for a custom installation, expose its pkg-config metadata or add its share directory to XDG_DATA_DIRS. No files were changed."
                );
            }
            let dir = env::var_os("BASH_COMPLETION_USER_DIR")
                .filter(|v| !v.is_empty())
                .and_then(|v| env::split_paths(&v).find(|p| !p.as_os_str().is_empty()))
                .unwrap_or_else(|| data.join("bash-completion"));
            (
                dir.join("completions/fpc.bash"),
                "if command -v fpc >/dev/null 2>&1; then\n    eval \"$(COMPLETE=bash fpc)\"\n    _fpc_completion_registration=$(complete -p fpc)\n    eval \"${_fpc_completion_registration/#complete /complete -o filenames }\"\n    unset _fpc_completion_registration\nfi",
            )
        }
        Shell::Zsh => (
            data.join("zsh/site-functions/_fpc"),
            "if command -v fpc >/dev/null 2>&1; then\n    eval \"$(COMPLETE=zsh fpc)\"\n    if [[ ${_comps[fpc]} != _fpc && -n ${_comps[fpc]} ]]; then\n        \"${_comps[fpc]}\" \"$@\"\n    fi\nfi",
        ),
        Shell::Auto => unreachable!(),
    };
    let old = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).with_context(|| format!("Cannot read {}", path.display())),
    };
    let header = if matches!(shell, Shell::Zsh) {
        "#compdef fpc\n"
    } else {
        ""
    };
    if !old.is_empty() && !old.starts_with(&format!("{header}{START}\n")) {
        bail!(
            "{} already contains an unrelated completion; it was left untouched",
            path.display()
        );
    }
    let new = format!("{header}{START}\n{hook}\n{END}\n");
    if new != old {
        fs::create_dir_all(path.parent().context("No parent directory")?)?;
        fs::write(&path, new).with_context(|| format!("Cannot write {}", path.display()))?;
    }
    eprintln!(
        "Installed completion in:\n{}\n\nMake sure fpc is on PATH. Shell startup files were left untouched.",
        path.display()
    );
    match shell {
        Shell::Bash => {
            eprintln!("Enable bash-completion in your shell, then open a new Bash session.")
        }
        Shell::Zsh => {
            eprintln!(
                "Add the following directory to fpath before running compinit, then open a new Zsh session:"
            );
            eprintln!("{}", path.parent().unwrap().display());
        }
        _ => eprintln!("Open a new Fish session to load the completion."),
    }
    Ok(())
}
