# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fpc.svg)](https://crates.io/crates/fpc)
[![Dependencies](https://deps.rs/repo/github/happy-proto/fpc/status.svg)](https://deps.rs/repo/github/happy-proto/fpc)

[English](README.md)

快速找到文件或目录，用一条简短的命令复制绝对路径。

给 Agent 指定文件时，工作区之外或被 `.gitignore` 忽略的路径往往不容易选择。`fpc` 通过内置模糊选择器让这些文件也能方便地找到，支持本地、SSH 和 tmux。

## 安装

使用 [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 安装最新正式版本：

```sh
cargo binstall fpc
```

## 使用方式

```sh
fpc                          # 模糊选择当前目录及其子目录中的文件或目录
fpc ./src/main.rs            # 直接复制路径
fpc --respect-ignore        # 选择遵循 .gitignore 和 .ignore 规则
fpc --print ./src/main.rs    # 输出路径，不操作剪贴板
fpc --install-completion fish # 安装 shell 补全，也支持 bash 和 zsh
fpc --help                   # 查看所有选项
```

默认包含隐藏项和被忽略的路径。

贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，发布流程见 [RELEASE.md](RELEASE.md)，变更记录见 [CHANGELOG.md](CHANGELOG.md)。
