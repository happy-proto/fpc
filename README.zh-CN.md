# fpc

[![CI](https://github.com/happy-proto/fpc/actions/workflows/ci.yml/badge.svg)](https://github.com/happy-proto/fpc/actions/workflows/ci.yml)
[![Dependencies](https://deps.rs/repo/github/happy-proto/fpc/status.svg)](https://deps.rs/repo/github/happy-proto/fpc)

[English](README.md)

快速找到文件或目录，用一条简短的命令复制绝对路径。

给 Agent 指定文件时，工作区之外或被 `.gitignore` 忽略的路径往往不容易选择。`fpc` 让这些文件也能方便地找到，支持本地、SSH 和 tmux。使用 Rust 编写，内置模糊选择器。

## 使用方式

```sh
fpc                          # 模糊选择当前目录及其子目录中的文件或目录
fpc ./src/main.rs            # 直接复制路径
fpc --respect-ignore        # 选择遵循 .gitignore 和 .ignore 规则
fpc --ignore-file ./rules    # 使用自定义的 Git 风格 ignore 文件
fpc --print ./src/main.rs    # 输出路径，不操作剪贴板
fpc --install-completion fish # 安装动态 Tab 补全，也支持 bash 和 zsh
```

默认包含隐藏项和被忽略的路径。隐藏路径略微降权，显式搜索隐藏项时取消降权，高度相关的结果仍容易选中。Tab 只补全当前路径层级中的文件和目录，不展示命令选项或单独的 `.`，递归搜索交给交互式选择器。

选择器中直接输入查询，用方向键或 Tab 选择，Enter 复制，Esc 取消。路径处理与 `realpath` 一致，会解析符号链接。

补全需要主动执行安装命令，只写独立文件，不修改 shell 启动配置。Fish 会自动加载；Bash 需要已安装并启用 `bash-completion`，检测不到时安装命令会报错退出。Zsh 需要在 `compinit` 前把 `${XDG_DATA_HOME:-$HOME/.local/share}/zsh/site-functions` 加入 `fpath`。安装命令会提示文件位置，升级程序通常无需重新安装补全。

本地使用系统剪贴板，SSH 通过终端剪贴板支持（OSC 52），tmux 通过自身的剪贴板通道处理。复制成功时保持静默。远端复制需要终端支持；剪贴板通道不可用时显示提示并输出路径，方便手动复制。

## 本地构建

```sh
cargo install --path . --locked
```

贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，发布流程见 [RELEASE.md](RELEASE.md)，变更记录见 [CHANGELOG.md](CHANGELOG.md)。
