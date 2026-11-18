# 发布与软件包

一次发布由三部分组成，它们变化的速度不同，所以分开构建、分开发布：更新程序只会重新构建二进制包。

| 内容 | 变化频率 | 构建者 | 发布形式 |
|---|---|---|---|
| **二进制包**：程序，以及作为 `.deb`/`.rpm` 的基础数据 | 每个版本 | 工作流 `Drahtseilakt release`（CI） | 标签为 `X.Y.Z` 的发布（草稿） |
| **关卡**：13 张精选地图 | 很少 | `scripts/pack-assets.sh`（所有者的机器） | 标签为 `data-<DATA_VERSION>` 的发布 |

**基础数据在 git 中**（所有者 2026-11-17 的决定，decisions.md「git 中的二进制文件：基础数据」），因此 CI 直接从检出中取用，
不下载任何东西；数据包 `drahtseilakt-data`（`.deb`/`.rpm`）在发布工作流中由它构建。只有 13 张精选地图不在 git 中
（那里的 `level.bin` 超过 GitHub 单文件 100 MB 的限制）；它们在所有者的机器上由本地 `assets/` 制作，手动上传。

## 基础数据

不含精选地图（`assets/levels/drahtseilakt/NN-*`，其 `level.txt` 留在 git 中）和仅供开发的文件（`mods-dev`、`own/make`）的
`assets/`：我们自己的部件和纹理、字体、声音、出厂模组、演示关卡（`00-Showcase`、`Parts_Test`）以及游戏所依赖的 13 张社区
`.nmo` 地图（`assets/levels/community`）。在 git 中约 100 MB（355 个文件，最大 3.6 MB），打包后 93 MB。绝不包含任何原版
Ballance 文件（加入时已与原版副本逐一比对哈希）。不放进 git 的是 CI 能轻易生成的内容：玩家手册（`assets/manual`，
`tools/make_manual.py`，由 `scripts/ci-assets.sh` 运行）。`.gitignore` 忽略精选地图的文件（`level.txt` 除外）以及派生的
`.dtx`、`level.source`、`*.remaster/`、`textures.original/`。

## 关卡

13 张精选地图，每张一个 `.zip`（29-204 MB，共 1.1 GB），同样的地图也打成一个包 `drahtseilakt-levels`（安装后 2.8 GB，压缩后 1.1 GB）。每个文件都低于 GitHub 的 2 GiB 限制；超过的包脚本会拒绝生成。

## 版本

- `scripts/DATA_VERSION` 是日期 `YYYYMMDD`（如 `20261117`）。它既是数据的版本，也是程序包要求的版本：程序包依赖 `drahtseilakt-data (>= DATA_VERSION)`。
- 数据或关卡变化时更新它（并提交），然后打包，再上传。如果程序发布需要比最新数据发布更新的数据，必须先发布数据。
- 程序版本就是标签 `X.Y.Z`。没有标签的工作流运行在包中的版本是 `0~git<sha>`。
- 数据发布的标签是 `data-<DATA_VERSION>`；工作流的标签过滤器（`[0-9]+.[0-9]+.[0-9]+`）不匹配它，因此不会触发任何运行。

## 制作关卡包

```sh
scripts/pack-assets.sh          # OUT 默认为 ../drahtseilakt-portable/bundles
```

在 OUT 中生成（`V` = `scripts/DATA_VERSION`）：

| 文件 | 用途 |
|---|---|
| `levels/NN-Name.zip`（13 个） | 所有系统：解压到 `assets/levels/drahtseilakt/` |
| `drahtseilakt-levels_V-1_all.deb`、`drahtseilakt-levels-V-1.noarch.rpm` | Linux，全部 13 张地图（依赖数据包） |
| `SHA256SUMS` | 以上所有文件 |

`--with-data` 还会由本地 `assets/` 生成旧式的 `drahtseilakt-data-V.tar.gz` 和数据包，用于本地检查软件包；它们不上传：
CI 从 git 构建 `drahtseilakt-data`（`scripts/make-distro-packages.sh data --out DIR`，取自检出；`--tar FILE` 则改用压缩包）。

`.deb` 和 `.rpm` 由 [nfpm](https://nfpm.goreleaser.com) 2.49.0 生成，一个工具通吃，因此同一脚本既能在所有者的机器上运行
（无需 `rpmbuild`），也能在 CI 中运行。`scripts/fetch-nfpm.sh` 把它下载到 `third_party/`（已加入 gitignore；绝不装入系统路径），
并校验脚本中固定的 SHA-256。`scripts/pack-assets.sh --no-packages` 跳过软件包。

## 上传关卡发布

由所有者在检出目录中运行；这里没有任何东西会自动执行：

```sh
V=$(cat scripts/DATA_VERSION)
cd ../drahtseilakt-portable/bundles && sha256sum --check --quiet SHA256SUMS
gh release create "data-$V" --repo eugene-vereshchagin/drahtseilakt --draft --latest=false \
  --title "Drahtseilakt levels $V" \
  --notes "The 13 picked maps for Drahtseilakt (data version $V). Programs and base data: the releases X.Y.Z." \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

查看草稿后再发布。其余无需设置：CI 不需要变量、密钥，也不下载数据。

## 二进制发布（CI）

Actions -> “Drahtseilakt release” -> Run workflow（文件是运行产物），或推送标签 `X.Y.Z`：该运行还会创建一个包含下列文件和 `SHA256SUMS` 的**草稿**发布（不含数据和关卡）。输入项 `dev` 会加上开发版（绝不进入发布）。推送到分支不会触发任何运行。

**发布文件名保留版本号**（便于阅读）：按标签发布时，下表文件原样发布，例如 `drahtseilakt-0.1.0-windows-x86_64-setup.exe`、`drahtseilakt_0.1.0-1_amd64.deb`。脚本或网站通过 API 取最新发布（`GET https://api.github.com/repos/eugene-vereshchagin/drahtseilakt/releases/latest`），在 `assets[]` 中选出 `name` 符合模式（如 `drahtseilakt-*-windows-x86_64-setup.exe`）的文件，使用它的 `browser_download_url`；在浏览器里 <https://github.com/eugene-vereshchagin/drahtseilakt/releases/latest> 显示最新发布。精选地图在标签为 `data-<DATA_VERSION>` 的发布里，不在 “latest” 中。哪个文件给谁：[distribution.md](distribution.md)。

`<V>` 为标签（不在标签上时为 `git-<sha>`），生成这些文件：

| 目标 | 文件 |
|---|---|
| Windows x86_64, aarch64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` （安装程序）、`drahtseilakt-<V>-windows-<arch>.zip`（便携版） |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` （便携版）、`drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux（x86_64 任务，一次） | `drahtseilakt-data_<DATA_VERSION>-1_all.deb`、`drahtseilakt-data-<DATA_VERSION>-1.noarch.rpm` |
| Linux x86_64 DEV（输入项 `dev`） | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` （未签名的 `.app`）、`drahtseilakt-<V>-macos-<arch>.tar.gz`（便携版） |
| FreeBSD x86_64（实验性） | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **便携压缩包**和 **Windows 安装程序**包含基础数据，因此可独立运行；精选地图是数据发布（`data-<DATA_VERSION>`）中单独的 `.zip` 文件。便携压缩包还带有工具箱。
- **Windows 安装程序**（NSIS，在 Ubuntu 上由便携 zip 构建）：默认按用户安装（`%LOCALAPPDATA%\Programs\Drahtseilakt`，无需管理员权限），带开始菜单文件夹，并在“应用”中提供卸载程序。它不安装 `portable.txt`，所以玩家数据存放在用户配置目录，卸载绝不会动它。安装程序、卸载程序和快捷方式带有我们的图标（`scripts/packaging/icon.ico`，安装在程序旁）。未签名：SmartScreen 会警告。**尚未做到：**`drahtseilakt.exe` 自身内嵌的图标（资源管理器、便携版的任务栏）：这需要一个嵌入资源的构建依赖，离线 crate 缓存中没有；窗口本身有图标（运行时设置）。
- **便携副本**（程序旁有 `portable.txt`）只在自己的 `assets/` 中查找资源，绝不查找源码树、`/usr/share` 或 `XDG_DATA_DIRS`。
- **Linux 软件包**：程序在 `/usr/bin/drahtseilakt`（依赖 `libasound2`、`libudev1`；.rpm 依赖的是 soname：`libasound.so.2`、`libudev.so.1`、`libc.so.6(GLIBC_2.35)`，Fedora 和 openSUSE 都提供，不像 `systemd-libs` 这类包名；推荐游戏运行时加载的窗口系统和图形库：`libvulkan1`、`libxkbcommon-x11-0`、`libx11-6`、`libxcursor1`、`libxi6`、`libwayland-client0`），数据在游戏查找的位置 `/usr/share/drahtseilakt/assets`（程序先从 `/usr/bin/../share` 查找，再查 `$XDG_DATA_DIRS`：见 `files.md`）。还有 `.desktop` 文件和图标（`scripts/packaging/icon-512.png`，我们的木球，与网站和窗口用的是同一张图；512 px，另有经 `tools/icon.py` 缩放的 256 px 副本）。`drahtseilakt` 依赖 `drahtseilakt-data (>= DATA_VERSION)`；`drahtseilakt-levels` 是可选的。工具箱不在其中（仅便携版）。`drahtseilakt-dev` 提供并与 `drahtseilakt` 冲突（仅 `.deb`，来自输入项 `dev`）。
- **macOS**：`.app` 把数据放在 `Contents/Resources/assets`；未签名也未公证，首次启动需要右键 -> 打开。

在 Debian/Ubuntu 上安装： `sudo apt install ./drahtseilakt-data_V-1_all.deb ./drahtseilakt_X.Y.Z-1_amd64.deb`
（以及 `./drahtseilakt-levels_V-1_all.deb`）；Fedora： `sudo dnf install ./drahtseilakt-data-V-1.noarch.rpm ./drahtseilakt-X.Y.Z-1.x86_64.rpm`.

## 版本固定与权限

Actions 按完整提交 SHA 固定，Rust 工具链按精确版本固定，crate 由 `Cargo.lock` 固定，nfpm 和 Real-ESRGAN 按 SHA-256 固定；工作流只有 `contents: read`，仅 release 任务有 `contents: write`。NSIS 来自 Ubuntu 24.04 软件包仓库（由发行版签名，未按哈希固定；版本打印在日志中）。
