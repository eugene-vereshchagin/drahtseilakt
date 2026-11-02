# 菜单：原版的结构和我们的

游戏菜单的计划，2026-11-02 与项目所有者商定。在玩家打开扩展模式之前，游戏会尽我们所能地还原原版；唯一可见的区别是多了一个用于打开扩展模式的选项。
扩展模式随后有它自己的设置：显示什么、使用我们的哪些新增功能，以及自制地图。

## 原版的菜单

读取自副本的 `3D Entities/Menu.nmo`（其中的脚本 `Menu_*`）和 `Language.nmo`（字符串，有德语、英语、西班牙语、意大利语和法语；
`GameSettings.Language` 选择用哪一列），并在 C 版游戏中核对过。

| 界面 | 各行，从上到下 | 说明 |
|---|---|---|
| 主菜单（`Menu_Main`） | Start, Highscore, Options, Credits, Exit | Exit 会询问 "Do you want to quit the game?"（Yes / No） |
| Start（`Menu_Start`） | Level 1 … Level 12, Back | 前一关完成后，这一关才能玩（`DB_Levelfreischaltung`）；其余的显示为灰色。第 13 关附加包会替换 Menu.nmo 并加上它的那一行 |
| Highscore（`Menu_Highscore`） | "Highscore Level N"、十个名字及分数、Back；左右键切换关卡 | 副本的默认值是 base.cmo 中的数组 `DB_Highscore_LvNN`（"Mr. Default"） |
| Options（`Menu_Options`） | Graphics, Controls, Sound, Back | |
| Graphics（`Menu_Opt_Graphics`） | Screen Resolution, Synch to Screen? (Yes/No), Clouds? (Yes/No), OK | |
| Controls（`Menu_Opt_Keys`） | Forward, Backward, Left, Right, Rotation, Overview：各一个键；Invert Rotation? (Yes/No)；OK | 默认：方向键、Shift、Space |
| Sound（`Menu_Opt_Sound`） | Music Volume, OK | 一个音量 |
| Credits（`Menu_Credits`） | 原版的制作人员名单 | |
| 暂停，关卡中按 Esc（`Menu_Pause`） | Back, Restart Level, Highscore, Options, Exit Level | Restart 和 Exit 会先询问 Yes / No；分数保留在屏幕上 |
| 死亡，没有剩余生命（`Menu_Dead`） | Restart Level, Home | |
| 终点结算（`Menu_Score`） | Level Bonus, Time Points, Extra Lives, Score | 逐行累加显示 |
| 新高分（`Menu_HighscoreEntry`） | "New highscore entry!"、分数、要输入的名字、OK | 会预填上一次输入的名字（`DB_Options.LastPlayer`） |
| 关卡结束（`Menu_End`） | Next level, Restart Level, Highscore, Options, Home | |

设置和高分榜保存在游戏文件夹的 `Database.tdb` 中。没有玩家档案：唯一与玩家相关的东西是高分榜里的名字。

## 我们的：经典模式（默认）

- 与上表相同的界面、行、顺序和字符串，用副本自己的纹理和字体绘制。字符串来自副本的 `Language.nmo`，所以菜单使用副本的语言。
- **一处新增**：Options 有第四个按钮 *Extended*；它的页面上有开关 *Extended mode?*（Yes / No）。这是玩原版关卡的玩家能看到的唯一区别。
- 关卡解锁方式与原版相同。高分榜也是原版的：每关十条，分数、名字。它们从副本的默认值开始，保存在我们自己的数据文件夹中，绝不写入游戏的文件。
  读取玩家的 `Database.tdb` 以接管其旧分数是以后的选项（一个解析器：和每个文件读取器一样要经过审查）。
- 原版有而我们暂时还没有的设置（屏幕分辨率、同步、云、按键）会随着各自行为的实现而加入；在此之前，这些行显示当前值，不能修改。
- 每一次完成的游玩仍然会保存为回放（在屏幕上没有任何代价）；最快的一次算作扩展模式中显示的最佳时间。

## 我们的：扩展模式

在 Options 中打开，或用 `--debug` 只对一次游玩打开。菜单保持原版的外观；变化的是：

- **主菜单**：Start、Highscore、Options、Credits、Exit，与原版相同；*Start* 会先询问要玩哪类关卡。
- **Options**：*Extended* 页面上有该模式的开关，其下每项新增内容各有一个开关（随模式全部打开；设置中以 `extended.<key> off` 记录被关闭的项）：
  - 所有关卡开放，无需解锁；
  - 关卡旁显示最佳时间；
  - Start 下按类别列出关卡；
  - 练习工具：自由视角（F2），减速和加速（`[` `]`），保存当前时刻并回到该时刻（F5 / F9），后退五秒（F3，再按一次继续后退，最多半分钟），屏幕上显示小节和 tick；借助这些工具完成的游玩会被保存，但永远不算最佳成绩；
  - 音乐音量旁边增加音效音量；
  - 无限生命：最后一个球永远不会丢失（因此而保住的游玩永远不算最佳成绩）。没有需要取消的时间限制：分数只会递减并停在零，与原版相同。
  更多开关会随路线图中的功能一起到来（视野、计时器、最佳成绩的幽灵）。
- **按类别的关卡**：Start 提供 *Original*（经典关卡列表）、*Remastered*、*Community maps* 和 *Open format*，即 `levels/`（[levels.md](levels.md)）中的各个文件夹。每一类中先按文件名列出其关卡，再列出包含更多关卡的文件夹（Enter 打开，Esc 返回上一级），像原版关卡一样游玩。无法加载的关卡会说明原因且不会启动；其游玩记录和最佳时间与原版关卡分开保存。以后：从社区目录获取标题、作者和难度。
- 材质包和我们自己的模组出现后也会放在这里，每个都有自己的开关，并提示第三方内容的游玩风险由玩家自行承担。

## 玩家档案

原版没有（见上文）。对我们来说，一个档案会把名字、设置、已解锁的关卡、高分榜和回放打包在一起，让几个人可以共用一台电脑。经典模式不需要它，
经典模式保留原版在高分登记中的单个名字。留给项目所有者的待定问题：玩家档案以后是否作为扩展模式的功能加入。

## 工作顺序

1. 已完成（2026-11-02）：默认经典模式；按上表实现的主菜单和 Options；*Extended* 页面。
2. 已完成：带 Yes / No 询问的暂停菜单；死亡菜单；关卡结束菜单。
3. 已完成：带名字输入的高分榜；关卡解锁。
4. 已完成：自定义关卡列表（levels.md）和扩展页面上的开关。
5. Graphics 和 Controls 设置随各自行为的实现而加入（目前只显示，不能修改）。

`DRAHTSEILAKT_SHOT_MENU=base:keys` 配合 `--screenshot` 可以在没有窗口的情况下把菜单画在画面上（base 为 `main`、`pause`、`dead` 或 `end`；
keys 中 `u d l r` 表示方向键，`e` 表示 Enter，`b` 表示 Esc，`n` 表示名字输入），用来查看每一个界面。
