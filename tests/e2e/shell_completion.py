#!/usr/bin/env -S uv run --script
#
# /// script
# requires-python = ">=3.15"
# dependencies = [
#     "pyte>=0.8.2",
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

import pyte
import typer

app = typer.Typer(add_completion=False, pretty_exceptions_enable=False)


class Shell(StrEnum):
    bash = "bash"
    fish = "fish"
    zsh = "zsh"


PROMPT = "FPC_READY> "
ANSI = re.compile(
    r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*?(?:\x07|\x1b\\)|[()][A-Z0-9]|.)"
)


class Terminal:
    def __init__(self, command, environment, directory):
        self.check_alt_screen = False
        self.shell = Path(command[0]).name
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(directory)
            os.execve(command[0], command, environment)
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
        self.screen = pyte.Screen(120, 40)
        self.screen.write_process_input = self.send
        self.stream = pyte.ByteStream(self.screen)
        try:
            self.wait_for(PROMPT)
            self.check_alt_screen = True
        except BaseException:
            self.close()
            raise

    def read(self, timeout):
        if not select.select([self.fd], [], [], timeout)[0]:
            return ""
        try:
            data = os.read(self.fd, 65536)
            self.stream.feed(data)
            output = data.decode(errors="replace")
            assert not self.check_alt_screen or "\x1b[?1049h" not in output, (
                "Tab completion entered the alternate screen"
            )
            return output
        except OSError as error:
            if error.errno == errno.EIO:
                raise AssertionError("Shell exited unexpectedly") from error
            raise

    def wait_for(self, expected, timeout=8, *, line=False):
        output = ""
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            output += self.read(0.1)
            clean = ANSI.sub("", output).replace("\r", "")
            if (
                expected.rstrip("\n") in clean.splitlines()
                if line
                else expected in clean
            ):
                return clean
        raise AssertionError(f"Timed out waiting for {expected!r}: {output!r}")

    def send(self, text):
        os.write(self.fd, text.encode())

    def wait_screen(self, predicate, timeout=8):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.read(0.05)
            if predicate(self.screen.display):
                return
        raise AssertionError(self.screen.display)

    def resize(self, columns, rows):
        self.screen.resize(lines=rows, columns=columns)
        fcntl.ioctl(
            self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0)
        )

    def clear_line(self):
        self.send("\x15printf 'FPC_MENU_DONE\\n'\r")
        cleared = self.wait_for("FPC_MENU_DONE\n", line=True)
        if PROMPT not in cleared.rsplit("FPC_MENU_DONE\n", 1)[1]:
            self.wait_for(PROMPT)

    def menu(self, prefix):
        self.send(f"fpc {prefix}" + ("\t" if self.shell == "fish" else "\t\t"))
        output = ""
        deadline = time.monotonic() + 0.8
        while time.monotonic() < deadline:
            output += self.read(0.05)
        if self.shell == "fish":
            # Fish can cancel its pager without reprinting the unchanged prompt.
            self.send("\x03")
            self.wait_screen(
                lambda rows: not any("Enter insert" in row for row in rows)
            )
            self.clear_line()
        else:
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


def verify_fish_inline_menu(terminal):
    terminal.send("printf 'FPC_CONTEXT_MARKER\\n'\r")
    terminal.wait_for("FPC_CONTEXT_MARKER\n", line=True)
    terminal.send("fpc readmemd\t")
    terminal.wait_screen(lambda rows: any("Enter insert" in row for row in rows))
    rows = terminal.screen.display
    assert any("FPC_CONTEXT_MARKER" in row for row in rows), rows
    assert any("FPC_READY> fpc readmemd" in row for row in rows), rows
    assert any("README.md" in row for row in rows), rows
    terminal.send("\x1b")
    terminal.wait_screen(
        lambda rows: (
            not any("Enter insert" in row for row in rows)
            and any("FPC_READY> fpc readmemd" in row for row in rows)
        )
    )
    terminal.clear_line()

    terminal.send("fpc rdm\t")
    terminal.wait_screen(lambda rows: any("1 / 3" in row for row in rows))
    for name in ["README.md", "README.zh-CN.md", "old-readme.txt"]:
        assert any(name in row for row in terminal.screen.display), name
    terminal.send("\x03")
    terminal.wait_screen(
        lambda rows: (
            not any("Enter insert" in row for row in rows)
            and any("FPC_READY> fpc rdm" in row for row in rows)
        )
    )
    terminal.clear_line()

    suffix = " --print"
    terminal.send("fpc readmemd" + suffix + "\x1b[D" * len(suffix) + "\t")
    terminal.wait_screen(lambda rows: any("Enter insert" in row for row in rows))
    terminal.send("\x1b")
    terminal.wait_screen(lambda rows: not any("Enter insert" in row for row in rows))
    terminal.send("x")
    terminal.wait_screen(
        lambda rows: any("fpc readmemdx --print" in row for row in rows)
    )
    terminal.clear_line()

    terminal.resize(64, 8)
    terminal.send("printf 'FPC_SMALL_CONTEXT\\n'\r")
    terminal.wait_for("FPC_SMALL_CONTEXT\n", line=True)
    terminal.send("fpc \t")
    terminal.wait_screen(lambda rows: any("Enter insert" in row for row in rows))
    rows = terminal.screen.display
    assert any("FPC_SMALL_CONTEXT" in row for row in rows), rows
    assert (
        sum(bool(row.strip()) and row.startswith(("  ", "› ")) for row in rows) == 5
    ), rows
    terminal.send("\x1b[B" * 6)
    terminal.wait_screen(lambda rows: any("7 /" in row for row in rows))
    terminal.resize(80, 12)
    terminal.wait_screen(lambda rows: not any("Enter insert" in row for row in rows))
    terminal.clear_line()
    terminal.resize(120, 40)


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
        if shell == "fish":
            (work / "line\nbreak.txt").touch()
            (work / ("界" * 40 + ".txt")).touch()
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
            "TERM": "xterm-256color",
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
                f"function fish_prompt; printf '%s' {shlex.quote(PROMPT)}; end; set -g fish_greeting; set -g fish_autosuggestion_enabled 0; function fpc_test_tab; set -g fpc_test_tab_used yes; commandline -f complete; end; bind \\t fpc_test_tab; COMPLETE=fish fpc | source",
            ]
        terminal = Terminal(command, environment, work)
        try:

            def menu_for(prefix):
                return terminal.menu(prefix)

            menu = menu_for("")
            assert ("README.md" if shell == "fish" else "sample.txt") in menu, (
                shell,
                menu,
            )
            assert "folder" in menu, (shell, menu)
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
            assert ".readme-secret" not in readme, (shell, readme)
            substring = menu_for("zh")
            assert "README.zh-CN.md" in substring, (shell, substring)
            nonconsecutive = menu_for("rdm")
            assert "README.md" in nonconsecutive, (shell, nonconsecutive)
            joined = menu_for("readmemd")
            assert "README.md" in joined, (shell, joined)
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
            if shell == "fish":
                assert "README.md" in rules, (shell, rules)
            else:
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
            empty_menu = menu_for("")
            assert (
                "--help" not in empty_menu and "--respect-ignore" not in empty_menu
            ), (shell, empty_menu)
            terminal.send(f"cd {shlex.quote(str(work))}\r")
            terminal.wait_for(PROMPT)
            terminal.send("fpc --install-completion fi\t\r")
            destination = str(home / "config/fish/completions/fpc.fish")
            output = terminal.wait_for(destination + "\n")
            if PROMPT not in output.split(destination + "\n", 1)[1]:
                terminal.wait_for(PROMPT)
            for command in [
                "fpc --print --re\t ./sample.txt\r",
                "fpc --print --ignore-file sam\t ./sample.txt\r",
                "fpc --print --ignore-file=sam\t ./sample.txt\r",
            ]:
                terminal.send(command)
                expected = str(work / "sample.txt")
                output = terminal.wait_for(expected + "\n", line=True)
                assert expected in output.splitlines(), (shell, command, output)
                if PROMPT not in output.rsplit(expected + "\n", 1)[1]:
                    terminal.wait_for(PROMPT)
            for typed, expected in [
                ("sam", work / "sample.txt"),
                ("zhcn", work / "README.zh-CN.md"),
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
                output = terminal.wait_for(str(expected) + "\n", line=True)
                assert str(expected) in output.splitlines(), (shell, typed, output)
                if PROMPT not in output.rsplit(str(expected) + "\n", 1)[1]:
                    terminal.wait_for(PROMPT)
            if shell == "fish":
                terminal.send("fpc --print readmemd\t")
                terminal.wait_screen(
                    lambda rows: any("Enter insert" in row for row in rows)
                )
                terminal.send("\r")
                terminal.wait_screen(
                    lambda rows: not any("Enter insert" in row for row in rows)
                )
                terminal.send("\r")
                expected = str(work / "README.md")
                output = terminal.wait_for(expected + "\n", line=True)
                assert expected in output.splitlines(), (shell, output)
                if PROMPT not in output.rsplit(expected + "\n", 1)[1]:
                    terminal.wait_for(PROMPT)
                for command, expected in [
                    (
                        "fpc --print lineb\t >/dev/null; and printf 'FPC_NEWLINE_OK\\n'\r",
                        "FPC_NEWLINE_OK\n",
                    ),
                    ("printf 'FPC_NON_FPC=%s\\n' sam\t\r", "FPC_NON_FPC=sample.txt\n"),
                    (
                        "printf 'FPC_ORIGINAL_TAB=%s\\n' $fpc_test_tab_used\r",
                        "FPC_ORIGINAL_TAB=yes\n",
                    ),
                ]:
                    terminal.send(command)
                    output = terminal.wait_for(expected)
                    if PROMPT not in output.split(expected, 1)[1]:
                        terminal.wait_for(PROMPT)
                verify_fish_inline_menu(terminal)
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
