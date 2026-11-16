# 制作与分享关卡

关卡作者和玩家需要知道的内容：如何把关卡放到游戏能找到的地方，如何给它命名使菜单显示得好，以及如何分享它。每条规则背后的
细节见 [levels.md](levels.md)（文件夹）、[开放格式](open-format.md) 和 [menu.md](menu.md)（列表）；其背后的所有者决定见
[decisions.md](decisions.md)（“一个关卡就是一个文件夹”、“默认使用自有内容，按玩家意愿使用原版内容”）。

## 关卡放在哪里

所有关卡都位于游戏数据文件夹中的 `levels/`（Linux 上为 `~/.local/share/drahtseilakt/`，Windows 上为
`%APPDATA%\drahtseilakt\`，macOS 上为 `~/Library/Application Support/drahtseilakt/`）。你自己的或别人的关卡放进 `levels/user-levels/`，分两种：


- 社区地图（Virtools `.nmo` 文件，原版的格式）；
- [开放格式](open-format.md)的关卡（普通的 glTF 和 PNG 文件，便于手工完成或修改）。

**一个关卡，一个文件夹，直接放在该文件夹中：**

*自 2026-11-12 起，`levels/user-levels/` 是唯一推荐的文件夹（旧的 `downloaded/`、`community/`、`open/` 已在首次启动时移入其中，未删除任何文件）。* 菜单中：*Start，社区地图，我的关卡*。

```
levels/user-levels/
  Green Dam/                 关卡的文件夹：简单的名称，最多 40 个字符
    Green Dam.NMO            关卡：恰好一个 .nmo ……
    level.txt                它的名称、作者、难度（见下；可选，但没有标题的关卡排在最后）
    README.txt               可选：它是什么、怎么玩、它的许可证
    preview.png              可选：它的图片（也可以是 preview.jpg）
levels/user-levels/
  Sky Road/
    level.gltf level.bin     …… 或一个开放格式的关卡
    textures/                它自己的纹理（PNG），直接放在 textures/ 中
    game/                    它携带的游戏文件（它的部件、天空、声音：open-format.md）
    level.txt  README.md  preview.jpg
```

**规则：** 只有当一个关卡位于自己的命名文件夹中，且该文件夹里有一个 `.nmo` 文件或一个开放格式关卡（`level.gltf` ……），再加上描述文件 `level.txt`，它才算关卡。可以用子文件夹分组（`levels/user-levels/Pro Racing/Season 1/Green Dam/`）。没有 `level.txt` 的关卡文件夹和直接放在 `user-levels/` 里的单个 `.nmo` 不会列出；游戏日志会逐一指出并说明该怎么做。（以前单个 `.nmo` 自成一个关卡，level.txt 也是可选的。）

## level.txt

普通的 UTF-8 文本，每行一个 `键 = 值`，`#` 开始注释；未知的键被忽略。最多读取 4 KiB 和 64 行；更大的文件或链接被忽略，
如同不存在。

| 键 | 含义 | 规则 |
|---|---|---|
| `title` | 关卡的名称，**英文** | 要列在有名称的关卡中就需要它；最多 40 个字符（按单词截断，“…”） |
| `title.<代码>` | 它在另一种语言中的名称：`title.ru`、`title.de`、`title.zh`、`title.fr`、`title.es`、`title.it` | 菜单使用该语言时显示，否则显示英文标题；代码为 1–8 个小写字母；最多 16 个 |
| `author` | 作者 | 最多 40 个字符 |
| `version` | 它的版本 | 最多 40 个字符 |
| `difficulty` | 它的难度 | 一个整数，**1**（简单）到 **10**（最难）；其他任何值（0、11、3.5、"hard"）都视为未知 |
| `order` | 它在同一作者的关卡列表中的位置 | 一个整数；难度相同时，有它的关卡排在前面，按它的顺序 |
| `sky` | 地图使用的天空 | 游戏天空盒的名称：`A`、`B`、`C` 或我们自己的（`Mountains`、`Forest`、`Sea`、`Storm`、`Sunset`、`Night`、`Desert`、`Mist`、`Clouds`）；它优先于关卡文件自带的天空和原版的表，后者只在有副本且地图未指定时才读取 |
| `music` | 音乐主题 | 1 到 5 的整数 |
| `light` | 光的颜色 | 三个 0 到 1 的数，如 `1 0.9 0.8` |
| `hidden` | `yes`：不出现在列表中（用于测试的关卡） | 仍可用 `--open` 打开 |
| `source` | toolbox 在哪里找到它 | 由 `import-maps` 写入；不显示 |

每个值都会被清理：去掉控制字符、文字方向覆盖符和其他不可见格式字符，连续空格合为一个。例如我们自己关卡的 level.txt：

```
title = Paper Winds
title.ru = Бумажные ветра
title.de = Papierwinde
title.zh = 纸之风
author = Drahtseilakt
version = 1
licence = GPL-3.0-only
order = 3
difficulty = 3
```

## 名称

- **文件夹名称**要简单：字母、数字、空格和 `-_.()` 在任何系统上都可用；它不能是 Windows 保留的名称（`CON`、`NUL`、
  `COM1`……），不能以点或空格结尾，也不能以点开头（隐藏）。最多 40 个字符。toolbox 写出的每个名称在 Linux、Windows 和
  macOS 上都可用。
- **标题用英文。** 来自其他社区的关卡把自己的名称保留为 `title.<代码>`（中文地图的写作 `title.zh`），这样每位玩家都能
  看懂列表，原名也不会丢失。
- 尽量把版本写在 `version` 中，而不是标题中（“Lost City”，`version = 1.1`）。

## 菜单显示什么，按什么顺序

在扩展模式的 *Start → Community maps*（或 *Open format*）下是一个扁平列表，每个关卡一行：一颗星和它的难度（★7；未知时为
★?）以及它的标题（若 level.txt 提供了菜单语言的标题则用该语言）。列表排序如下：

1. 按难度，最简单的在前；难度未知的关卡排在所有已知难度的关卡之后；
2. 难度相同时，先按给出的 `order`，再按标题；
3. level.txt 中没有 `title`（只以文件夹命名）的关卡排在最后。

较长的列表可以滚动，有滚动条和“▼ 还有 N 项”。行内放不下的标题会缩小绘制或从中间缩短（“...”）；它绝不会超出自己的行。

## 什么被忽略，什么被放到一边

菜单只看直接位于 `community/` 和 `open/` 中的文件夹，并且只看每个文件夹中的关卡。以下内容被无害地忽略：关卡文件夹中的
任何其他文件、其中的每个文件夹（不嵌套：一个合集就是每关一个文件夹，“45-Degree Rotated - Level 07”）、隐藏名称、链接
（从不跟随），以及没有关卡或有多个关卡的文件夹（多个 `.nmo`，或 `.nmo` 与 `level.gltf` 并存）：这样的文件夹不是关卡，
会在控制台中列出。

`levels/community-unsorted/` 存放 toolbox 放到一边、留给你手工查看的内容：压缩包（`.zip`、`.rar`、`.7z`、
`.tar.gz`……）、不是单个关卡的文件夹，以及既不在社区目录中也没有附带 readme 的地图。其中的 `README.txt` 列出每一项及其
原因。游戏从不列出它；把想保留的内容解压、整理、命名，给它一个 level.txt，再移到 `community/` 中。

## 用 toolbox 导入合集

toolbox（`drahtseilakt-toolbox`，一个单独的程序；游戏本身从不运行它）把下载的合集整理成这种结构：

```
drahtseilakt-toolbox import-maps ~/Downloads/Maps --catalogue CustomMapList.md
```

- 该文件夹下（任意深度的文件夹中）的每个 `.nmo` 都成为 `levels/user-levels/` 中的一个关卡文件夹（`--to 文件夹` 可指定
  另一个），其中有地图、作为预览的图片，以及说明来源的 level.txt；
- 使用 `--catalogue`（社区的地图列表 `CustomMapList.md`）时，在其中找到的地图会得到英文标题、原名（中文地图为
  `title.zh`）、作者和难度（目录以半级为单位评定 1–5；乘以二得到我们的 1–10）；
- 既不在目录中、旁边也没有 readme 的地图，以及压缩包和不是单个关卡的文件夹，会被复制到 `levels/community-unsorted/`
  （`--unsorted 文件夹` 可指定另一个），并附一个说明原因的 README.txt；源文件始终只读；
- `--refresh levels/community --catalogue CustomMapList.md [--source 文件夹]` 把以前导入的文件夹更新到最新：它只重写
  level.txt 文件（目录中没有对应内容的地方保留你自己的行），并把不合格的文件夹移到未整理文件夹中。如果你编辑过
  level.txt 文件，请先备份。

## 分享关卡

- **你自己的资源，自由的许可证。** 连同文件一起分享的关卡只携带你自己制作的或采用自由许可证（CC0、CC-BY、GPL……）的
  内容，许可证放在其文件夹中（`LICENSE`，或 level.txt 和 README 中的一行），并注明他人作品的署名。
- **绝不包含原版的文件。** Ballance 的纹理、声音、网格或 `.nmo` 部件都不得放进分享的关卡。使用原版部件或纹理的关卡
  只指名它们：带 `"parts": "copy"`（默认）的开放格式关卡从每位玩家自己的拷贝中取得部件，用 `"parts": "own"` 制作的
  关卡则使用游戏自己的部件（open-format.md、resources.md）；`game:` 纹理由玩家的纹理包或拷贝绘制。toolbox 制作玩家
  拷贝所提供的内容；玩家往自己的文件夹里复制什么，是他们自己的事。
- **随游戏附带的关卡**还需要作者的许可、一个通关的回放，并且其中不含任何原版内容（[levels.md](levels.md)，“随游戏附带
  的内容”）。

## 上限

关卡来自陌生人，所以游戏把它当作恶意内容读取，超出上限的内容会被拒绝，而不是导致崩溃：

| 内容 | 上限 |
|---|---|
| level.txt | 4 KiB、64 行、每个值 40 个字符、16 个其他语言的标题 |
| 一个文件夹 | 在 `community/` 或 `open/` 中查看 10 000 个条目，在关卡文件夹中查看 4 096 个 |
| 一个 `.nmo` | 256 MiB；其中的每个结构都经过边界检查并有上限（nmo-format.md） |
| 一个开放格式关卡 | `level.gltf` 64 MiB，`level.bin` 1 GiB，图片只能直接放在 `textures/` 中，路径限制在该文件夹内（open-format.md，“读取”） |

游戏无法加载的关卡会在菜单中说明原因，且不会启动。

## 我们的关卡工具：检查什么

工具包的关卡以文本编写（`level.lvl`，[levels.md](levels.md)，“关卡工具包”），由
`assets/own/make/leveldsl.py` 借助 `levelkit.py` 生成；目前展示关卡和 `Parts_Test` 是用它制作的（它原本为之编写的我们自己的十个关卡已于 2026-11-16 移除）。用该工具制作的关卡，若破坏了关卡可玩所需的条件，会被拒绝并指出
位置（空间和平坦地面检查以及接缝测试随第 1 关的工作一起到来，在分支 `own-level-1` 中，levels.md 会在那里描述每一项）：

- **没有接缝**：一段地面从起点或一个检查点到下一个检查点是一个网格，因此球永远不会碰到两块之间的边缘；
- **给球留出空间**：变球器、风扇或气球上方 12 以内没有任何地面或轨道；从另一层地面或轨道下方经过的路，其底面距路面
  最多 3.6（球碰到的一堵墙）或 6 以上（可以通过的空间），绝不在两者之间，否则球会卡住；
- **部件下方地面平坦**：变球器的底座、箱子或圆顶立在其脚下一块平坦、水平的地面上；
- **挡得住的边缘**：护栏、砖砌挡边和槽形地面把扔向它们的球留在轨道上（测试以每秒 12、25 和 40 个单位扔出每种球）；
- **一个起点和一个终点、一个标题**，检查点的朝向是相机能够采用的。

关卡也可以由建立在工具之上的自有脚本生成：`assets/own/make/make_synthetic_level.py` 根据脚本内的描述生成随游戏附带的
展示关卡（[levels.md](levels.md)，“展示关卡”），带有自己的命令（标牌、大门、展品）、工具的地面不使用的材质，以及由
`scriptkit.py` 写入关卡 `scripts` 的行为图；检查相同。
