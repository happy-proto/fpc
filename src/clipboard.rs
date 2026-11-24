use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{
    env,
    fs::OpenOptions,
    io::{self, IsTerminal, Read, Seek, Write},
    process::{Command, Stdio},
};

fn command_copy(program: &str, args: &[&str], bytes: &[u8]) -> Result<()> {
    // Clipboard owners may outlive their parent and keep inherited stderr open.
    let mut stderr = tempfile::tempfile().context("Cannot create clipboard diagnostics")?;
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(stderr.try_clone()?)
        .spawn()
        .with_context(|| format!("Cannot start {program}"))?;
    let written = child
        .stdin
        .take()
        .context("Cannot open clipboard input")?
        .write_all(bytes);
    let status = child
        .wait()
        .with_context(|| format!("Cannot wait for {program}"))?;
    if !status.success() {
        stderr.rewind()?;
        let mut diagnostics = Vec::new();
        stderr.take(64 * 1024).read_to_end(&mut diagnostics)?;
        bail!(
            "{program} exited with {}: {}",
            status,
            String::from_utf8_lossy(&diagnostics).trim()
        );
    }
    written.with_context(|| format!("Cannot write to {program}"))
}

fn present(key: &str) -> bool {
    env::var_os(key).is_some_and(|value| !value.is_empty())
}

const REMOTE_MARKERS: &[&str] = &["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"];

fn osc_reason() -> Option<&'static str> {
    if env::var_os("TERM_PROGRAM").as_deref() == Some("rex".as_ref()) {
        return Some("TERM_PROGRAM=rex");
    }
    REMOTE_MARKERS.iter().copied().find(|key| present(key))
}

pub fn osc52(bytes: &[u8]) -> Vec<u8> {
    format!("\x1b]52;c;{}\x07", STANDARD.encode(bytes)).into_bytes()
}

fn tmux_output(args: &[&str]) -> Result<String> {
    let output = Command::new("tmux")
        .args(args)
        .output()
        .context("Cannot query tmux")?;
    if !output.status.success() {
        bail!(
            "tmux query failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8(output.stdout).context("tmux output is not Unicode")
}

fn tmux_client<'a>(clients: &'a str, pane: &str) -> Option<&'a str> {
    clients
        .lines()
        .filter_map(|line| {
            let mut columns = line.split('\t');
            let tty = columns.next()?;
            let visible_pane = columns.next()?;
            let activity = columns.next()?.parse::<u64>().ok()?;
            (!tty.is_empty() && visible_pane == pane).then_some((tty, activity))
        })
        .max_by_key(|(_, activity)| *activity)
        .map(|(tty, _)| tty)
}

fn osc_tty() -> Result<std::fs::File> {
    OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .context("No controlling terminal for OSC 52")
}

fn osc_copy(bytes: &[u8]) -> Result<()> {
    let mut tty = osc_tty()?;
    tty.write_all(&osc52(bytes))?;
    tty.flush()?;
    Ok(())
}

fn tmux_target() -> Result<String> {
    // Prefer the most recently active client viewing this pane. Mirrored clients
    // cannot be distinguished reliably from inside the pane.
    let pane = env::var("TMUX_PANE").context("TMUX_PANE is missing")?;
    let session = tmux_output(&["display-message", "-p", "-t", &pane, "#{session_id}"])?;
    if session.trim().is_empty() {
        bail!("No tmux session for this pane");
    }
    let clients = tmux_output(&[
        "list-clients",
        "-t",
        session.trim(),
        "-F",
        "#{client_tty}\t#{pane_id}\t#{client_activity}",
    ])?;
    tmux_client(&clients, &pane)
        .map(str::to_owned)
        .context("No attached tmux client viewing this pane")
}

type NativeCommand = (&'static str, &'static [&'static str], Option<&'static str>);

#[cfg(target_os = "macos")]
const NATIVE: &[NativeCommand] = &[("pbcopy", &[], None)];
#[cfg(target_os = "linux")]
const NATIVE: &[NativeCommand] = &[
    ("wl-copy", &[], Some("WAYLAND_DISPLAY")),
    ("xclip", &["-selection", "clipboard"], Some("DISPLAY")),
];
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const NATIVE: &[NativeCommand] = &[];

fn native_commands() -> impl Iterator<Item = (&'static str, &'static [&'static str])> {
    NATIVE.iter().filter_map(|&(program, args, marker)| {
        marker.is_none_or(present).then_some((program, args))
    })
}

pub fn doctor(out: &mut impl Write) -> io::Result<()> {
    for key in ["TERM", "TERM_PROGRAM", "TERM_PROGRAM_VERSION"] {
        match env::var_os(key) {
            Some(value) => writeln!(out, "{key}={value:?}")?,
            None => writeln!(out, "{key}: unset")?,
        }
    }
    for key in [
        "REX_BLOCK",
        "TMUX",
        "TMUX_PANE",
        "DISPLAY",
        "WAYLAND_DISPLAY",
    ]
    .into_iter()
    .chain(REMOTE_MARKERS.iter().copied())
    {
        let state = match env::var_os(key) {
            None => "unset",
            Some(value) if value.is_empty() => "empty",
            Some(_) => "set",
        };
        writeln!(out, "{key}: {state}")?;
    }
    writeln!(
        out,
        "TTY: stdin={}, stdout={}, stderr={}",
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
        io::stderr().is_terminal()
    )?;
    if present("TMUX") {
        writeln!(out, "Clipboard route: tmux (TMUX)")?;
        match tmux_target() {
            Ok(_) => writeln!(out, "tmux target: attached client found")?,
            Err(error) => writeln!(out, "tmux target: {error:#}")?,
        }
    } else if let Some(reason) = osc_reason() {
        writeln!(out, "Clipboard route: OSC 52 ({reason})")?;
    } else {
        let route = native_commands()
            .map(|(program, _)| program)
            .chain(["OSC 52"])
            .collect::<Vec<_>>();
        writeln!(out, "Clipboard route: {}", route.join(" -> "))?;
    }
    match osc_tty() {
        Ok(_) => writeln!(out, "/dev/tty: writable")?,
        Err(error) => writeln!(out, "/dev/tty: {error:#}")?,
    }
    writeln!(out, "Clipboard delivery: untested (no clipboard write)")
}

pub fn copy(bytes: &[u8]) -> Result<()> {
    if present("TMUX") {
        let client = tmux_target()?;
        return command_copy("tmux", &["load-buffer", "-w", "-t", &client, "-"], bytes);
    }
    if osc_reason().is_some() {
        return osc_copy(bytes);
    }
    let mut failures = Vec::new();
    for (program, args) in native_commands() {
        match command_copy(program, args, bytes) {
            Ok(()) => return Ok(()),
            Err(error) => failures.push(format!("{error:#}")),
        }
    }
    match osc_copy(bytes) {
        Ok(()) => Ok(()),
        Err(error) => {
            failures.push(format!("{error:#}"));
            bail!("{}", failures.join("; "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tmux_client_selection_stays_with_the_current_pane() {
        let clients = "/dev/pts/1\t%1\t20\n/dev/pts/2\t%2\t40\n/dev/pts/3\t%1\t30\n";
        assert_eq!(tmux_client(clients, "%1"), Some("/dev/pts/3"));
        assert_eq!(tmux_client(clients, "%4"), None);
    }
    #[test]
    #[cfg(unix)]
    fn clipboard_command_failure_is_propagated() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let program = temp.path().join("clipboard");
        std::fs::write(&program, "#!/bin/sh\nexit 7\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(command_copy(program.to_str().unwrap(), &[], b"path").is_err());
    }
    #[test]
    fn osc52_encodes_exact_bytes_without_a_newline() {
        assert_eq!(osc52(b"/tmp/a\nb"), b"\x1b]52;c;L3RtcC9hCmI=\x07");
    }
}
