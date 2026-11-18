# Drahtseilakt

[English](README.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · **简体中文**

*Drahtseilakt*（德语，意为“走钢丝”）是德国人对一切走错一步就会坠落的平衡表演的称呼。Ballance（平衡球）
由德国小工作室 Cyparade 于 2004 年制作，讲的正是这件事：一个球在深渊之上的轨道上。这个名字是我们向这款游戏
以及制作它的人们致敬。

Drahtseilakt 是 **Ballance**（Cyparade / Atari，2004）的一个新引擎，用 Rust 从零写成。它在 Linux、Windows 和 macOS 上
原生运行，支持任意分辨率和宽高比，最高 4K，自带界面、部件和声音，因此装好就能玩。它可以运行你自己游戏拷贝中的原版
关卡（包括第 13 关）、社区地图（`.nmo` 文件及其 Virtools 脚本），以及自家开放、有文档的[关卡格式](docs/zh/drahtseilakt/open-format.md)
的关卡。游戏随附 13 张保持作者原样的社区地图，另有 13 张精选地图（Drahtseilakt 套装，单独下载），都标明了作者
（[CREDITS.md](CREDITS.md)）。原版游戏的任何内容我们都绝不发布；你想要的，由 toolbox 从你自己的拷贝中制作。

本项目最初是 emdzej 的 [OpenBallance](https://github.com/emdzej/openballance) 的一个分支。它的历史记录和逆向工程笔记
（`docs/`）得到保留，物理部分已移植到 Rust，其余是新代码（原因见[决策](docs/zh/drahtseilakt/decisions.md)）。这款游戏
为什么需要第二次生命：[历史](docs/zh/drahtseilakt/history.md)。

**状态：** 开发中，版本 0.1.0，尚未公开发布。引擎、菜单、声音、物理、回放、社区地图的脚本、开放格式和打包都已就绪；发布前
还差什么，见[路线图](docs/zh/drahtseilakt/roadmap.md)中的“发布标准”。发布只由所有者进行；在此之前，下面的下载只是草稿或 CI 构建。

## 下载

每次发布都包含 Windows、Linux 和 macOS 的安装程序和便携压缩包（FreeBSD 为实验性）、Linux 的 `.deb`/`.rpm` 软件包，
[toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox) 包含在便携包和安装程序中。13 张精选地图是单独的下载，
很少变化。文件名保留版本号。最新发布：`https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest`；脚本通过 API 获取（`GET https://api.github.com/repos/eugene-vereshchagin/drahtseilakt/releases/latest`，再在 `assets[]` 中选 `name` 符合如 `drahtseilakt-*-windows-x86_64-setup.exe` 的文件，用它的 `browser_download_url`）。哪个文件给谁用、为什么这样拆分下载：[distribution.md](docs/zh/drahtseilakt/distribution.md)；
确切的文件列表和安装命令：[release.md](docs/zh/drahtseilakt/release.md)。

## 构建与游玩

需要 Rust（stable，1.89 或更新版本）和 cargo；在 Linux 上需要 Vulkan 驱动，以及手柄所需的 udev 头文件（Debian/Ubuntu 上为
`libudev-dev`，Fedora 上为 `systemd-devel`）。构建可离线进行：依赖已固定并放在仓库中。

```sh
cargo build --release
target/release/drahtseilakt
```

在检出的源码目录中，游戏会在 `assets/` 里找到基础数据。它以主菜单启动；`--level N` 游玩原版第 N 关，`--map FILE.nmo`
游玩任意关卡文件或社区地图，`--open DIR` 游玩开放格式的关卡，`--replay FILE` 播放录制的一局；`drahtseilakt --help` 列出全部
选项。方向键相对于镜头滚动小球，Shift+左/右转动镜头，空格键让球跳起，F11 全屏，Esc 暂停，F1 打开手册；也支持手柄，
Options → Controls 可绑定其他按键。菜单、模式和选项：[菜单](docs/zh/drahtseilakt/menu.md)；游戏的文件放在哪里（便携或安装）：
[文件](docs/zh/drahtseilakt/files.md)；关卡放在哪里、如何制作和分享：[关卡](docs/zh/drahtseilakt/levels.md)、
[制作关卡](docs/zh/drahtseilakt/making-levels.md)。在 Windows 上，发布版不会打开控制台窗口；消息写入数据文件夹里的 `drahtseilakt.log`。

启动游戏不需要 Ballance 拷贝。要玩原版关卡，请让 toolbox 指向你已安装的游戏或光盘镜像（或运行 `drahtseilakt --import DIR`）；
之后原版的关卡、声音和外观就会出现在游戏中。

## toolbox

`drahtseilakt-toolbox` 是独立仓库
[eugene-vereshchagin/drahtseilakt-toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox) 中的独立程序，有自己的发布。
它在你的电脑上准备关卡：导入你的 Ballance 拷贝，重制原版关卡（形状更圆润、纹理放大数倍；若已安装 Real-ESRGAN 则用它，
否则用普通缩放），转换下载的社区地图并检查它们能否游玩。游戏只负责游玩，从不运行其他程序。要从源码构建两者，请把 toolbox
克隆到本仓库旁边（它通过本仓库的 crate 读取关卡）。面向玩家的说明见[手册](docs/manual/zh/toolbox.md)。

## 文档

文档有英文、俄文、德文和简体中文版本（`docs/`、`docs/ru/`、`docs/de/`、`docs/zh/`），也有用 VitePress 构建的网站（`docs/.vitepress`）。
请从[路线图](docs/zh/drahtseilakt/roadmap.md)（计划，以及“完成”的含义）和[决策](docs/zh/drahtseilakt/decisions.md)（为什么）读起。
玩家手册在 `docs/manual/<语言>/`，随游戏发布。测试方面有无窗口的回放检查、由测试播放的录制通关
和对每个解析器的模糊测试（[模糊测试](docs/zh/drahtseilakt/fuzzing.md)）。
贡献者须知见 [distribution.md](docs/zh/drahtseilakt/distribution.md) 和 `CLAUDE.md`。

## 谁做的

代码、文档以及生成的声音和音乐由 **Claude**（Anthropic 的 AI 模型，在 Claude Code 中工作）编写。**Veresk** 是项目的所有者：
想法、决策、计划、试玩，以及判断什么才对的耳朵。提交记录中带有两个名字。

## 许可与致谢

从 OpenBallance（GPL-3.0，emdzej 及贡献者）移植的文件在文件头中注明了这一点；`ivp` crate 是物理部分的移植。
对 OpenBallance 和 emdzej 的致谢随本仓库的历史记录保留。

下载包中所有不是为 Drahtseilakt 编写的内容及其许可证（上游代码、字体、纹理来源、声音和音乐、随附的社区关卡及其作者、编译进程序的
每个 crate）都列在 [THIRD-PARTY.md](THIRD-PARTY.md) 中，它与 `LICENSE` 放在一起；地图及其作者见 [CREDITS.md](CREDITS.md)。
`tools/third_party.py` 可离线根据 `Cargo.lock` 重新生成 crate 列表，若有依赖的许可证与 GPL-3.0 不兼容，测试会失败。

Drahtseilakt 采用 GPL-3.0 许可，与它起源的 OpenBallance 相同。它绝不发布 Ballance 的数据，启动也不需要游戏拷贝。
（在本仓库保持私有期间，`levels/` 中保存着原版关卡的备份用于开发；公开仓库不会包含它。）社区地图归其作者所有；
若作者提出异议，我们会移除该地图。Ballance © 2004 Cyparade / Atari。本项目与他们没有关联。
