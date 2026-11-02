# 关卡存放在哪里

游戏如何保存其关卡，已于 2026-11-02 与项目所有者商定（decisions.md，“关卡存放在一个文件夹中，分四类”）。显示关卡的菜单见 [menu.md](menu.md)。

## 文件夹

所有关卡都位于游戏数据文件夹（Linux 上为 `~/.local/share/drahtseilakt/`，Windows 上为 `%APPDATA%\drahtseilakt\`，macOS 上为 `~/Library/Application Support/drahtseilakt/`）中的同一个文件夹 `levels/` 里，按来源和格式划分：

| 文件夹 | 内容 | 来源 | 模式 |
|---|---|---|---|
| `levels/original/` | 原版游戏：`base.cmo`、`3D Entities/`、`Textures/`、`Sounds/` 等，并已安装第 13 关的扩展包 | 由游戏从玩家已安装的游戏或 CD 中复制 | 经典模式和扩展模式 |
| `levels/community/` | 自制地图，`.nmo`，可按玩家喜好放在文件夹中（菜单可打开它们，最多四层） | 由玩家下载；文件夹中的 README 说明去哪里获取 | 扩展模式 |
| `levels/remastered/` | 转换为开放格式的原版关卡，纹理放大到 4K–8K，并加上现代画面所需的其他内容 | 由我们的脚本在玩家机器上根据 `original/` 生成；从不分发 | 扩展模式 |
| `levels/open/` | 开放格式的关卡：直接用它制作的，或转换后的自制地图 | 由玩家制作；部分随游戏附带 | 扩展模式 |

经典模式只看到 `original/`；扩展模式看到全部四个（menu.md）。

## 原版游戏：默认复制

游戏从玩家自己的副本中把所需内容一次性复制到 `levels/original/`：从已安装的游戏复制（`drahtseilakt --import <Ballance folder>`），或直接从游戏 CD 或其镜像复制（`drahtseilakt --import Ballance.cue`，也可以是 `.bin`、`.iso`：读取 `SETUP/` 中的 InstallShield 6 压缩包，由 `crates/cdimage` 实现）。没有找到游戏时启动会打开一个窗口完成同样的事：它列出在电脑上找到的内容（Windows、Wine、GOG、Heroic 和 Lutris 常用文件夹中已安装的游戏；光驱中的光盘；主文件夹、桌面和“下载”文件夹中的 CD 镜像），接受拖到窗口上的文件夹或镜像或输入的路径，然后用这份副本启动游戏。副本是玩家自己机器上的玩家自己的数据；不下载任何东西，也不往仓库里放任何东西。`--data DIR` 仍可不经复制直接从任意文件夹游玩。

该文件夹的 README 列出了需要手动复制的内容：已安装的 Ballance 中的 `base.cmo` 以及 `3D Entities`、`Textures` 和 `Sounds` 文件夹（游戏不读取其他内容）；对于第 13 关，还需按其说明文件将 SuDu 扩展包安装到该副本中。

## 任何文件夹都可以移走

四个文件夹都可以在设置中指向别处（`levels.original`、`levels.community`、`levels.remastered`、`levels.open`），以便使用大容量磁盘或共享文件夹。此时默认文件夹中只保留一个 README，说明关卡在哪里，这样玩家去那里找时也能找到。

## 随游戏附带的内容

游戏安装时会在 `levels/open/` 中附带一些关卡（我们自己的），以及作者希望随游戏发布的转换后的自制地图。每个随附的关卡需要：

- 作者的许可和自由许可证，记录在关卡旁边（其文件夹中的 `LICENSE`）；
- 不含原版游戏的任何内容：不含 Ballance 的纹理、声音或网格。使用原版纹理的转换地图只写明纹理名称，游戏从 `levels/original/`（玩家的副本）中取用，就像该地图在原版中那样；
- 一个能通关的回放，按与原版关卡回放相同的方式检查。

只有在许可证允许后，仓库才会收录这类关卡；原版游戏的文件绝不会进入仓库。

## 每个文件夹都自带说明

首次启动时，游戏会在每个文件夹中用玩家的语言写入一个 README：这里该放什么、从哪里获取，以及对于 `remastered/`，由哪个脚本生成。这些文本随游戏附带（各语言的文档）；README 中的链接只是文本，游戏从不打开它们。

## 按类别记录成绩和高分榜

回放、最佳时间和高分榜按类别分别保存，因此与原版关卡同名的自制地图绝不会被算作原版关卡：原版关卡为 `replays/<level>/`，其他为 `replays/maps/<folder>/<map>/`（`community`、`open`、`remastered`）。

## 语言

社区规模最大的是中国（百度贴吧 平衡球吧、中文 Ballance 论坛、哔哩哔哩；见 community-maps.md），因此文档和各文件夹的 README 除英语、俄语和德语外，也提供简体中文版本。

## 工作顺序

1. 已完成（2026-11-02）：本页和该决定；中文文档。
2. 已完成：`levels/` 各文件夹及其 README；用于移动它们的设置，以及留在原处的 README。
3. 已完成：从已安装的游戏或 CD 镜像 `--import`（不加 `--data` 游戏也能找到导入的副本）。欧洲版 CD 上的 318 个游戏文件与已安装的副本逐字节一致。
4. 读取 `community/` 和 `open/` 的自定义关卡菜单；`remastered/` 由 `drahtseilakt-toolbox remaster` 生成：转换器（第 6 阶段）、更圆的形状以及放大 N 倍的纹理；所有关卡共用的部件（球、模块、变球器）也以同样方式重制到隐藏的 `remastered/.shared/` 中，重制关卡会按此绘制它们（安装了 Real-ESRGAN 时使用它，否则用 Lanczos；第 7 阶段）。拷贝中缺少的原版关卡（没有第 13 关的拷贝）在关卡菜单中从它在 `remastered/` 里的导出版本来玩。
5. 随游戏附带的关卡，视作者提供情况而定。
