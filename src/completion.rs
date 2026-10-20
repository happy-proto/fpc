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
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Shell {
    Fish,
    Bash,
    Zsh,
}

const START: &str = "# >>> fpc completion >>>";
const END: &str = "# <<< fpc completion <<<";

// Shell adapters exchange NUL-delimited records so file names remain data, including newlines.
pub fn interact() -> Result<()> {
    use std::io::Write;

    let args: Vec<_> = env::args_os().skip(2).collect();
    let index = env::var("_CLAP_COMPLETE_INDEX")
        .context("Missing completion cursor index")?
        .parse::<usize>()?;
    let request = Arc::new(Mutex::new(None));
    let mut cmd = command();
    for id in ["path", "ignore_file"] {
        let request = Arc::clone(&request);
        cmd = cmd.mut_arg(id, |arg| {
            arg.add(ArgValueCompleter::new(move |value: &OsStr| {
                *request.lock().unwrap() = Some(value.to_owned());
                Vec::<CompletionCandidate>::new()
            }))
        });
    }
    let cwd = env::current_dir()?;
    let completing_options = args
        .get(index)
        .is_some_and(|value| value.as_encoded_bytes().starts_with(b"--"));
    let inline_value = args
        .get(index)
        .is_some_and(|value| value.as_encoded_bytes().starts_with(b"--ignore-file="));
    let candidates = clap_complete::engine::complete(&mut cmd, args, index, Some(&cwd))?;
    let current = request.lock().unwrap().take();
    let mut out = io::stdout().lock();
    let Some(current) = current.filter(|_| !completing_options || candidates.is_empty()) else {
        out.write_all(b"fallback\0")?;
        return Ok(());
    };
    let path = Path::new(&current);
    let text = current.to_string_lossy();
    let (directory, query) = if text == "." || text.ends_with("/.") {
        (
            PathBuf::from(text.strip_suffix('.').unwrap()),
            ".".to_owned(),
        )
    } else if !text.ends_with('/')
        && let Some(name) = path.file_name()
    {
        (
            path.parent().unwrap_or_else(|| Path::new("")).to_owned(),
            name.to_string_lossy().into_owned(),
        )
    } else {
        (path.to_owned(), String::new())
    };
    let mut prefix = directory.clone();
    prefix.push("");
    let entries = PathCompleter::any()
        .complete(prefix.as_os_str())
        .into_iter()
        .filter(|candidate| candidate.get_value() != OsStr::new("."))
        .map(|candidate| {
            let path = PathBuf::from(candidate.get_value());
            let directory_entry = candidate.get_value().as_encoded_bytes().ends_with(b"/");
            crate::paths::Candidate::new(path, &directory, directory_entry)
        })
        .collect();
    match crate::picker::complete(entries, &query)? {
        Some(path) => {
            let path = if let Ok(suffix) = path.strip_prefix("~") {
                env::home_dir()
                    .context("Cannot read home directory")?
                    .join(suffix)
            } else {
                path
            };
            out.write_all(b"selected\0")?;
            if inline_value {
                out.write_all(b"--ignore-file=")?;
            }
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;
                out.write_all(path.as_os_str().as_bytes())?;
            }
            #[cfg(not(unix))]
            out.write_all(path.to_str().context("Path is not Unicode")?.as_bytes())?;
            out.write_all(b"\0")?;
        }
        None => out.write_all(b"cancelled\0")?,
    }
    Ok(())
}

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
        self.0.write_registration(var, name, bin, completer, buf)?;
        let script = match self.name() {
            "bash" => include_str!("completions/bash.bash"),
            "fish" => include_str!("completions/fish.fish"),
            "zsh" => include_str!("completions/zsh.zsh"),
            _ => unreachable!(),
        };
        buf.write_all(script.as_bytes())
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
    let header = if matches!(shell, Shell::Zsh) {
        "#compdef fpc\n"
    } else {
        ""
    };
    let mut files = vec![prepare_install(path, hook, header)?];
    if matches!(shell, Shell::Fish) {
        files.push(prepare_install(
            env_path("XDG_CONFIG_HOME")
                .unwrap_or_else(|| home.join(".config"))
                .join("fish/conf.d/fpc.fish"),
            "if status is-interactive; and command -q fpc\n    COMPLETE=fish fpc | source\nend",
            "",
        )?);
    }
    for (path, content) in &files {
        fs::create_dir_all(path.parent().context("No parent directory")?)?;
        fs::write(path, content).with_context(|| format!("Cannot write {}", path.display()))?;
    }
    eprintln!("Installed completion in:");
    for (path, _) in &files {
        eprintln!("{}", path.display());
    }
    eprintln!("\nMake sure fpc is on PATH. Existing shell startup files were left untouched.");
    match shell {
        Shell::Bash => {
            eprintln!("Enable bash-completion in your shell, then open a new Bash session.")
        }
        Shell::Zsh => {
            eprintln!(
                "Add the following directory to fpath before running compinit, then open a new Zsh session:"
            );
            eprintln!("{}", files[0].0.parent().unwrap().display());
        }
        _ => eprintln!("Open a new Fish session to load the completion."),
    }
    Ok(())
}

fn prepare_install(path: PathBuf, hook: &str, header: &str) -> Result<(PathBuf, String)> {
    let old = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).with_context(|| format!("Cannot read {}", path.display())),
    };
    if !old.is_empty() && !old.starts_with(&format!("{header}{START}\n")) {
        bail!(
            "{} already contains an unrelated completion; it was left untouched",
            path.display()
        );
    }
    let new = format!("{header}{START}\n{hook}\n{END}\n");
    Ok((path, new))
}
