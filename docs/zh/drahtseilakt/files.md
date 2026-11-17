# 游戏把文件放在哪里

游戏读取两类文件：**随游戏发布的资源**（`assets/`：字体、自带部件、关卡、出厂 mod、音效主题）和**玩家的数据**（设置、回放、最高分、导入和自制的关卡、玩家的包、mod 和音效主题）。下面两种布局用的是同一个程序。

**发布资源**，先找到的生效：`$DRAHTSEILAKT_ASSETS`；程序旁边的 `assets/`；`../Resources/assets`（macOS 的 .app 包）；`../share/drahtseilakt/assets`（安装在前缀下：Linux 软件包把程序放在 `/usr/bin`，数据放在 `/usr/share/drahtseilakt/assets`）；仅供开发者使用的源码树（`target/release` 往上三级的 `assets/`，或编译时的路径）；最后是 `$XDG_DATA_DIRS` 每个目录下的 `drahtseilakt/assets`（默认 `/usr/local/share:/usr/share`）。发布包：[release.md](release.md)。启动时游戏会打印一行 `paths: assets ... | data ... (portable|per-user)`，会话日志也会重复这一行。

**便携版**（复制文件夹即可玩，不触碰文件夹之外的任何东西）：`drahtseilakt(.exe)`、`assets/` 和一个空文件 `portable.txt` 放在一起。玩家数据此时位于程序旁的 `data/`。规则只有这个文件：没有 `portable.txt` 就不是便携模式。

**便携版构建**（`scripts/make-portable.sh`）生成这样一个包含游戏和工具箱的文件夹，每个系统一个：默认是源码目录旁的 `drahtseilakt-portable/<os>-<arch>/`（`linux-x86_64`、`windows-x86_64`；`--target linux|windows`，其他输出文件夹作为参数）。它离线以 release 模式构建两个程序，并放入：游戏（`drahtseilakt.exe`；Linux 上是启动脚本 `drahtseilakt` 和程序 `drahtseilakt.bin`，`lib/` 中是声音、输入和 udev 库 `libasound`、`libudev`、`libcap`，其许可证在 `lib/LICENSES/`）、工具箱（`drahtseilakt-toolbox(.exe)`；它以同样方式识别 `portable.txt`，所以它转换和重制的内容都写入这个 `data/`）、`realesrgan/`（来自工具箱为该系统固定下载的 Real-ESRGAN，经 SHA-256 校验，附 `LICENSES.txt`；Windows 上连同发布包中的 `vcomp140.dll`；缺少时拒绝构建）、Windows 上程序导入的 MinGW DLL、`assets/`（所有发布内容，按 `SHA256SUMS` 校验；不含 `mods-dev/`、`own/make/` 和 Python 缓存）、`README.txt`、`LICENSE`、`CREDITS.md`、`THIRD-PARTY.md`、`VERSION`（两个仓库的 `git describe`）、`portable.txt` 和一个空的 `data/`。这是发布版构建，适用于所有系统：不含开发者 mod。`--dev`（仅 Linux）生成开发者构建到 `<os>-<arch>-dev/`：包含 `assets/mods-dev/`（god-mode），并在 `VERSION` 和 `README.txt` 中标为 DEV。对同一文件夹再次运行会替换程序和资源并保留 `data/`。`--with-originals DIR` 像游戏的导入功能那样把 Ballance 副本中的原版关卡导入 `data/`：仅用于私人测试构建，绝不用于发布版本。

**安装版**（程序和 `assets/` 在同一文件夹，如 `/opt/drahtseilakt`、`C:\Program Files\Drahtseilakt`、.app 的 Resources）：玩家数据在用户文件夹：Linux 为 `$XDG_DATA_HOME/drahtseilakt`（默认 `~/.local/share/drahtseilakt`），Windows 为 `%APPDATA%\drahtseilakt`，macOS 为 `~/Library/Application Support/drahtseilakt`。那里玩家自己的包、mod 和主题会与发布的一起被找到。
