#!/usr/bin/env -S uv run --script
#
# /// script
# requires-python = ">=3.15"
# dependencies = [
#     "typer>=0.27.3",
# ]
# ///

import errno
import fcntl
import os
import pty
import re
import select
import shlex
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
from enum import StrEnum
from pathlib import Path
from typing import Annotated

import typer

app = typer.Typer(add_completion=False, pretty_exceptions_enable=False)


class Shell(StrEnum):
    bash = "bash"
    fish = "fish"
    zsh = "zsh"


PROMPT = "FPC_READY> "
ANSI = re.compile(
    r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\)|[()][A-Z0-9]|.)"
)


class Terminal:
    def __init__(self, command, environment, directory):
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(directory)
            os.execve(command[0], command, environment)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        try:
            self.wait_for(PROMPT)
        except BaseException:
            self.close()
            raise

    def read(self, timeout):
        if not select.select([self.fd], [], [], timeout)[0]:
            return ""
        try:
            return os.read(self.fd, 65536).decode(errors="replace")
        except OSError as error:
            if error.errno == errno.EIO:
                raise AssertionError("Shell exited unexpectedly") from error
            raise

    def wait_for(self, expected, timeout=8):
        output = ""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            output += self.read(0.1)
            clean = ANSI.sub("", output).replace("\r", "")
            if expected in clean:
                return clean
        raise AssertionError(f"Timed out waiting for {expected!r}: {output!r}")

    def send(self, text):
        os.write(self.fd, text.encode())

    def menu(self, prefix):
        self.send(f"fpc {prefix}\t\t")
        output = ""
        deadline = time.monotonic() + 0.8
        while time.monotonic() < deadline:
            output += self.read(0.05)
        self.send("\x03")
        self.wait_for(PROMPT)
        return ANSI.sub("", output).replace("\r", "")

    def close(self):
        os.kill(self.pid, signal.SIGKILL)
        os.close(self.fd)
        os.waitpid(self.pid, 0)


def framework():
    override = os.environ.get("BASH_COMPLETION_FILE")
    candidates = (
        [Path(override)]
        if override
        else [
            Path("/usr/share/bash-completion/bash_completion"),
            Path("/usr/local/share/bash-completion/bash_completion"),
            Path("/opt/homebrew/share/bash-completion/bash_completion"),
        ]
    )
    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()
    raise AssertionError(
        "Install bash-completion, or set BASH_COMPLETION_FILE to its main script"
    )


def verify(shell, executable, binary, completion_framework):
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix=f"fpc-{shell}-e2e-") as temporary:
        home = Path(temporary).resolve()
        work = home / "work"
        work.mkdir()
        for name in [
            "sample.txt",
            "space name.txt",
            ".env",
            "--report",
            "README.md",
            "README.zh-CN.md",
            "old-readme.txt",
            ".readme-secret",
        ]:
            (work / name).touch()
        (work / "folder").mkdir()
        (work / "folder/nested.txt").touch()
        (home / "bin").mkdir()
        (home / "bin/fpc").symlink_to(binary)
        if completion_framework is not None:
            (home / "system/bash-completion").mkdir(parents=True)
            (home / "system/bash-completion/bash_completion").symlink_to(
                completion_framework
            )
        environment = {
            "HOME": str(home),
            "PATH": f"{home / 'bin'}:{os.environ['PATH']}",
            "TERM": "dumb",
            "LC_ALL": "C.UTF-8" if sys.platform != "darwin" else "en_US.UTF-8",
            "XDG_CONFIG_HOME": str(home / "config"),
            "XDG_DATA_HOME": str(home / "data"),
            "XDG_DATA_DIRS": str(home / "system"),
            "BASH_COMPLETION_USER_DIR": str(home / "data/bash-completion"),
            "ZDOTDIR": str(home),
            "SHELL": executable,
        }
        subprocess.run(
            [str(binary), "--install-completion", shell],
            env=environment,
            cwd=work,
            check=True,
            capture_output=True,
        )
        subprocess.run(
            [str(binary), "--install-completion"],
            env=environment,
            cwd=work,
            check=True,
            capture_output=True,
        )
        if shell == "bash":
            startup = home / "bashrc"
            startup.write_text(
                f"PS1={shlex.quote(PROMPT)}\nsource {shlex.quote(str(completion_framework))}\nbind 'set show-all-if-ambiguous on'\n"
            )
            command = [executable, "--noprofile", "--rcfile", str(startup), "-i"]
        elif shell == "zsh":
            (home / ".zshrc").write_text(
                f"PS1={shlex.quote(PROMPT)}\nfpath=({shlex.quote(str(home / 'data/zsh/site-functions'))} $fpath)\nautoload -Uz compinit\ncompinit -D -u\nbindkey '^I' expand-or-complete\n"
            )
            command = [executable, "-d", "-i"]
        else:
            command = [
                executable,
                "--interactive",
                "--init-command",
                f"function fish_prompt; printf '%s' {shlex.quote(PROMPT)}; end; set -g fish_greeting; set -g fish_autosuggestion_enabled 0",
            ]
        terminal = Terminal(command, environment, work)
        try:
            completion_work = work

            def menu_for(prefix):
                if shell == "fish":
                    return subprocess.run(
                        [
                            executable,
                            "-c",
                            f"complete -C {shlex.quote('fpc ' + prefix)}",
                        ],
                        env=environment,
                        cwd=completion_work,
                        check=True,
                        capture_output=True,
                        text=True,
                    ).stdout
                return terminal.menu(prefix)

            menu = menu_for("")
            assert "sample.txt" in menu and "folder" in menu, (shell, menu)
            assert not re.search(r"(?:^|\s)\.(?:\s|$)", menu), (shell, menu)
            for option in [
                "--help",
                "--version",
                "--print",
                "--respect-ignore",
                "--ignore-file",
                "--install-completion",
            ]:
                assert option not in menu, (shell, menu)
            hidden = menu_for(".")
            assert ".env" in hidden, (shell, hidden)
            readme = menu_for("readme")
            assert "README.md" in readme and "README.zh-CN.md" in readme, (
                shell,
                readme,
            )
            assert "old-readme.txt" not in readme and ".readme-secret" not in readme, (
                shell,
                readme,
            )
            substring = menu_for("zh")
            assert "README.zh-CN.md" in substring, (shell, substring)
            nonconsecutive = menu_for("rdm")
            assert "README" not in nonconsecutive, (shell, nonconsecutive)
            nested = menu_for("folder/nes")
            assert "nested.txt" in nested, (shell, nested)
            directory = menu_for("FOL")
            assert "folder/" in directory, (shell, directory)
            rules_readme = menu_for("--ignore-file readme")
            assert "README.md" in rules_readme and "README.zh-CN.md" in rules_readme, (
                shell,
                rules_readme,
            )
            option = menu_for("--re")
            assert "--respect-ignore" in option, (shell, option)
            shells = menu_for("--install-completion ")
            for value in ["fish", "bash", "zsh"]:
                assert value in shells, (shell, value, shells)
            assert not re.search(r"(?:^|\s)auto(?:\s|$)", shells), (shell, shells)
            assert "sample.txt" not in shells and "--help" not in shells, (
                shell,
                shells,
            )
            rules = menu_for("--ignore-file ")
            assert "sample.txt" in rules and "folder" in rules, (shell, rules)
            assert "--help" not in rules, (shell, rules)
            escaped = menu_for("-- --r")
            assert "--report" in escaped and "--respect-ignore" not in escaped, (
                shell,
                escaped,
            )
            empty = home / "empty"
            empty.mkdir()
            terminal.send(f"cd {shlex.quote(str(empty))}\r")
            terminal.wait_for(PROMPT)
            completion_work = empty
            empty_menu = menu_for("")
            assert (
                "--help" not in empty_menu and "--respect-ignore" not in empty_menu
            ), (shell, empty_menu)
            terminal.send(f"cd {shlex.quote(str(work))}\r")
            terminal.wait_for(PROMPT)
            completion_work = work
            terminal.send("fpc --install-completion fi\t\r")
            destination = str(home / "config/fish/completions/fpc.fish")
            output = terminal.wait_for(destination + "\n")
            if PROMPT not in output.split(destination + "\n", 1)[1]:
                terminal.wait_for(PROMPT)
            for command in [
                "fpc --print --re\t ./sample.txt\r",
                "fpc --print --ignore-file sam\t ./sample.txt\r",
            ]:
                terminal.send(command)
                expected = str(work / "sample.txt")
                output = terminal.wait_for(expected + "\n")
                assert expected in output.splitlines(), (shell, command, output)
                if PROMPT not in output.split(expected + "\n", 1)[1]:
                    terminal.wait_for(PROMPT)
            for typed, expected in [
                ("sam", work / "sample.txt"),
                ("folder/nes", work / "folder/nested.txt"),
                ("sp", work / "space name.txt"),
                ("NAME", work / "space name.txt"),
                ("zh", work / "README.zh-CN.md"),
                (".e", work / ".env"),
                ("-- --r", work / "--report"),
                (f"{work}/sam", work / "sample.txt"),
                ("../work/sam", work / "../work/sample.txt"),
            ]:
                terminal.send(f"fpc --print {typed}\t\r")
                output = terminal.wait_for(str(expected) + "\n")
                assert str(expected) in output.splitlines(), (shell, typed, output)
                if PROMPT not in output.split(str(expected) + "\n", 1)[1]:
                    terminal.wait_for(PROMPT)
        finally:
            terminal.close()
    print(
        f"{shell}: real Tab, contextual options and values, path menus, navigation and quoting passed ({time.monotonic() - started:.2f}s)"
    )


@app.command(help="Verify real Tab completion in isolated shell sessions.")
def main(
    binary: Annotated[
        Path,
        typer.Argument(
            exists=True,
            dir_okay=False,
            readable=True,
            resolve_path=True,
            help="Built fpc executable to test.",
        ),
    ],
    selected_shells: Annotated[
        list[Shell] | None,
        typer.Option(
            "--shell",
            help="Shell to test; repeat for multiple shells. Defaults to bash, fish, and zsh; missing prerequisites always fail.",
        ),
    ] = None,
):
    shells = selected_shells or list(Shell)
    completion_framework = None
    if "bash" in shells:
        completion_framework = framework()
    for shell in shells:
        executable = shutil.which(shell)
        assert executable, f"Required shell is missing: {shell}"
        verify(shell, executable, binary, completion_framework)


if __name__ == "__main__":
    app()
