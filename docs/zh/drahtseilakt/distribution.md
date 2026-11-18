# Drahtseilakt 如何分发，什么在哪里

一页回答两个问题：*我该下载什么？* 和 *东西在仓库的哪里？* 构建与上传的步骤见 [release.md](release.md)，
原因见 [decisions.md](decisions.md)（下文有链接）。

## 每个下载包里有什么，为什么这样分

Drahtseilakt 由三部分组成，它们变化的速度不同，所以分开发布：

- **程序**（游戏；便携包和安装程序里还有 toolbox）：每个版本都会变。
- **基础数据**：我们自己的部件、纹理、字体、声音、出厂模组、展示关卡 `00-Showcase`、隐藏的 `Parts_Test`
  和 13 张社区 `.nmo` 地图。它们在 git 中，偶尔变化。
- **13 张精选地图**（Drahtseilakt 套装）：很少变化。每张一个 `.zip`，外加一个 Linux 包 `drahtseilakt-levels`。

这样分的原因：

- **GitHub 的限制。** git 中的文件不能超过 100 MB，发布附件不能超过 2 GiB。精选地图的 `level.bin` 超过 100 MB，
  所以这些地图只作为发布附件提供（合计约 1.1 GB，每个文件都在限制之内）；基础数据（约 100 MB，最大文件 3.6 MB）
  可以放进 git（decisions.md「git 中的二进制文件：基础数据」，见 [此处](decisions.md)）。
- **程序更新保持小巧。** 关卡不随程序变化，所以新版程序只重建二进制包，玩家不必再下载一遍 1 GB。程序的版本是标签
  `X.Y.Z`；数据有自己的版本 `scripts/DATA_VERSION`（一个日期），程序包要求 `drahtseilakt-data >= DATA_VERSION`。
- **默认使用自有内容；原版内容绝不随包发布。** 下载包里只有我们自己制作的内容，或作者允许我们发布的内容（见
  `CREDITS.md`、`THIRD-PARTY.md`）。原版的资源由 toolbox 在玩家自己的电脑上，从玩家自己的拷贝中制作
  （decisions.md「默认使用自有内容，按玩家意愿使用原版内容」）。这既是出于法律上的谨慎，也是项目的内容政策。
- **关卡由 toolbox 准备，游戏只负责游玩。** 导入你的拷贝、重制、转换下载的地图以及 Real-ESRGAN 放大器，都是 toolbox 的
  工作——它是独立仓库里的独立程序；游戏只播放其文件夹中的内容（decisions.md「关卡由 toolbox 准备，游戏只负责游玩」）。
  因此便携包和安装程序同时带有两个程序，Linux 软件包则只有游戏。
- **游戏永不联网。** 它不下载关卡、更新或字体，也不检查新版本；文件由你自己从 GitHub 下载（decisions.md「游戏永不联网」）。
  只有构建过程可以下载，且版本固定并校验校验和。
- **便携或安装。** 便携副本（程序旁有一个 `portable.txt`）把一切都放在自己的文件夹里，不碰其他地方；安装版把玩家数据
  放在用户配置目录，所以卸载不会删除它们（见 [files.md](files.md)）。
- **开发版（dev）** 用于开发，不给玩家：开启上帝模式的调试构建（`drahtseilakt-dev`，名称带 `-dev`）。只在需要时通过
  工作流输入 `dev` 构建，绝不会放进正式发布。

代码中唯一与游戏绑定的关卡是展示关卡 `00-Showcase`；其他关卡都是文件夹中的数据（见 [levels.md](levels.md)）。

## 我想要 X：下载 Y

最新发布通过 GitHub API 查找，而不是固定链接：`GET https://api.github.com/repos/eugene-vereshchagin/drahtseilakt/releases/latest`，然后在 `assets[]` 中选出 `name` 符合模式（如 `drahtseilakt-*-windows-x86_64-setup.exe`）的文件，使用它的 `browser_download_url`。页面 <https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest> 始终显示最新发布。文件名保留版本号（下文 `<version>`；数据包用 `<data-version>`），方便阅读。请用同一发布中的 `SHA256SUMS` 校验下载。

| 我想要 | 下载 |
|---|---|
| 在 Windows 上安装后游玩（按用户安装，无需管理员，`%LOCALAPPDATA%\Programs\Drahtseilakt`） | `drahtseilakt-<version>-windows-x86_64-setup.exe`（ARM：`-aarch64-`） |
| 在 Windows 上免安装游玩 | `drahtseilakt-<version>-windows-x86_64.zip`（ARM：`-aarch64`） |
| 在 Linux 上免安装游玩 | `drahtseilakt-<version>-linux-x86_64.tar.gz`（ARM：`-aarch64`） |
| Debian / Ubuntu 软件包 | `drahtseilakt_<version>-1_amd64.deb` + `drahtseilakt-data_<data-version>-1_all.deb`（ARM：`drahtseilakt_<version>-1_arm64.deb`） |
| Fedora / openSUSE 软件包 | `drahtseilakt-<version>-1.x86_64.rpm` + `drahtseilakt-data-<data-version>-1.noarch.rpm`（ARM：`drahtseilakt-<version>-1.aarch64.rpm`） |
| 在 macOS 上游玩 | `drahtseilakt-<version>-macos-aarch64.dmg`（Apple 芯片）或 `drahtseilakt-<version>-macos-x86_64.dmg`（Intel）；未签名：首次启动需右键点击并选“打开”。同名的 `.tar.gz` 是便携形式 |
| FreeBSD（实验性） | `drahtseilakt-<version>-freebsd-x86_64.tar.gz` |
| toolbox | 包含在每个便携包和安装程序里；或到它自己的发布页：[drahtseilakt-toolbox](https://github.com/eugene-vereshchagin/drahtseilakt-toolbox/releases) |
| 13 张精选地图 | 数据发布 `data-<DATA_VERSION>`：每张地图一个 `.zip`（解压到 `assets/levels/drahtseilakt/`），Linux 上可用 `drahtseilakt-levels`（`.deb`/`.rpm`） |
| 原版 Ballance 的关卡 | 我们不提供下载：toolbox 会从你自己的拷贝中导入 |
| 试用最新的未发布代码 | 自己构建（见 [README](https://github.com/eugene-vereshchagin/drahtseilakt/blob/main/README.zh.md)），或取工作流运行的产物（`git-<sha>` 构建，软件包版本 `0~git<sha>`） |

便携包和安装程序包含基础数据，可直接游玩；Linux 软件包需要数据包与程序一起安装
（`apt install ./drahtseilakt-data_*_all.deb ./drahtseilakt_*_amd64.deb`）。一次发布的完整文件列表和安装命令见 [release.md](release.md)。

## 东西放在哪里

| 位置 | 内容 |
|---|---|
| `eugene-vereshchagin/drahtseilakt`（本仓库） | 游戏、引擎 crate（`crates/`：`nmo`、`level`、`gamedata`、物理 `ivp`、`script`、`game` 等）、`assets/`（基础数据）、`docs/`、`scripts/`、`tools/`、`hints/`（自动驾驶计划）、`tests/` |
| `eugene-vereshchagin/drahtseilakt-toolbox` | toolbox，有自己的发布和 CI；它通过本仓库的 `crates/gamedata` 和 `crates/level` 读取关卡 |
| `assets/` | 随游戏发布的内容：我们的部件、字体、声音、出厂模组、`levels/community/` 和 `levels/drahtseilakt/`（后者在 git 中只有精选地图的 `level.txt`，其文件是发布附件） |
| `docs/drahtseilakt/` | 项目文档（路线图、决策、格式、菜单、模组）；每页在 `docs/ru/`、`docs/de/`、`docs/zh/` 中也有对应版本 |
| `docs/manual/<语言>/` | 玩家手册；`tools/make_manual.py` 将其生成到 `assets/manual`（不在 git 中，由 CI 生成：`scripts/ci-assets.sh`），游戏按 F1 打开 |
| `scripts/` | 发布打包（`ci-package.sh`、`make-distro-packages.sh`、`pack-assets.sh`、`DATA_VERSION`） |
| `.github/workflows/` | CI：测试（`drahtseilakt.yml`）和发布（`drahtseilakt-release.yml`）；都按需运行 |
| `docs/internals/`、根目录的 `docs/*.md` | 上游 OpenBallance 的笔记和逆向工程资料，作为参考保留 |

## 贡献者须遵守的规则

- **绝不使用原版资源**：Ballance 的任何文件、纹理、声音或模型都不得出现在 git、软件包、测试或测试数据中。需要游戏数据的
  测试从 `OPENBALLANCE_DATA` 读取；下载包里没有任何东西来自原版。
- **固定依赖**：crate 由 `Cargo.lock` 锁定（已 vendor，构建可离线进行），Actions 按完整提交 SHA 固定，工具和下载的二进制文件
  按版本和 SHA-256 固定。添加依赖前须确认它存在、仍在维护，且许可证与 GPL-3.0 兼容（`tools/third_party.py` 和一项测试把关）。
- **四种语言**：文档的修改须在同一次提交中同步到英文、俄文、德文和简体中文；新页面要在每个语言版本的侧边栏中添加条目
  （`docs/.vitepress/config.ts`）。菜单文字必须在所有语言中放得下。
- **CI 按需运行**，以及在 pull request 上运行，而不是每次 push 都运行：请把修改攒成一批。
- **游戏内不联网**；只有构建过程可以下载。
- **发布只由所有者进行**：未经他明确下令，任何人不得打标签、发布或上传数据发布。标签 `X.Y.Z` 会生成草稿，由他来发布。

详情见仓库根目录的 `CLAUDE.md`。
