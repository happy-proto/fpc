use anyhow::{Context, Result, bail};
use clap::{CommandFactory, ValueEnum};
use clap_complete::engine::{
    ArgValueCompleter, CompletionCandidate, PathCompleter, ValueCompleter,
};
use clap_complete::{
    CompleteEnv,
    env::{Bash, EnvCompleter, Fish, Shells, Zsh},
};
use std::{
    collections::HashSet,
    env,
    ffi::{OsStr, OsString},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Shell {
    Fish,
    Bash,
    Zsh,
}

const START: &str = "# >>> fpc completion >>>";
const END: &str = "# <<< fpc completion <<<";

pub fn command() -> clap::Command {
    crate::Cli::command()
        .mut_arg("path", |arg| {
            arg.add(ArgValueCompleter::new(path_candidates))
        })
        .mut_arg("ignore_file", |arg| {
            arg.add(ArgValueCompleter::new(file_candidates))
        })
        .mut_arg("install_completion", |arg| arg.num_args(1))
}

pub fn complete() {
    CompleteEnv::with_factory(command)
        .shells(Shells(&[
            &ContextualCompleter(&Bash),
            &ContextualCompleter(&Fish),
            &ContextualCompleter(&Zsh),
        ]))
        .complete();
}

struct ContextualCompleter(&'static dyn EnvCompleter);

impl EnvCompleter for ContextualCompleter {
    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn is(&self, name: &str) -> bool {
        self.0.is(name)
    }

    fn write_registration(
        &self,
        var: &str,
        name: &str,
        bin: &str,
        completer: &str,
        buf: &mut dyn io::Write,
    ) -> io::Result<()> {
        if self.name() == "zsh" {
            let mut registration = Vec::new();
            self.0
                .write_registration(var, name, bin, completer, &mut registration)?;
            // Candidates are already matched by fpc. Zsh must not apply prefix matching again.
            let script = String::from_utf8(registration).map_err(io::Error::other)?;
            buf.write_all(
                script
                    .replace("'values' dirs", "'values' dirs -U")
                    .replace("'values' other", "'values' other -U")
                    .as_bytes(),
            )
        } else {
            self.0.write_registration(var, name, bin, completer, buf)
        }
    }

    fn write_complete(
        &self,
        cmd: &mut clap::Command,
        mut args: Vec<OsString>,
        current_dir: Option<&Path>,
        buf: &mut dyn io::Write,
    ) -> io::Result<()> {
        let index = if self.name() == "fish" {
            args.len().saturating_sub(1)
        } else {
            env::var("_CLAP_COMPLETE_INDEX")
                .ok()
                .and_then(|index| index.parse().ok())
                .unwrap_or_default()
        };
        if index == args.len() {
            args.push(OsString::new());
        }
        let current = args
            .get(index)
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        let complete_options = current.starts_with("--");
        let mut candidates =
            clap_complete::engine::complete(cmd, args.clone(), index, current_dir)?;
        let options: HashSet<_> = cmd
            .get_arguments()
            .flat_map(|arg| {
                arg.get_long_and_visible_aliases()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|name| format!("--{name}"))
                    .chain(
                        arg.get_short_and_visible_aliases()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|name| format!("-{name}")),
                    )
            })
            .collect();
        let is_option = |candidate: &CompletionCandidate| {
            candidate.get_id().is_some()
                && options.contains(candidate.get_value().to_string_lossy().as_ref())
        };
        if complete_options && candidates.iter().any(is_option) {
            candidates.retain(is_option);
        } else if !complete_options {
            candidates.retain(|candidate| !is_option(candidate));
        }
        // A positional-only second pass preserves candidates while reusing clap's shell formatting.
        let candidates = Mutex::new(candidates);
        let mut output = clap::Command::new("fpc")
            .disable_help_flag(true)
            .disable_version_flag(true)
            .arg(
                clap::Arg::new("candidates")
                    .num_args(0..)
                    .allow_hyphen_values(true)
                    .add(ArgValueCompleter::new(move |_: &OsStr| {
                        std::mem::take(&mut *candidates.lock().unwrap())
                    })),
            );
        self.0.write_complete(&mut output, args, current_dir, buf)
    }
}

fn file_candidates(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_path(current, PathCompleter::file())
}

fn path_candidates(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_path(current, PathCompleter::any())
}

fn complete_path(current: &OsStr, completer: PathCompleter) -> Vec<CompletionCandidate> {
    let path = Path::new(current);
    let trailing_separator = current
        .as_encoded_bytes()
        .last()
        .is_some_and(|byte| std::path::is_separator(*byte as char));
    if !trailing_separator && let Some(name) = path.file_name().and_then(OsStr::to_str) {
        // Enumerate only the directory being completed; match its final component ourselves.
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let mut directory = parent.to_path_buf();
        directory.push("");
        let query = name.to_lowercase();
        let mut prefixes = Vec::new();
        let mut substrings = Vec::new();
        for candidate in completer.complete(directory.as_os_str()) {
            let Some(name) = Path::new(candidate.get_value())
                .file_name()
                .and_then(OsStr::to_str)
            else {
                continue;
            };
            if name.starts_with('.') && !query.starts_with('.') {
                continue;
            }
            let name = name.to_lowercase();
            if name.starts_with(&query) {
                prefixes.push(candidate);
            } else if name.contains(&query) {
                substrings.push(candidate);
            }
        }
        return if prefixes.is_empty() {
            substrings
        } else {
            prefixes
        };
    }
    let explicit_dot = current == OsStr::new(".");
    completer
        .complete(if explicit_dot {
            OsStr::new("")
        } else {
            current
        })
        .into_iter()
        .filter(|candidate| candidate.get_value() != OsStr::new("."))
        .filter(|candidate| {
            !candidate.is_hide_set()
                || explicit_dot
                || path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        })
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

pub fn install(shell: Option<Shell>) -> Result<()> {
    let shell = if let Some(shell) = shell {
        shell
    } else {
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
