mod clipboard;
mod completion;
mod paths;
mod picker;

use anyhow::{Context, Result};
use clap::{Parser, ValueHint};
use clap_complete::CompleteEnv;
use std::{
    io::{self, Write},
    path::PathBuf,
};

shadow_rs::shadow!(build);

const COMMIT: &str = if build::COMMIT_HASH.is_empty() {
    "unknown"
} else {
    build::COMMIT_HASH
};
const DIRTY: &str = if build::COMMIT_HASH.is_empty() || build::GIT_CLEAN {
    ""
} else {
    " (dirty)"
};
const LONG_VERSION: &str = shadow_rs::formatcp!(
    "{}\nCommit: {}{}\nBuilt: {}\nTarget: {}\nRust: {}\n\n{}",
    build::PKG_VERSION,
    COMMIT,
    DIRTY,
    build::BUILD_TIME_3339,
    build::BUILD_TARGET,
    build::RUST_VERSION,
    env!("CARGO_PKG_REPOSITORY")
);

#[derive(Debug, Parser)]
#[command(
    name = "fpc",
    version,
    long_version = LONG_VERSION,
    about = "Pick a file or directory and copy its absolute path",
    after_help = concat!(
        "Use --version for commit, build time, target, and compiler details.\n\n",
        env!("CARGO_PKG_REPOSITORY")
    )
)]
struct Cli {
    /// File or directory to copy; omit to open the fuzzy picker
    #[arg(value_hint = ValueHint::AnyPath)]
    path: Option<PathBuf>,
    /// Respect .gitignore, .ignore, and Git exclude rules in the picker
    #[arg(long)]
    respect_ignore: bool,
    /// Apply an additional ignore file using Git ignore syntax
    #[arg(long, value_hint = ValueHint::FilePath)]
    ignore_file: Vec<PathBuf>,
    /// Print the absolute path instead of copying it
    #[arg(long)]
    print: bool,
    /// Install dynamic completion (auto, fish, bash, or zsh)
    #[arg(long, num_args = 0..=1, default_missing_value = "auto", value_enum,
        conflicts_with_all = ["path", "respect_ignore", "ignore_file", "print"])]
    install_completion: Option<completion::Shell>,
}

fn run() -> Result<()> {
    CompleteEnv::with_factory(completion::command).complete();
    let cli = Cli::parse();
    if let Some(shell) = cli.install_completion {
        return completion::install(shell);
    }
    let path = match cli.path {
        Some(path) => path,
        None => {
            let root = std::env::current_dir().context("Cannot read current directory")?;
            let walker = paths::walker(&root, cli.respect_ignore, &cli.ignore_file)?;
            let Some(path) = picker::pick(walker, &root)? else {
                return Ok(());
            };
            path
        }
    };
    std::fs::symlink_metadata(&path)
        .with_context(|| format!("Cannot access {}", path.display()))?;
    let path = std::path::absolute(&path)
        .with_context(|| format!("Cannot make {} absolute", path.display()))?;
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes()
    };
    #[cfg(not(unix))]
    let bytes = path.to_str().context("Path is not Unicode")?.as_bytes();
    if !cli.print {
        match clipboard::copy(bytes) {
            Ok(()) => return Ok(()),
            Err(error) => {
                eprintln!("Clipboard unavailable: {error:#}\nPath printed for manual copying.")
            }
        }
    }
    let mut out = io::stdout().lock();
    out.write_all(bytes)?;
    out.write_all(b"\n")?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("fpc: {error:#}");
        std::process::exit(1);
    }
}
