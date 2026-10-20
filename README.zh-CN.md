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
fpc --install-completion     # 自动检测 shell 并安装补全
fpc --help                   # 查看所有选项
```

默认包含隐藏项和被忽略的路径。

补全路径时，Tab 会打开模糊选择器，以当前文件名作为初始查询。例如，`readmemd<Tab>` 和 `rdm<Tab>` 都能找到 `README.md`。可以编辑查询或用方向键选择，按 Enter 将路径填回命令行，不执行命令，也不操作剪贴板。即使只有一个匹配，也需要确认。Esc 或 Ctrl+C 取消并保留原命令行。每次只搜索一个目录层级，选中目录后保留末尾 `/`；查询以 `.` 开头时才显示隐藏名称。参数名和 shell 名称仍使用普通 shell 补全。

升级后运行 `fpc --install-completion`，再打开新 shell。Fish 还会安装一个受管理的 `conf.d/fpc.fish` 文件，确保首次补全前就加载 Tab 绑定；其他命令保留原有的 Tab 绑定。

选择器会高亮命中查询的字符，包括分散在路径中的非连续匹配，并滚动长路径以展示命中区域。输入 `'dcjanus`（只在开头加一个单引号）要求包含连续子串 `dcjanus`；直接输入 `dcjanus` 使用模糊匹配。

贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，发布流程见 [RELEASE.md](RELEASE.md)，变更记录见 [CHANGELOG.md](CHANGELOG.md)。
