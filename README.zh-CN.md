# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fpc.svg)](https://crates.io/crates/fpc)

[English](README.md)

`fpc` 用于查找文件或目录，并将其绝对路径复制到剪贴板。

做 AI coding 时，常常需要把文件的绝对路径传给 Agent，尤其是工作区之外或被 `.gitignore` 忽略、不方便从编辑器中选择的文件。`fpc` 提供一个终端内的模糊选择器，也可以直接复制指定路径。

在 SSH 会话中运行时，可以把远端文件的绝对路径复制到本地剪贴板，也支持在 tmux 中使用。

## 安装

使用 [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 安装最新正式版本：

```sh
cargo binstall fpc
```

## 使用方式

```sh
fpc                    # 查找并复制文件或目录的路径
fpc ./src/main.rs      # 复制指定文件的绝对路径
```

选择器搜索当前目录及其子目录，默认包含隐藏项和被 Git 忽略的路径。

运行 `fpc --install-completion` 可启用 Bash、Fish 或 Zsh 的 Tab 补全，全部选项见 `fpc --help`。

[参与贡献](CONTRIBUTING.md) · [变更记录](CHANGELOG.md) · [MIT 许可证](LICENSE)
