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

路径 Tab 补全优先匹配忽略大小写的前缀，没有前缀结果时再使用 nucleo 模糊匹配，因此 `readmemd<Tab>` 和 `rdm<Tab>` 都能找到 `README.md`。每次只搜索一个目录层级，输入以 `.` 开头时才显示隐藏名称。Tab 只填入路径，不操作剪贴板。

在 Fish 中，多个路径候选会在命令行下方打开紧凑菜单，每次最多显示五项，不进入 alt screen。使用 ↑/↓ 或 Tab/Shift+Tab 选择，Enter 填入。Esc 或 Ctrl+C 取消并保留原输入；需要修改查询时，取消菜单后继续在 shell 中输入。调整终端大小会关闭菜单。唯一候选直接填入。菜单完整使用 fpc 的候选和排序，不经过 Fish 筛选，也不改写查询的大小写。参数名和 shell 名称保留普通补全。`--install-completion fish` 会安装受管理的 `conf.d/fpc.fish` 片段，在启动时加载 Tab 适配；其他命令保持已有的 Tab 行为。

Bash 和 Zsh 使用原生补全菜单，fpc 返回的有序候选最终如何展示由 shell 决定。

选择器会高亮命中查询的字符，包括分散在路径中的非连续匹配，并滚动长路径以展示命中区域。输入 `'dcjanus`（只在开头加一个单引号）要求包含连续子串 `dcjanus`；直接输入 `dcjanus` 使用模糊匹配。

贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，发布流程见 [RELEASE.md](RELEASE.md)，变更记录见 [CHANGELOG.md](CHANGELOG.md)。
