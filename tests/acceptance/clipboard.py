#!/usr/bin/env -S uv run --script
#
# /// script
# requires-python = ">=3.15"
# dependencies = [
#     "typer>=0.27.3",
# ]
# ///

import contextlib
import getpass
import json
import os
import platform
import shlex
import shutil
import socket
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Annotated

import typer

app = typer.Typer(add_completion=False, pretty_exceptions_enable=False)


def run(args, **kwargs):
    return subprocess.run(args, check=True, timeout=20, **kwargs)


def wait_for(predicate, description, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for {description}")


def shell_script(path, contents):
    path.write_text("#!/bin/sh\nset -eu\n" + contents)
    path.chmod(0o700)
    return str(path)


def clipboard_read():
    command = (
        ["pbpaste"]
        if platform.system() == "Darwin"
        else ["xclip", "-selection", "clipboard", "-o"]
    )
    result = subprocess.run(command, check=False, capture_output=True, timeout=5)
    return result.stdout if result.returncode == 0 else None


def clipboard_write(value):
    command = (
        ["pbcopy"]
        if platform.system() == "Darwin"
        else ["xclip", "-selection", "clipboard"]
    )
    run(command, input=value)


@contextlib.contextmanager
def ssh_server(root):
    host_key = root / "host_key"
    client_key = root / "client_key"
    for key in (host_key, client_key):
        run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", str(key)])
    authorized = root / "authorized_keys"
    authorized.write_bytes(client_key.with_suffix(".pub").read_bytes())
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    pidfile = root / "sshd.pid"
    config = root / "sshd_config"
    pam = "yes" if platform.system() == "Linux" else "no"
    config.write_text(
        f"Port {port}\nListenAddress 127.0.0.1\n"
        f'HostKey "{host_key}"\nPidFile "{pidfile}"\n'
        f'AuthorizedKeysFile "{authorized}"\nAllowUsers {getpass.getuser()}\n'
        "StrictModes no\nPasswordAuthentication no\nKbdInteractiveAuthentication no\n"
        f"PubkeyAuthentication yes\nUsePAM {pam}\nLogLevel VERBOSE\n"
    )
    sshd_log = root / "sshd.log"
    sshd_log.touch(mode=0o600)
    run(["sudo", "-n", "/usr/sbin/sshd", "-t", "-f", str(config)])
    run(["sudo", "-n", "/usr/sbin/sshd", "-f", str(config), "-E", str(sshd_log)])
    try:
        wait_for(pidfile.exists, "isolated sshd PID")
        ssh = [
            "ssh",
            "-F",
            "/dev/null",
            "-i",
            str(client_key),
            "-p",
            str(port),
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            f"UserKnownHostsFile={root / 'known_hosts'}",
            "-o",
            "ConnectTimeout=5",
        ]
        key = host_key.with_suffix(".pub").read_text().split()
        (root / "known_hosts").write_text(f"[127.0.0.1]:{port} {key[0]} {key[1]}\n")
        run([*ssh, f"{getpass.getuser()}@127.0.0.1", "true"])
        yield [*ssh, "-tt", f"{getpass.getuser()}@127.0.0.1"]
    finally:
        if pidfile.exists():
            run(["sudo", "-n", "kill", pidfile.read_text().strip()])


def scenario(root, binary, ghostty, ssh, mode, filename):
    directory = root / mode
    directory.mkdir()
    home = directory / "home"
    home.mkdir()
    target = directory / filename
    target.touch()
    expected = os.fsencode(target.resolve())
    done = directory / "done"
    release = directory / "release"
    output = directory / "fpc.stdout"
    errors = directory / "fpc.stderr"
    precondition = ""
    if mode != "native":
        precondition = 'test -n "${SSH_CONNECTION:-}"\n'
    if mode == "ssh-tmux":
        precondition += (
            'test -n "${TMUX:-}"\n'
            "while test -z \"$(tmux list-clients -F '#{client_tty}')\"; do sleep 0.1; done\n"
        )
    copy = shell_script(
        directory / "copy.sh",
        precondition
        + f"{shlex.quote(str(binary))} {shlex.quote(str(target))} "
        + f">{shlex.quote(str(output))} 2>{shlex.quote(str(errors))}\n"
        + f"touch {shlex.quote(str(done))}\n"
        + f"while ! test -f {shlex.quote(str(release))}; do sleep 0.1; done\n",
    )
    command = [copy]
    tmux_socket = directory / "tmux.sock"
    tmux_config = directory / "tmux.conf"
    tmux_config.write_text(
        "set -g set-clipboard external\nset -as terminal-features ',*:clipboard'\n"
    )
    if mode == "ssh-tmux":
        command = [
            shutil.which("tmux"),
            "-S",
            str(tmux_socket),
            "-f",
            str(tmux_config),
            "new-session",
            "-s",
            "acceptance",
            copy,
        ]
    if mode != "native":
        remote = (
            f"env HOME={shlex.quote(str(home))} "
            f"XDG_CONFIG_HOME={shlex.quote(str(home / '.config'))} "
            f"PATH={shlex.quote(os.environ['PATH'])} " + shlex.join(command)
        )
        command = [*ssh, remote]
    config = directory / "ghostty.config"
    config.write_text(
        "clipboard-write = allow\nclipboard-read = deny\nshell-integration = none\n"
        "confirm-close-surface = false\nquit-after-last-window-closed = true\n"
        "window-save-state = never\n"
    )
    environment = os.environ.copy()
    for key in ("TMUX", "TMUX_PANE", "SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"):
        environment.pop(key, None)
    environment.update(
        HOME=str(home),
        XDG_CONFIG_HOME=str(home / ".config"),
        XDG_DATA_HOME=str(home / ".local/share"),
    )
    clipboard_write(b"fpc acceptance sentinel")
    with (directory / "ghostty.log").open("wb") as log:
        terminal = subprocess.Popen(
            [
                ghostty,
                "--config-default-files=false",
                f"--config-file={config}",
                "-e",
                *command,
            ],
            env=environment,
            stdout=log,
            stderr=subprocess.STDOUT,
        )
        try:
            wait_for(done.exists, f"{mode} copy completion")
            wait_for(lambda: clipboard_read() == expected, f"{mode} system clipboard")
            assert output.read_bytes() == b"", output.read_bytes()
            assert errors.read_bytes() == b"", errors.read_bytes()
            print(f"PASS {mode}: exact clipboard bytes and silent copy", flush=True)
        finally:
            release.touch()
            try:
                terminal.wait(timeout=10)
            except subprocess.TimeoutExpired:
                terminal.terminate()
                terminal.wait(timeout=5)
            if tmux_socket.exists():
                subprocess.run(
                    ["tmux", "-S", str(tmux_socket), "kill-server"],
                    check=False,
                    timeout=5,
                )


@app.command(
    help="Verify actual clipboard writes through Ghostty, SSH, and tmux; requires a desktop and passwordless sudo for isolated sshd."
)
def main(
    binary: Annotated[
        Path,
        typer.Argument(
            exists=True,
            dir_okay=False,
            readable=True,
            resolve_path=True,
            help="Built native fpc executable to test.",
        ),
    ],
    allow_desktop: Annotated[
        bool,
        typer.Option(
            "--allow-desktop",
            help="Allow opening Ghostty and replacing the system clipboard; required outside CI=true.",
        ),
    ] = False,
    artifacts: Annotated[
        Path | None,
        typer.Option(
            "--artifacts",
            file_okay=False,
            resolve_path=True,
            help="Directory for terminal/sshd logs and environment details, including on failure.",
        ),
    ] = None,
):
    if os.environ.get("CI") != "true" and not allow_desktop:
        raise typer.BadParameter(
            "This opens Ghostty and replaces the clipboard; use --allow-desktop explicitly"
        )
    ghostty = os.environ.get("GHOSTTY_BINARY") or shutil.which("ghostty")
    if not ghostty and platform.system() == "Darwin":
        ghostty = "/Applications/Ghostty.app/Contents/MacOS/ghostty"
    if not ghostty or not Path(ghostty).is_file():
        raise typer.BadParameter(
            "Set GHOSTTY_BINARY to the installed Ghostty executable"
        )
    with tempfile.TemporaryDirectory(prefix="fpc-acceptance-") as temporary:
        root = Path(temporary)
        try:
            with ssh_server(root) as ssh:
                for mode in ("native", "ssh", "ssh-tmux"):
                    scenario(
                        root, binary, ghostty, ssh, mode, "路径 ' quoted ; $ space.txt"
                    )
        finally:
            if artifacts:
                artifacts.mkdir(parents=True, exist_ok=True)
                for path in root.rglob("*"):
                    if path.is_file() and path.suffix in (".log", ".stdout", ".stderr"):
                        destination = artifacts / path.relative_to(root)
                        destination.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copyfile(path, destination)
                (artifacts / "environment.json").write_text(
                    json.dumps(
                        {
                            "platform": platform.platform(),
                            "ghostty": ghostty,
                        },
                        indent=2,
                    )
                )


if __name__ == "__main__":
    app()
