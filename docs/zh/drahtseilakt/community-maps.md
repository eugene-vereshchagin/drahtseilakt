# 自制地图：在哪里，以及如何使用目录

本页供[路线图](roadmap.md)第 4 阶段参考，也供任何需要自制地图或其相关数据的人（或智能体）使用。目录快照日期：2026-09-25。

## 来源

| 内容 | 位置 | 备注 |
|---|---|---|
| 目录（列表） | [pseudoacetyl/BallanceCustomMaps — `CustomMapList.md`](https://github.com/pseudoacetyl/BallanceCustomMaps/blob/main/CustomMapList.md) | MIT 许可的仓库；程序化访问方式见下文 |
| 全部地图（文件） | [Mega: Custom Maps](https://mega.nz/folder/ARR3RRBL#kvoVNZVOxbUmpDBUqLUccg) | 由目录作者提供 |
| 精选合集 | [MediaFire: Ballance Map Packs](http://www.mediafire.com/folder/vyzo7893i0oln/Ballance_Map_Packs) | “高质量”地图，7.6 版；每隔几年才更新一次，因此缺少近期的地图 |
| 玩家、模组 | [Mega: Players](https://mega.nz/folder/EIYiyASK#GGanOfhFVcSWjnjxOzJjIQ)、[Mega: Mods](https://mega.nz/folder/wEB11B5D#FhU0JpzqomuFCAkMuBePWA) | |
| 新地图 | Ballance International Discord，频道 `#new` | 使用方法：[Discord FAQ](https://yyc12345.github.io/ballance-discord-rules/en/faq/) |
| 中文社区 | 百度贴吧 平衡球吧（Baidu Tieba）、中文 Ballance 论坛（Chinese Ballance Forum）；视频在哔哩哔哩（Bilibili） | 最大的受众群体；目录中的 ID 指向这些地方 |

**智能体如何访问。** 目录是 GitHub 上的纯 Markdown：
`git clone --depth 1 https://github.com/pseudoacetyl/BallanceCustomMaps` 或
`https://raw.githubusercontent.com/pseudoacetyl/BallanceCustomMaps/main/CustomMapList.md`。
云端沙箱通常无法访问 Mega 和 MediaFire；地图文件由项目所有者在自己的机器上下载。

## 目录的格式

一张 Markdown 表格，每行一张地图，按时间从旧到新排列：

| 列 | 内容 |
|---|---|
| Posted | 发布日期 `YYYY-MM-DD`；日期为近似值时加括号，为估计值时写作 `(~…)` |
| Baidu Tieba ID & Chinese Ballance Forum ID | 帖子 ID；论坛 ID 形如 `C126`；可能为空或为 `N/A` |
| Original Name | 通常为中文 |
| English Name | 译名 |
| Author | |
| Difficulty | 1–5，有时为半级（`2.5`）或区间（`2.5–3`） |
| Type | Comprehensive、Technical、Logical、Casual、Skillogic、Racing、Puzzle、Landscape |
| Videos | YouTube 链接，多个链接以 `<br />` 分隔；可能为空 |

解析时要做好防御：少数行的名称中含有 `|`，会使列发生错位（在 2026-09-25 的快照中，有几行在难度一栏出现了作者或名称）。逐行校验，并报告格式错误的行，而不要去猜测。

## 数据概览（2026-09-25）

- 约 427 张地图，发布于 2009 年至 2026 年；每年新增 20–60 张。
- 难度：1 级约 75 张，2 级约 75 张，3 级约 100 张，4 级约 100 张，5 级约 65 张。
- 类型：约一半为 Comprehensive；其次是 Technical（约 85）、Logical（约 55）、Casual（约 30）、Skillogic、Racing、Puzzle、
  Landscape。
- 视频：366 张地图至少有一个视频，**99 张有两个或更多**。被多人游玩并发布过视频的地图自然构成“前 100 名”——这是首批用于测试的选集。

## 社区安装地图的惯例

- **替换关卡**：将地图复制并覆盖 `3D Entities/Level/Level_01.NMO` … `Level_13.NMO` 之一。无需模组。
- **BML**：将地图放入 `ModLoader/Maps`；BallanceModLoader(Plus) 会添加一个地图菜单。
- 我们的引擎两者都不用：地图放在引擎自己的地图文件夹中，绝不写入游戏的文件（第 4 阶段）。

地图的制作方式：Blender 3.6 配合 [BallanceBlenderHelper](https://github.com/yyc12345/BallanceBlenderHelper)，
再通过 Virtools 3.5 导出；较早的地图则使用 3ds Max 插件制作。

## 项目所有者机器上的本地副本

项目所有者已下载了地图和模组。它们与游戏数据一样存放在**仓库之外**，通过环境变量定位：

| 变量 | 指向 | 使用者 |
|---|---|---|
| `OPENBALLANCE_DATA` | 游戏：安装目录、CD 目录或光盘镜像 | 游戏数据测试、游戏本身 |
| `OPENBALLANCE_MAPS` | 自制地图文件夹（`.nmo`，保持下载时的样子，压缩包已解压） | 地图测试、缺失 Building Block 报告 |
| `OPENBALLANCE_MODS` | 已下载模组的文件夹 | 仅供阅读，见下文 |

解压下载内容属于工作的一部分，须遵守压缩包规则（`modules/input.md`）：压缩包内的每个路径都要检查（不得含 `..`、不得为绝对路径、不得为链接），并限制文件数量和解压后的大小。引擎为玩家准备的地图文件夹（第 4 阶段）与这一开发用语料库是分开的。

**地图是数据。** 它们是第 4 阶段要加载的内容，也是测试运行的对象：对整个文件夹做一次无界面（headless）遍历，加载每张地图，游玩一段时间，并报告崩溃和缺失的 Building Blocks。

**模组是代码。** BML 模组是原版播放器加载进自身进程的 Windows DLL。它们绝不会被执行——引擎不执行，测试不执行，也不会在项目所有者的机器上用 Wine “只是看看”。它们唯一的用处是作为需求来源：社区为游戏添加了什么（速度表、地图菜单、镜头选项、修复），就说明玩家想要哪些功能。这样的功能会根据其可观察的行为在我们的引擎中重新实现，与其他功能一样；不会从模组中加载任何东西。

## 地图除场景之外还带有什么

由审计（`drahtseilakt --audit 目录`）在 606 张地图上查出；`nmo-dump --extras 文件` 可列出单张地图的情况。

- **2D 框**（34 张地图）：作者在屏幕一角的签名或标志。文件中显示、且材质带有纹理的框，会绘制在画面之上、游戏界面之下，
  位置按文件在原版 640 x 480 上下文中的设定，依 Z 顺序绘制（每张地图最多 16 个）。隐藏的框或没有纹理的框由地图脚本
  显示或填充：不绘制。这些图像几乎全部保存在地图内，地图自带的许多地板和部件纹理也是如此：一种原始位图，各通道按整幅
  平面依次存放（蓝、绿、红、Alpha，每个平面为 宽 x 高 字节，行自下而上）。开放格式的关卡暂不携带框。
- **没有图像的纹理**（31 张地图）：既无像素也无文件名的纹理，以游戏纹理命名，常被导出器截成 DOS 文件名
  （`METAL_ST`、`RAIL_ENV`、`WOOD_MET.Copy`），或者没有名字、但所属材质以某个纹理命名（`_aaaSkyLayer`）。它被当作
  该游戏纹理（我们的、纹理包的或玩家副本的）。截断的名字对应多个纹理时取最短的（`FLOOR_TO` 即 `Floor_Top_Flat`）：
  这是推测，审计以 `texture-by-name` 列出。原版绘制它们时不带纹理。
- **谁都没有的纹理**（9 张地图）：作者没有放进地图的文件。以调暗的材质颜色绘制，绝不是纯白。
- **声音**（8 张地图）：已读取（文件、循环、音量），但文件不在地图里，且只由地图脚本播放：不发声。
- **曲线**（16 张地图）：导出器留下的形状；没有任何模块或参数读取它们。不绘制。
- 未读取：面片网格（2 张地图）、MIDI 声音（3 张地图）。

## 本项目的规则

- **地图是不可信的输入**：来自陌生人网盘的文件。它们只能经过加固过、经过模糊测试的解析器处理（见 `CLAUDE.md`）。
- **地图文件绝不进入仓库**——它们是作者的作品，我们无权以 GPL 重新分发。选集仅以数据形式保存：名称、作者、日期、目录行、链接。
- 预期的外观和路线以目录中的视频为准，而不是以我们引擎碰巧显示的内容为准。
