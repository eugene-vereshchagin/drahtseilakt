# 游戏把文件放在哪里

游戏读取两类文件：**随游戏发布的资源**（`assets/`：字体、自带部件、关卡、出厂 mod、音效主题）和**玩家的数据**（设置、回放、最高分、导入和自制的关卡、玩家的包、mod 和音效主题）。下面两种布局用的是同一个程序。

**发布资源**，先找到的生效：`$DRAHTSEILAKT_ASSETS`；程序旁边的 `assets/`；`../Resources/assets`（macOS 的 .app 包）；仅供开发者使用的源码树（`target/release` 往上三级的 `assets/`，或编译时的路径）。启动时游戏会打印一行 `paths: assets ... | data ... (portable|per-user)`，会话日志也会重复这一行。

**便携版**（复制文件夹即可玩，不触碰文件夹之外的任何东西）：`drahtseilakt(.exe)`、`assets/` 和一个空文件 `portable.txt` 放在一起。玩家数据此时位于程序旁的 `data/`。规则只有这个文件：没有 `portable.txt` 就不是便携模式。

**安装版**（程序和 `assets/` 在同一文件夹，如 `/opt/drahtseilakt`、`C:\Program Files\Drahtseilakt`、.app 的 Resources）：玩家数据在用户文件夹：Linux 为 `$XDG_DATA_HOME/drahtseilakt`（默认 `~/.local/share/drahtseilakt`），Windows 为 `%APPDATA%\drahtseilakt`，macOS 为 `~/Library/Application Support/drahtseilakt`。那里玩家自己的包、mod 和主题会与发布的一起被找到。
