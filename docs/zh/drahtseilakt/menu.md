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
| Options（`Menu_Options`） | Graphics, Controls, Sound, Back | 我们在 Back 前另有 Language 和 Extended |
| Graphics（`Menu_Opt_Graphics`） | Screen Resolution, Synch to Screen? (Yes/No), Clouds? (Yes/No), OK | 副本的默认值（`DB_Options`）：不同步，有云 |
| Controls（`Menu_Opt_Keys`） | Forward, Backward, Left, Right, Rotation, Overview：各一个键；Invert Rotation? (Yes/No)；OK | 默认：方向键、Shift、Space；`DB_Options` 以按键在 `Language.nmo` 的 `all_keys` 中的行号保存它（按顺序的 DirectInput 扫描码） |
| Sound（`Menu_Opt_Sound`） | Music Volume, Sound Volume, OK | 音乐和音效两个音量（我们在所有模式下都显示两者，与项目所有者记得的原版一致） |
| Credits（`Menu_Credits`） | 原版的制作人员名单 | |
| 暂停，关卡中按 Esc（`Menu_Pause`） | Back, Restart Level, Highscore, Options, Exit Level | Restart 和 Exit 会先询问 Yes / No；分数保留在屏幕上 |
| 死亡，没有剩余生命（`Menu_Dead`） | Restart Level, Home | |
| 终点结算（`Menu_Score`） | Level Bonus, Time Points, Extra Lives, Score | 逐行累加显示 |
| 新高分（`Menu_HighscoreEntry`） | "New highscore entry!"、分数、要输入的名字、OK | 会预填上一次输入的名字（`DB_Options.LastPlayer`） |
| 关卡结束（`Menu_End`） | Next level, Restart Level, Highscore, Options, Home | |

设置和高分榜保存在游戏文件夹的 `Database.tdb` 中。没有玩家档案：唯一与玩家相关的东西是高分榜里的名字。

## 我们的：经典模式（默认）

- 与上表相同的界面、行、顺序和字符串，用我们自己的界面绘制（decisions.md，“自有界面为默认”；[resources.md](resources.md)，“自有界面”）：我们的纹理、字体和文字。扩展模式的 Interface 开关可以选择原版界面，即用副本的纹理和字体、并使用其 `Language.nmo` 的语言。
- Options 中有**两处新增**：*Language* 和 *Extended*。*Language* 打开语言页面，每种语言以其自身名称显示（English、Русский、Deutsch、简体中文、Français、Español、Italiano），当前语言显示为金色；按 Enter 立即切换到所选语言并保存（`language zh`）。在玩家选择之前，如果系统语言（区域设置变量 `LC_ALL`、`LC_MESSAGES`、`LANG`；从桌面启动的程序在 Windows 和 macOS 上没有这些变量）在列表中，菜单就使用系统语言，否则使用英语。*Extended* 页面上有开关 *Extended mode?*（Yes / No）。这就是玩原版关卡的玩家能看到的全部变化。
- 关卡解锁方式与原版相同。高分榜也是原版的：每关十条，分数、名字。它们从我们自己的默认榜单开始（使用原版界面时从副本的默认值开始），保存在我们自己的数据文件夹中，绝不写入游戏的文件。
  读取玩家的 `Database.tdb` 以接管其旧分数是以后的选项（一个解析器：和每个文件读取器一样要经过审查）。
- **Graphics**：原版的三行，再加一行 *Fullscreen?*（No / Yes / Exclusive），因为窗口也可以占满屏幕。左右键（或 Enter）修改一行；
  修改立即生效并保存在 `settings.txt` 中：
  - *Screen Resolution* 依次切换显示器的模式和常见的窗口尺寸（`resolution 1600x900`）；窗口采用该尺寸，独占全屏采用该显示模式；
  - *Synch to Screen?* 让画面等待显示器（`vsync on`；默认关闭，与副本相同，也因为在测试机器上等待会卡住；`DRAHTSEILAKT_PRESENT`
    仍然优先）；
  - *Clouds?* 显示关卡的云层（`clouds on|off`），即其 `SkyLayer` 对象，选项打开时 `Gameplay.nmo` 会显示它。它的纹理是黑底上的灰色云，
    材质不做混合，所以我们把它叠加到后面的天空上绘制（这是我们的理解；副本的材质没有说明）。视频追踪绘制时不带云层；
  - *Fullscreen?*（`fullscreen off|borderless|exclusive`）：无边框使用桌面的显示模式，独占在平台允许时更改它（Wayland 不允许：改为无边框）。
    F11 在窗口和无边框之间切换。
- **Controls**：在某个按键的行上按 Enter 会等待按键（“Press a key”）；按下的键被绑定，Esc 取消。已被其他动作占用的键会互换，所以没有一个键
  同时做两件事。Esc 和 F11 属于游戏本身，不能绑定；绑定的键优先于扩展模式的练习键。Invert Rotation? 让相机反向转动。按键保存为
  `key.forward ArrowUp` … `key.overview Space` 和 `invert-rotation on|off`（winit 的按键名）；没有这些行时使用副本 `DB_Options`
  中的按键。只要另一个 Shift 本身没有被绑定，任意一个 Shift 都起到已绑定的那个的作用。页面用副本中的名字显示按键，没有时用我们自己的。
- **手柄**（gilrs）按下与键盘相同的键，所以无论用什么玩，一次游玩和它的回放都相同：左摇杆或十字键滚动球，左右肩键转动相机（Rotation 加左或右），
  下方的面键（A / ✕）是 Overview，Start 暂停。在菜单中十字键移动，下方的键是 Enter，右边的键（B / ○）是 Esc。手柄按键暂时不能重新绑定。
- **鼠标**可以操作所有菜单，与原版一样：光标下的按钮被选中（仅在光标移动时，所以静止的光标不会与按键冲突），左键单击如 Enter 般按下它，
  右键和鼠标的后退键是 Esc。带数值的行（音量、分辨率、视野、是/否开关）单击左半边减小、右半边增大，如左右方向键；高分榜的箭头切换关卡。
  滚轮滚动社区关卡列表（选中光标下的行），在其他菜单中逐行移动，在高分榜中切换关卡。在 Controls 页面单击按键所在的行会如 Enter 般等待一个键；
  鼠标按键不能绑定。光标在菜单上显示，在游玩中隐藏，从不被锁在窗口内；游玩不接受鼠标输入，所以关闭菜单的那次单击不会在关卡中起作用。
  按钮在其绘制的位置被点中，任何窗口大小、两种界面皆然。
- 每一次完成的游玩仍然会保存为回放（在屏幕上没有任何代价）；最快的一次算作扩展模式中显示的最佳时间。

## 我们的：扩展模式

在 Options 中打开，或用 `--debug` 只对一次游玩打开。菜单保持原版的外观；变化的是：

- **主菜单**：Start、Highscore、Options、Credits、Exit，与原版相同；*Start* 会先询问要玩哪类关卡。
- **Options**：调试类开关（练习工具、无限生命）位于其子页面 *Debug mode* 中；*Extended* 页面上有该模式的开关，其下每项新增内容各有一个开关（随模式全部打开；设置中以 `extended.<key> off` 记录被关闭的项）：
  - 所有关卡开放，无需解锁；
  - 关卡旁显示最佳时间；
  - Start 下按类别列出关卡；
  - Interface：own / original / remaster，界面文件的来源（[resources.md](resources.md)）；
  - Sounds：音效主题——系统、生成、原版（由 toolbox 制作）或玩家自己的主题（[resources.md](resources.md)）；在玩家选择其他主题之前（经典模式下始终）播放生成主题；
  - 练习工具：自由视角（F2），减速和加速（`[` `]`），保存当前时刻并回到该时刻（F5 / F9），后退五秒（F3，再按一次继续后退，最多半分钟），屏幕上显示小节和 tick；借助这些工具完成的游玩会被保存，但永远不算最佳成绩；
  - 无限生命：最后一个球永远不会丢失（因此而保住的游玩永远不算最佳成绩）。没有需要取消的时间限制：分数只会递减并停在零，与原版相同。
  更多开关会随路线图中的功能一起到来（计时器、最佳成绩的幽灵）。
- **Graphics** 增加 *Field of View*：游戏相机在 4:3 上的视野宽度，40° 到 100°，步长 2（`fov 58`；原版为 58°，即 InGameCam 的
  1.0123 弧度）。经典模式始终使用原版的视野。
- **Controls** 增加 *Defaults*：恢复原版的按键（方向键、Shift、Space），并取消相机反转。
- **按类别的关卡**：Start 提供 *Original*（经典关卡列表）、*Remastered*、*Community maps* 和 *Open format*，即 `levels/`（[levels.md](levels.md)）中的各个文件夹。每一类中是按标题排序的扁平关卡列表（一个关卡是只含一个关卡的文件夹，或单独的 `.nmo`：levels.md，“一个关卡就是一个文件夹”；没有要打开的文件夹，也不显示其他东西），像原版关卡一样游玩。标题最多 40 个字符，会缩小或换行以留在按钮上。无法加载的关卡会说明原因且不会启动；其游玩记录和最佳时间与原版关卡分开保存。以后：从社区目录获取标题、作者和难度。
- 材质包和我们自己的模组出现后也会放在这里，每个都有自己的开关，并提示第三方内容的游玩风险由玩家自行承担。

## 玩家档案

原版没有（见上文）。对我们来说，一个档案会把名字、设置、已解锁的关卡、高分榜和回放打包在一起，让几个人可以共用一台电脑。经典模式不需要它，
经典模式保留原版在高分登记中的单个名字。留给项目所有者的待定问题：玩家档案以后是否作为扩展模式的功能加入。

## 工作顺序

1. 已完成（2026-11-02）：默认经典模式；按上表实现的主菜单和 Options；*Extended* 页面。
2. 已完成：带 Yes / No 询问的暂停菜单；死亡菜单；关卡结束菜单。
3. 已完成：带名字输入的高分榜；关卡解锁。
4. 已完成：自定义关卡列表（levels.md）和扩展页面上的开关。
5. 已完成：Graphics 和 Controls；手柄。

`DRAHTSEILAKT_SHOT_MENU=base:keys` 配合 `--screenshot` 可以在没有窗口的情况下把菜单画在画面上（base 为 `main`、`pause`、`dead` 或 `end`；
keys 中 `u d l r` 表示方向键，`e` 表示 Enter，`b` 表示 Esc，`n` 表示名字输入，`w` 表示在等待按键时按下 W 键），用来查看每一个界面。不在关卡之上的菜单画在界面的菜单 3D 场景前，场景取其镜头环绕一周中的第 `DRAHTSEILAKT_SHOT_MENU_MS` 毫秒（默认 0）。
