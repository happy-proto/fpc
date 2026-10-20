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
            if expected in clean or expected in output:
                return clean
        raise AssertionError(f"Timed out waiting for {expected!r}: {output!r}")

    def send(self, text):
        os.write(self.fd, text.encode())

    def menu(self, prefix):
        self.send(f"fpc {prefix}\t")
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
            "line\nbreak.txt",
            "odd ' quote $(touch PWNED); [x].txt",
            "中文路径.txt",
        ]:
            (work / name).touch()
        (work / "folder").mkdir()
        (work / "folder/nested.txt").touch()
        (home / "bin").mkdir()
        (home / "bin/fpc").symlink_to(binary)
        for name in ["pbcopy", "wl-copy", "xclip", "xsel"]:
            stub = home / "bin" / name
            stub.write_text(
                '#!/bin/sh\nprintf copy >> "$HOME/clipboard-calls"\ncat >/dev/null\n'
            )
            stub.chmod(0o755)
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
            (home / "config/fish/conf.d/00-custom-tab.fish").write_text(
                "function previous_tab\n"
                "    printf used >> $HOME/previous-tab\n"
                "    commandline -f complete\n"
                "end\n"
                "bind \\t previous_tab\n"
            )
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

            option = menu_for("--re")
            assert "--respect-ignore" in option, (shell, option)
            shells = menu_for("--install-completion ")
            for value in ["fish", "bash", "zsh"]:
                assert value in shells, (shell, value, shells)
            assert "sample.txt" not in shells, (shell, shells)

            def open_picker(prefix):
                terminal.send(f"fpc {prefix}\t")
                return terminal.wait_for("insert")

            def close_picker():
                output = terminal.wait_for("\x1b[?1049l")
                if shell == "fish" and PROMPT not in output:
                    terminal.wait_for(PROMPT)

            def cancel_to_prompt():
                terminal.send("\x1b")
                close_picker()
                terminal.send("\x01\x0bprintf '\\nFPC_RESET\\n'\r")
                output = terminal.wait_for("\nFPC_RESET\n")
                if PROMPT not in output.split("\nFPC_RESET\n", 1)[1]:
                    terminal.wait_for(PROMPT)

            opened = open_picker("--print ")
            assert "sample.txt" in opened and ".readme-secret" not in opened, (
                shell,
                opened,
            )
            cancel_to_prompt()
            opened = open_picker("--print .")
            assert ".env" in opened, (shell, opened)
            cancel_to_prompt()

            # Completion itself never invokes clipboard tools, even without --print.
            open_picker("sam")
            terminal.send("\r")
            close_picker()
            terminal.send("\x01\x0bprintf '\\nFPC_RESET\\n'\r")
            output = terminal.wait_for("\nFPC_RESET\n")
            if PROMPT not in output.split("\nFPC_RESET\n", 1)[1]:
                terminal.wait_for(PROMPT)
            assert not (home / "clipboard-calls").exists(), shell

            terminal.send("echo sam\t; printf '\\nFPC_OTHER\\n'\r")
            output = terminal.wait_for("\nFPC_OTHER\n")
            assert "sample.txt" in output, (shell, output)
            if PROMPT not in output.split("\nFPC_OTHER\n", 1)[1]:
                terminal.wait_for(PROMPT)
            if shell == "fish":
                assert (home / "previous-tab").read_text() == "used"

            # Even a single result requires confirmation, and cancellation keeps the token.
            for cancel in ["\x1b", "\x03"]:
                opened = open_picker("--print readmemd")
                assert "README.md" in opened, (shell, opened)
                terminal.send(cancel)
                close_picker()
                terminal.send("\r")
                output = terminal.wait_for("Cannot access readmemd")
                if PROMPT not in output.split("Cannot access readmemd", 1)[1]:
                    terminal.wait_for(PROMPT)

            # A query with no matches stays editable instead of inserting a guessed path.
            opened = open_picker("--print zzzzzzzzz")
            assert "0/" in opened.replace(" ", ""), (shell, opened)
            terminal.send("\x15readmemd")
            terminal.wait_for("README.md")
            terminal.send("\x1b")
            close_picker()
            terminal.send("\r")
            output = terminal.wait_for("Cannot access zzzzzzzzz")
            if PROMPT not in output.split("Cannot access zzzzzzzzz", 1)[1]:
                terminal.wait_for(PROMPT)

            for typed, expected in [
                ("sam", work / "sample.txt"),
                ("readmemd", work / "README.md"),
                ("rdm", work / "README.md"),
                ("zh", work / "README.zh-CN.md"),
                ("folder/nes", work / "folder/nested.txt"),
                ("sp", work / "space name.txt"),
                ("linebreak", work / "line\nbreak.txt"),
                ("odd", work / "odd ' quote $(touch PWNED); [x].txt"),
                ("中文", work / "中文路径.txt"),
                (".e", work / ".env"),
                ("-- --r", work / "--report"),
                (f"{work}/sam", work / "sample.txt"),
                ("../work/sam", work / "../work/sample.txt"),
                ("~/work/sam", work / "sample.txt"),
            ]:
                open_picker("--print " + typed)
                terminal.send("\r")
                # The first Enter only fills the command line; the next executes it.
                close_picker()
                terminal.send(
                    f" > {shlex.quote(str(home / 'result'))}; printf '\\nFPC_DONE\\n'\r"
                )
                output = terminal.wait_for("\nFPC_DONE\n")
                assert (home / "result").read_bytes() == os.fsencode(
                    expected
                ) + b"\n", (
                    shell,
                    typed,
                    output,
                )
                assert not (work / "PWNED").exists(), (shell, typed)
                if PROMPT not in output.split("\nFPC_DONE\n", 1)[1]:
                    terminal.wait_for(PROMPT)

            suffix = " --respect-ignore"
            terminal.send("fpc --print readmemd" + suffix + "\x02" * len(suffix) + "\t")
            terminal.wait_for("insert")
            terminal.send("\r")
            close_picker()
            terminal.send(
                f"\x05 > {shlex.quote(str(home / 'result'))}; printf '\\nFPC_MIDDLE\\n'\r"
            )
            output = terminal.wait_for("\nFPC_MIDDLE\n")
            assert (home / "result").read_bytes() == os.fsencode(
                work / "README.md"
            ) + b"\n", (
                shell,
                output,
            )
            if PROMPT not in output.split("\nFPC_MIDDLE\n", 1)[1]:
                terminal.wait_for(PROMPT)

            # Directory confirmation keeps the slash so another Tab opens that directory.
            open_picker("--print fol")
            terminal.send("\r")
            close_picker()
            terminal.send("nes\t")
            terminal.wait_for("insert")
            terminal.send("\r")
            close_picker()
            terminal.send("\r")
            output = terminal.wait_for(str(work / "folder/nested.txt") + "\n")
            if PROMPT not in output.split(str(work / "folder/nested.txt") + "\n", 1)[1]:
                terminal.wait_for(PROMPT)

            # The ignore-file value also uses the picker; selecting never copies a path.
            for option in ["--ignore-file sam", "--ignore-file=sam"]:
                open_picker("--print " + option)
                terminal.send("\r")
                close_picker()
                terminal.send(" ./sample.txt\r")
                output = terminal.wait_for(str(work / "sample.txt") + "\n")
                if PROMPT not in output.split(str(work / "sample.txt") + "\n", 1)[1]:
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
