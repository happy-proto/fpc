# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fpc.svg)](https://crates.io/crates/fpc)

[English](README.md)

在终端里找到文件，复制它的路径。

给 AI 助手或同事指出一个文件时，运行 `fpc`，输入几个字符，按下回车，它的绝对路径就到了剪贴板里。

```sh
fpc                  # 在当前目录及其子目录中选择文件或目录
fpc src/main.rs      # 已经知道路径时，直接复制
```

默认包含隐藏文件和被 Git 忽略的路径，`.env` 和构建产物也能找到。本地终端、SSH 和 tmux 中都可以使用同一个命令。

## 安装

使用 [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 安装最新正式版本：

```sh
cargo binstall fpc
```

## Shell 补全

为 Bash、Fish 或 Zsh 启用 Tab 补全：

```sh
fpc --install-completion
```

更多选项见 `fpc --help`，包括将路径输出到终端而不复制到剪贴板。

[参与贡献](CONTRIBUTING.md) · [变更记录](CHANGELOG.md) · [MIT 许可证](LICENSE)
