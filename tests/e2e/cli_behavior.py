#!/usr/bin/env -S uv run --script
#
# /// script
# requires-python = ">=3.15"
# dependencies = [
#     "typer>=0.27.3",
# ]
# ///

import base64
import datetime
import errno
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time
import tomllib
from pathlib import Path
from typing import Annotated

import typer

app = typer.Typer(add_completion=False, pretty_exceptions_enable=False)


class Process:
    def __init__(self, binary, arguments, directory, environment):
        self.stdout = directory / "stdout"
        self.stderr = directory / "stderr"
        ready, start = os.pipe()
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.close(start)
            os.read(ready, 1)
            os.close(ready)
            os.chdir(directory)
            for number, path in [(1, self.stdout), (2, self.stderr)]:
                descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
                os.dup2(descriptor, number)
                os.close(descriptor)
            os.execve(str(binary), [str(binary), *arguments], environment)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 120, 0, 0))
        self.output = b""
        self.status = None
        self.initial_modes = termios.tcgetattr(self.fd)[3]
        os.close(ready)
        os.write(start, b"1")
        os.close(start)

    def read(self, timeout=0.05):
        if select.select([self.fd], [], [], timeout)[0]:
            try:
                self.output += os.read(self.fd, 65536)
            except OSError as error:
                if error.errno != errno.EIO:
                    raise
        if self.status is None:
            pid, status = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                self.status = os.waitstatus_to_exitcode(status)

    def wait_for(self, expected, timeout=5):
        deadline = time.monotonic() + timeout
        while expected not in self.output:
            self.read()
            assert self.status is None, (
                expected,
                self.output,
                self.stderr.read_bytes(),
            )
            assert time.monotonic() < deadline, (expected, self.output)

    def send(self, value):
        os.write(self.fd, value)

    def finish(self):
        deadline = time.monotonic() + 5
        while self.status is None:
            self.read()
            assert time.monotonic() < deadline, self.output
        self.read(0)
        assert self.status == 0, (self.status, self.stderr.read_bytes())
        mask = termios.ECHO | termios.ICANON | termios.ISIG
        assert termios.tcgetattr(self.fd)[3] & mask == self.initial_modes & mask
        return self.stdout.read_bytes(), self.stderr.read_bytes()

    def close(self):
        if self.status is None:
            os.kill(self.pid, signal.SIGKILL)
            os.waitpid(self.pid, 0)
        os.close(self.fd)


def environment(root):
    return {"HOME": str(root), "PATH": "/usr/bin:/bin", "TERM": "xterm-256color"}


def remote_copy(binary, root):
    name = "-space name\n中文"
    target = root / name
    target.touch()
    expected = os.fsencode(target.resolve())
    for variable in ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]:
        env = environment(root) | {variable: "remote-session"}
        process = Process(binary, ["--", name], root, env)
        try:
            stdout, stderr = process.finish()
            assert stdout == stderr == b"", (stdout, stderr)
            assert (
                process.output == b"\x1b]52;c;" + base64.b64encode(expected) + b"\x07"
            )
        finally:
            process.close()
        output = subprocess.run(
            [str(binary), "--", name],
            cwd=root,
            env=env,
            start_new_session=True,
            capture_output=True,
            check=True,
        )
        assert output.stdout == expected + b"\n", output
        assert b"Clipboard unavailable" in output.stderr, output
        assert b"\x1b]52;" not in output.stdout + output.stderr, output


def picker(binary, root, arguments, query, target=None, keys=b"\r"):
    process = Process(binary, ["--print", *arguments], root, environment(root))
    try:
        process.wait_for(b"\x1b[?1049h")
        process.wait_for(b"visible.txt")
        if query:
            process.send(query.encode())
            time.sleep(0.12)
        process.send(keys)
        stdout, stderr = process.finish()
        assert stderr == b"", stderr
        expected = b"" if target is None else os.fsencode(target.resolve()) + b"\n"
        assert stdout == expected, (stdout, expected, process.output)
        assert b"\x1b[?1049l" in process.output, process.output
    finally:
        process.close()


def excluded(binary, root, arguments, query):
    process = Process(binary, ["--print", *arguments], root, environment(root))
    try:
        process.wait_for(b"visible.txt")
        process.send(query.encode())
        time.sleep(0.12)
        process.send(b"\r")
        deadline = time.monotonic() + 0.15
        while time.monotonic() < deadline:
            process.read(0.02)
        assert process.status is None, (query, process.output)
        process.send(b"\x1b")
        assert process.finish() == (b"", b"")
    finally:
        process.close()


def verify_picker(binary, root):
    for name in [
        "visible.txt",
        "nav-a",
        "nav-b",
        ".secret",
        "ignored-only",
        "odd space\n中文",
    ]:
        (root / name).touch()
    (root / ".git").mkdir()
    (root / ".gitignore").write_text("ignored-only\n")
    (root / ".private").mkdir()
    (root / ".private/nested-target").touch()
    (root / "rules").write_text(".private/\n")
    picker(binary, root, [], "odd", root / "odd space\n中文")
    picker(binary, root, [], ".secret", root / ".secret")
    picker(binary, root, [], "ignored-only", root / "ignored-only")
    picker(
        binary,
        root,
        ["--respect-ignore"],
        ".private/nested",
        root / ".private/nested-target",
    )
    picker(binary, root, [], ".private", root / ".private")
    for key in [b"\t", b"\x1b[B"]:
        picker(binary, root, [], "nav", root / "nav-b", keys=key + b"\r")
    for key in [b"\x1b", b"\x03", b"\x04"]:
        picker(binary, root, [], "", keys=key)
    excluded(binary, root, ["--respect-ignore"], "ignored-only")
    excluded(binary, root, ["--ignore-file", "rules"], "nested-target")
    output = subprocess.run(
        [str(binary), "--print"],
        cwd=root,
        env=environment(root),
        start_new_session=True,
        capture_output=True,
        check=False,
    )
    assert output.returncode != 0 and output.stdout == b"", output
    assert output.stderr, output


def verify_build_information(binary):
    package = tomllib.loads(
        (Path(__file__).resolve().parents[2] / "Cargo.toml").read_text()
    )["package"]
    result = subprocess.run(
        [str(binary), "--version"], capture_output=True, text=True, check=True
    )
    lines = result.stdout.splitlines()
    assert not result.stderr and lines[0] == f"fpc {package['version']}", result
    fields = dict(line.split(": ", 1) for line in lines[1:] if ": " in line)
    commit = fields["Commit"].removesuffix(" (modified)")
    assert commit == "unknown" or (
        len(commit) == 40
        and all(character in "0123456789abcdef" for character in commit)
    ), fields
    assert datetime.datetime.fromisoformat(fields["Built"]).tzinfo is not None, fields
    assert fields["Target"] and fields["Rust"].startswith("rustc "), fields
    assert package["repository"] in lines, lines
    short = subprocess.run(
        [str(binary), "-V"], capture_output=True, text=True, check=True
    )
    assert short.stdout == lines[0] + "\n" and not short.stderr, short
    help_output = subprocess.run(
        [str(binary), "--help"], capture_output=True, text=True, check=True
    )
    assert package["repository"] in help_output.stdout.splitlines(), help_output
    assert not help_output.stderr, help_output


@app.command(help="Verify picker and remote clipboard behavior through isolated PTYs.")
def main(
    binary: Annotated[
        Path,
        typer.Argument(
            exists=True,
            dir_okay=False,
            readable=True,
            resolve_path=True,
            help="Built fpc executable to test; no desktop, SSH server, or clipboard required.",
        ),
    ],
):
    started = time.monotonic()
    verify_build_information(binary)
    with tempfile.TemporaryDirectory(prefix="fpc-behavior-") as temporary:
        root = Path(temporary).resolve()
        remote = root / "remote"
        remote.mkdir()
        remote_copy(binary, remote)
        work = root / "picker"
        work.mkdir()
        verify_picker(binary, work)
    print(
        f"PTY clipboard and picker behavior passed ({time.monotonic() - started:.2f}s)"
    )


if __name__ == "__main__":
    app()
