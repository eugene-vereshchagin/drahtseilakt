# 发布与软件包

一次发布由三部分组成，它们变化的速度不同，所以分开构建、分开发布：更新程序只会重新构建二进制包。

| 内容 | 变化频率 | 构建者 | 发布形式 |
|---|---|---|---|
| **二进制包**：程序 | 每个版本 | 工作流 `Drahtseilakt release`（CI） | 标签为 `X.Y.Z` 的发布（草稿） |
| **基础数据**：不含 13 张精选地图的 `assets/` | 很少 | `scripts/pack-assets.sh`（所有者的机器） | 标签为 `data-<DATA_VERSION>` 的发布 |
| **关卡**：13 张精选地图 | 很少 | `scripts/pack-assets.sh`（所有者的机器） | 同一个 `data-<DATA_VERSION>` 发布 |

数据和关卡不在 git 中（“git 中没有二进制文件”），因此无法在 CI 中构建；它们在所有者的机器上由本地 `assets/` 生成，并手动上传。

## 基础数据

不含精选地图（`assets/levels/drahtseilakt/NN-*`）和仅开发用文件（`mods-dev`、`own/make`）的 `assets/`：我们自己的部件、字体、声音、内置模组、手册、演示关卡（`00-Showcase`、`Parts_Test`），以及与游戏绑定的 13 张社区 `.nmo` 地图（`assets/levels/community`）。约 93 MB。绝不包含任何原版 Ballance 文件。

## 关卡

13 张精选地图，每张一个 `.zip`（29-204 MB，共 1.1 GB），同样的地图也打成一个包 `drahtseilakt-levels`（安装后 2.8 GB，压缩后 1.1 GB）。每个文件都低于 GitHub 的 2 GiB 限制；超过的包脚本会拒绝生成。

## 版本

- `scripts/DATA_VERSION` 是日期 `YYYYMMDD`（如 `20261117`）。它既是数据的版本，也是程序包要求的版本：程序包依赖 `drahtseilakt-data (>= DATA_VERSION)`。
- 数据或关卡变化时更新它（并提交），然后打包，再上传。如果程序发布需要比最新数据发布更新的数据，必须先发布数据。
- 程序版本就是标签 `X.Y.Z`。没有标签的工作流运行在包中的版本是 `0~git<sha>`。
- 数据发布的标签是 `data-<DATA_VERSION>`；工作流的标签过滤器（`[0-9]+.[0-9]+.[0-9]+`）不匹配它，因此不会触发任何运行。

## 制作数据包和关卡包

```sh
scripts/pack-assets.sh          # OUT defaults to ../drahtseilakt-portable/bundles
```

它在 OUT 中生成（`V` = `scripts/DATA_VERSION`）：

| 文件 | 用途 |
|---|---|
| `drahtseilakt-data-V.tar.gz` | 其他系统；CI 下载它来制作便携压缩包、安装程序和 .app |
| `drahtseilakt-data_V-1_all.deb`、`drahtseilakt-data-V-1.noarch.rpm` | Linux，基础数据 |
| `levels/NN-Name.zip`（13 个） | 所有系统：解压到 `assets/levels/drahtseilakt/` |
| `drahtseilakt-levels_V-1_all.deb`、`drahtseilakt-levels-V-1.noarch.rpm` | Linux，全部 13 张地图（依赖数据包） |
| `SHA256SUMS` | 以上所有文件 |

`.deb` 和 `.rpm` 由 [nfpm](https://nfpm.goreleaser.com) 2.49.0 生成，两者用同一个工具，所以同一个脚本既能在所有者的机器上运行（无需 `rpmbuild`），也能在 CI 中运行。`scripts/fetch-nfpm.sh` 把它下载到 `third_party/`（已被 gitignore；绝不写入系统路径），并校验脚本中固定的 SHA-256（来自该发布自带的 `checksums.txt`）。`scripts/pack-assets.sh --no-packages` 跳过软件包。

## 上传数据发布

由所有者在检出目录中运行；这里没有任何东西会自动运行：

```sh
V=$(cat scripts/DATA_VERSION)
cd ../drahtseilakt-portable/bundles && sha256sum --check --quiet SHA256SUMS
gh release create "data-$V" --repo eugene-vereshchagin/drahtseilakt --draft --latest=false \
  --title "Drahtseilakt data $V" \
  --notes "Base data and the 13 picked maps for Drahtseilakt (data version $V). Programs: the releases X.Y.Z." \
  "drahtseilakt-data-$V.tar.gz" \
  "drahtseilakt-data_$V-1_all.deb" "drahtseilakt-data-$V-1.noarch.rpm" \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

查看草稿，然后发布。接着让 CI 指向新的基础数据（仓库变量，每个数据版本设置一次）。在私有仓库中，发布资源通过其 API 地址下载（若设置了 `ASSETS_TOKEN` 则用它作为 bearer，否则用本次运行自己的令牌）：

```sh
V=$(cat scripts/DATA_VERSION)
URL=$(gh api "repos/eugene-vereshchagin/drahtseilakt/releases/tags/data-$V" \
  --jq ".assets[] | select(.name == \"drahtseilakt-data-$V.tar.gz\") | .url")
SUM=$(sha256sum "../drahtseilakt-portable/bundles/drahtseilakt-data-$V.tar.gz" | cut -d' ' -f1)
gh variable set ASSETS_BUNDLE_URL --repo eugene-vereshchagin/drahtseilakt --body "$URL"
gh variable set ASSETS_BUNDLE_SHA256 --repo eugene-vereshchagin/drahtseilakt --body "$SUM"
```

（公开发布也可以用普通下载地址 `https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/data-$V/drahtseilakt-data-$V.tar.gz`。）如果地址中不含 `scripts/DATA_VERSION` 的版本，`scripts/ci-assets.sh` 会发出警告。

## 二进制发布（CI）

Actions -> “Drahtseilakt release” -> Run workflow（文件是运行产物），或推送标签 `X.Y.Z`：该运行还会创建一个包含下列文件和 `SHA256SUMS` 的**草稿**发布（不含数据和关卡）。输入项 `dev` 会加上开发版（绝不进入发布）。推送到分支不会触发任何运行。

`<V>` 为标签（不在标签上时为 `git-<sha>`），生成这些文件：

| 目标 | 文件 |
|---|---|
| Windows x86_64, arm64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` （安装程序）、`drahtseilakt-<V>-windows-<arch>.zip`（便携版） |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` （便携版）、`drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux x86_64 DEV（输入项 `dev`） | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` （未签名的 `.app`）、`drahtseilakt-<V>-macos-<arch>.tar.gz`（便携版） |
| FreeBSD x86_64（实验性） | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **便携压缩包**和 **Windows 安装程序**包含基础数据，因此可独立运行；精选地图是数据发布中单独的 `.zip` 文件。便携压缩包还带有工具箱。
- **Windows 安装程序**（NSIS，在 Ubuntu 上由便携 zip 构建）：默认按用户安装（`%LOCALAPPDATA%\Programs\Drahtseilakt`，无需管理员权限），带开始菜单文件夹，并在“应用”中提供卸载程序。它不安装 `portable.txt`，所以玩家数据存放在用户配置目录，卸载绝不会动它。未签名：SmartScreen 会警告。
- **Linux 软件包**：程序在 `/usr/bin/drahtseilakt`（系统库：`libasound2`、`libudev1`），数据在游戏查找的位置 `/usr/share/drahtseilakt/assets`（程序先从 `/usr/bin/../share` 查找，再查 `$XDG_DATA_DIRS`：见 `files.md`）。还有 `.desktop` 文件和图标（站点的 favicon，经 `tools/icon.py` 缩放，git 中不新增文件）。`drahtseilakt` 依赖 `drahtseilakt-data (>= DATA_VERSION)`；`drahtseilakt-levels` 是可选的。工具箱不在其中（仅便携版）。`drahtseilakt-dev` 提供并与 `drahtseilakt` 冲突（仅 `.deb`，来自输入项 `dev`）。
- **macOS**：`.app` 把数据放在 `Contents/Resources/assets`；未签名也未公证，首次启动需要右键 -> 打开。

在 Debian/Ubuntu 上安装： `sudo apt install ./drahtseilakt-data_V-1_all.deb ./drahtseilakt_X.Y.Z-1_amd64.deb`
（以及 `./drahtseilakt-levels_V-1_all.deb`）；Fedora： `sudo dnf install ./drahtseilakt-data-V-1.noarch.rpm ./drahtseilakt-X.Y.Z-1.x86_64.rpm`.

## 版本固定与权限

Actions 按完整提交 SHA 固定，Rust 工具链按精确版本固定，crate 由 `Cargo.lock` 固定，nfpm 和 Real-ESRGAN 按 SHA-256 固定；工作流只有 `contents: read`，仅 release 任务有 `contents: write`。NSIS 来自 Ubuntu 24.04 软件包仓库（由发行版签名，未按哈希固定；版本打印在日志中）。
