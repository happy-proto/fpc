#![cfg(unix)]

use std::{
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::Path,
    process::Command,
};

fn cli(root: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fpc"));
    cmd.env_clear()
        .env("HOME", root)
        .env("PATH", "/usr/bin:/bin")
        .current_dir(root);
    cmd
}

fn stub(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn direct_paths_handle_spaces_newlines_symlinks_and_leading_dashes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let name = "-a b\n中文";
    let path = root.join(name);
    fs::write(&path, "").unwrap();
    std::os::unix::fs::symlink(&path, root.join("link")).unwrap();
    let mut expected = path.canonicalize().unwrap().as_os_str().as_bytes().to_vec();
    expected.push(b'\n');
    for args in [vec!["--print", "--", name], vec!["--print", "link"]] {
        let output = cli(root).args(args).output().unwrap();
        assert!(output.status.success(), "{:?}", output);
        assert_eq!(output.stdout, expected);
    }
    let invalid = cli(root).args(["--print", "missing"]).output().unwrap();
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
}

#[test]
fn tab_completion_only_offers_paths_without_a_current_directory_entry() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for name in ["sample.txt", "--report", ".env"] {
        fs::write(root.join(name), "").unwrap();
    }
    fs::create_dir(root.join("folder")).unwrap();
    fs::write(root.join("folder/nested.txt"), "").unwrap();
    for shell in ["fish", "bash", "zsh"] {
        for (prefix, expected) in [
            ("", "sample.txt"),
            ("-", "--report"),
            (".", ".env"),
            ("folder/", "folder/nested.txt"),
        ] {
            let output = cli(root)
                .env("COMPLETE", shell)
                .env("_CLAP_COMPLETE_INDEX", "1")
                .env("_CLAP_IFS", "\n")
                .args(["--", "fpc", prefix])
                .output()
                .unwrap();
            assert!(output.status.success(), "{shell}: {output:?}");
            let results = String::from_utf8(output.stdout).unwrap();
            assert!(
                results.contains(expected),
                "{shell}, {prefix:?}: {results:?}"
            );
            for result in results.lines() {
                let value = result.split(['\t', ':']).next().unwrap().trim_end();
                assert!(
                    ![
                        ".",
                        "./",
                        "--help",
                        "--version",
                        "--print",
                        "--respect-ignore",
                        "--ignore-file",
                        "--install-completion",
                        "-h",
                        "-V"
                    ]
                    .contains(&value),
                    "{shell}, {prefix:?}: {results:?}"
                );
            }
        }
    }
    assert!(cli(root).arg("--help").output().unwrap().status.success());
}

#[test]
fn completion_registration_never_runs_copy_or_picker() {
    let temp = tempfile::tempdir().unwrap();
    for shell in ["fish", "bash", "zsh"] {
        let output = cli(temp.path()).env("COMPLETE", shell).output().unwrap();
        assert!(output.status.success(), "{:?}", output);
        let script = String::from_utf8(output.stdout).unwrap();
        assert!(script.contains("fpc"));
        assert!(script.contains("COMPLETE"));
        assert!(!script.contains("Copied absolute path"));
    }
}

#[test]
fn completion_install_is_idempotent_and_respects_config_locations() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let config = root.join("config");
    let data = root.join("data");
    let system = root.join("system");
    fs::create_dir_all(system.join("bash-completion")).unwrap();
    fs::write(system.join("bash-completion/bash_completion"), "# fixture").unwrap();
    let zdot = root.join("zsh");
    fs::create_dir(&zdot).unwrap();
    fs::write(zdot.join(".zshrc"), "# existing user config\n").unwrap();
    fs::write(root.join(".bashrc"), "# existing bash config\n").unwrap();
    for shell in ["fish", "bash", "zsh"] {
        let install = || {
            cli(root)
                .env("XDG_CONFIG_HOME", &config)
                .env("XDG_DATA_HOME", &data)
                .env("XDG_DATA_DIRS", &system)
                .env("ZDOTDIR", &zdot)
                .args(["--install-completion", shell])
                .output()
                .unwrap()
        };
        assert!(install().status.success());
        let path = match shell {
            "fish" => config.join("fish/completions/fpc.fish"),
            "zsh" => data.join("zsh/site-functions/_fpc"),
            _ => data.join("bash-completion/completions/fpc.bash"),
        };
        let original = fs::read(&path).unwrap();
        assert!(install().status.success());
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::write(&path, "# unrelated completion\n").unwrap();
        assert!(!install().status.success());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# unrelated completion\n"
        );
    }
    assert_eq!(
        fs::read_to_string(zdot.join(".zshrc")).unwrap(),
        "# existing user config\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".bashrc")).unwrap(),
        "# existing bash config\n"
    );
    assert!(
        !cli(root)
            .args(["--install-completion", "fish", "some-file"])
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn bash_install_requires_framework_and_respects_custom_user_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    stub(&root.join("pkg-config"), "exit 1");
    let install = || {
        cli(root)
            .env("PATH", root)
            .env("XDG_DATA_DIRS", root.join("missing"))
            .env("BASH_COMPLETION_USER_DIR", root.join("custom"))
            .args(["--install-completion", "bash"])
            .output()
            .unwrap()
    };
    let missing = install();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("Cannot detect bash-completion"));
    assert!(!root.join("custom").exists());
    assert!(!root.join(".bashrc").exists());
    stub(&root.join("pkg-config"), "printf '%s\\n' \"$HOME/share\"");
    assert!(!install().status.success());
    fs::create_dir_all(root.join("share/bash-completion")).unwrap();
    fs::write(
        root.join("share/bash-completion/bash_completion"),
        "# fixture",
    )
    .unwrap();
    assert!(install().status.success());
    assert!(root.join("custom/completions/fpc.bash").is_file());
    assert!(!root.join(".bashrc").exists());
}

#[test]
fn tmux_routes_to_an_attached_client_and_checks_failures() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("file"), "").unwrap();
    let log = root.join("log");
    stub(
        &root.join("tmux"),
        "case \"$1\" in display-message) printf '$1';; list-clients) printf '/dev/pts/42\\t%%3\\t1\\n/dev/pts/43\\t%%4\\t2\\n';; *) printf '%s\\n' \"$@\" > \"$ARG_LOG\"; /bin/cat > \"$COPY_LOG\";; esac",
    );
    let output = cli(root)
        .env("PATH", root)
        .env("TMUX", "test")
        .env("TMUX_PANE", "%3")
        .env("COPY_LOG", &log)
        .env("ARG_LOG", root.join("args"))
        .arg("file")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        fs::read(log).unwrap(),
        root.join("file")
            .canonicalize()
            .unwrap()
            .as_os_str()
            .as_bytes()
    );
    assert_eq!(
        fs::read_to_string(root.join("args")).unwrap(),
        "load-buffer\n-w\n-t\n/dev/pts/42\n-\n"
    );
    assert!(output.stderr.is_empty(), "{output:?}");
    stub(&root.join("tmux"), "exit 7");
    let failed = cli(root)
        .env("PATH", root)
        .env("TMUX", "test")
        .env("TMUX_PANE", "%3")
        .arg("file")
        .output()
        .unwrap();
    assert!(failed.status.success());
    assert!(!failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Clipboard unavailable"));
}

#[test]
#[cfg(target_os = "macos")]
fn native_copy_has_no_newline() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("file"), "").unwrap();
    stub(&root.join("pbcopy"), "/bin/cat > \"$COPY_LOG\"");
    let output = cli(root)
        .env("PATH", root)
        .env("COPY_LOG", root.join("log"))
        .arg("file")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(
        fs::read(root.join("log")).unwrap(),
        root.join("file")
            .canonicalize()
            .unwrap()
            .as_os_str()
            .as_bytes()
    );
}
