# 菜单：原版的结构和我们的

游戏菜单的计划，2026-11-02 与项目所有者商定。在玩家打开扩展模式之前，游戏会尽我们所能地还原原版；唯一可见的区别是多了一个用于打开扩展模式的选项。
扩展模式随后有它自己的设置：显示什么、使用我们的哪些新增功能，以及自制地图。

## 原版的菜单

读取自副本的 `3D Entities/Menu.nmo`（其中的脚本 `Menu_*`）和 `Language.nmo`（字符串，有德语、英语、西班牙语、意大利语和法语；
`GameSettings.Language` 选择用哪一列），并在 C 版游戏中核对过。

| 界面 | 各行，从上到下 | 说明 |
|---|---|---|
| 主菜单（`Menu_Main`） | Start, Highscore, Options, Credits, Exit | Exit 会询问 "Do you want to quit the game?"（Yes / No） |
| 没有副本时的主菜单（我们的） | Start, Highscore, Options, Credits, Exit | Start 列出 "Our levels"（随附的关卡和 `levels/open/`）；Options 多出一行 Import（在 Extended 之前），打开引入玩家副本的窗口：Esc 或 Cancel 回到游戏，窗口的关闭按钮结束程序，导入后游戏用该副本重新启动（decisions.md，“游戏开箱即玩”） |
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
  - `cloud-quality low|medium|high|off`（设置文件；默认 `medium`）：我们关卡的云海绘制得多精细（见 levels.md“云海”）：穿过它的步数、朝向太阳的步数、边缘的小云团；`off` 不绘制，关闭 *Clouds?* 也会隐藏它；
  - *Fullscreen?*（`fullscreen off|borderless|exclusive`）：无边框使用桌面的显示模式，独占在平台允许时更改它（Wayland 不允许：改为无边框）。
    F11 在窗口和无边框之间切换。
- **外观选择**（所有者 2026-11-09 的决定）在经典模式下同样提供，设置键与扩展模式的各行相同：*Textures* 和 *Models* 在 Graphics 页的各行之下，*Sound theme* 在 Sound 页的两个音量之下（先是原版的各行，顺序不变，我们的在其下，如同 *Fullscreen?*）。箭头循环切换找到的每个包：内置的 `own`、`original`（仅当玩家的副本存在时）、`remastered`，以及数据文件夹中的任何包，因此玩家自己转换的资源会直接出现在选项中（[packs.md](packs.md)、[sound-themes.md](sound-themes.md)）。默认值不变（我们的纹理、关卡自带的部件、默认主题）。只改变外观：规则和玩法仍是原版的，回放不依赖外观。渲染修正（任意分辨率、宽屏、过滤、平滑）始终开启，不可切换。
- **Controls**：在某个按键的行上按 Enter 会等待按键（“Press a key”）；按下的键被绑定，Esc 取消。已被其他动作占用的键会互换，所以没有一个键
  同时做两件事。Esc 和 F11 属于游戏本身，不能绑定；绑定的键优先于模组的按键。Invert Rotation? 让相机反向转动。按键保存为
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

在 Options 中打开，或用 `--debug` 只对一次游玩打开（即调试运行，它还提供 *All levels open* 和开发用模组上帝模式，[mods.md](mods.md)，“上帝模式”）。菜单保持原版的外观；变化的是：

- **主菜单**：Start、Highscore、Options、Credits、Exit，与原版相同；*Start* 会先询问要玩哪类关卡。
- **Options**：*Extended* 页面上有该模式的开关，其下每项新增内容各有一个开关（随模式全部打开；设置中以 `extended.<key> off` 记录被关闭的项）：
  - 所有关卡开放，无需解锁：仅限调试运行（`--debug`）；玩家的游戏通过游玩来解锁关卡；
  - 关卡旁显示最佳时间；
  - （自 2026-11-09 起 Start 在两种模式下相同；不再有此开关）；
  - Interface：Drahtseilakt / Ballance / Remaster，界面文件的来源（[resources.md](resources.md)）；
  - 外观：Graphics 页上的一个选择 *外观*（原版 / 重制版 / Drahtseilakt / 自选包；见下文“外观”），取代纹理包一行；下一关加载时生效（[packs.md](packs.md)）；
  - Models：绘制部件和球所用的模型包（只改变外观，绝不改变物理）——关卡自带（每个关卡使用它自己采用的部件：默认）、Drahtseilakt、Ballance 或玩家的模型包（[packs.md](packs.md)）；
  - Sounds：音效主题——系统、生成、Ballance（由 toolbox 制作）或玩家自己的主题（[resources.md](resources.md)）；在玩家选择其他主题之前播放生成主题；
  - Mods：找到的每个模组所在的界面（内置模组和玩家的模组；调试运行中还有上帝模式，它包含原来的练习工具和无限生命），每项用左右键切换，Enter 打开其页面及选项和按键（[mods.md](mods.md)）；经典模式下关闭。没有需要取消的时间限制：分数只会递减并停在零，与原版相同。
- **Graphics** 增加 *Field of View*：游戏相机在 4:3 上的视野宽度，40° 到 100°，步长 2（`fov 58`；原版为 58°，即 InGameCam 的
  1.0123 弧度）。经典模式始终使用原版的视野。
- **Controls** 增加 *Defaults*：恢复原版的按键（方向键、Shift、Space），并取消相机反转。
- **关卡列表**（列表从 Start 打开，见下）：每一类中是扁平的关卡列表（一个关卡是只含一个关卡的文件夹，或单独的 `.nmo`：levels.md，“一个关卡就是一个文件夹”；没有要打开的文件夹，也不显示其他东西），像原版关卡一样游玩。每一行显示关卡的**难度**，一颗星和 1 到 10（level.txt 的 `difficulty`；未知时为“★?”），以及其英文**标题**（level.txt 的 `title`），若 level.txt 提供了菜单语言的标题（`title.zh`……）则显示该标题。列表按难度排序，其次按标题；难度未知的关卡排在难度已知的之后，只以文件夹命名（没有 `title`）的关卡排在最后。标题最多 40 个字符，会缩小绘制或从中间缩短，以留在其行内。无法加载的关卡会说明原因且不会启动；其游玩记录和最佳时间与原版关卡分开保存。如何制作和放置关卡：[making-levels.md](making-levels.md)。
- 材质包和模组都在这里：包在 Textures 和 Models 两行（[packs.md](packs.md)），模组在 Mods 界面（[mods.md](mods.md)），每个都有自己的开关；第三方内容的游玩风险由玩家自行承担。
- **长列表**（开始 > 我的地图文件夹、模组、修改版原版）：在任意一行按 Esc 或 Backspace（手柄的返回键）即可返回；Back 固定在列表下方，旁边有“Esc：返回”提示。PageUp / PageDown 翻一页，Home / End 跳到第一行 / 最后一行，按住上 / 下键会重复（0.35 秒后每秒 12 行，再过一秒后 30 行），滚轮滚动，输入字母跳到下一个以该字母开头的标题。*社区精选*是随游戏附带的 13 张地图（注明作者）；*我的地图文件夹*列出放在游戏数据文件夹 `levels/community/` 中的地图（路径显示在列表下方）。


## 按含义划分的 Start（所有者 2026-11-09 的决定）

Start 在经典模式和扩展模式下相同；为空或不可用的项目不显示。每个项目和子菜单项都在说明面板上写明它是什么、来自哪里。

| 项目 | 打开 | 需要 |
|---|---|---|
| *Drahtseilakt* | 我们自己的 13 个关卡，随游戏附带 | |
| *经典 Ballance*（en *Classic Ballance*） | 玩家副本中的 13 个原版关卡，按原版方式解锁 | 副本 |
| *Ballance 挑战*（en *Ballance Challenges*） | 社区改造的原版关卡：视角旋转 45°、反向通关、隐形道路（`levels/modified/`） | 副本和该文件夹中的关卡 |
| *社区地图*（en *Community maps*） | 子菜单 | |
| &nbsp;&nbsp;*精选*（en *Best*） | 随游戏附带的 13 张社区地图，附作者和致谢 | |
| &nbsp;&nbsp;*已下载*（en *Downloaded*） | 玩家的文件夹 `levels/downloaded/`（显示其路径）：社区 `.nmo` 地图和开放格式关卡合为一个列表，并包含仍会读取的旧文件夹 `levels/community/` 和 `levels/open/` | |

文字提供全部七种语言。自 2026-11-11 起 Start 的顺序：先是我们自己的关卡，*经典 Ballance* 排第二。

**外观（Look）**（所有者 2026-11-11 的决定，decisions.md “One Look, and beauty comes through the toolbox”）是 Graphics 页上的一行
*外观*（设置 `look original|remaster|drahtseilakt|player`，`textures` 保存其所用的纹理包），取代了单独的 Textures 行和 2026-11-09
的 Options 行。*原版*：副本的纹理配 2004 年的造型。*重制版*：工具箱放大的纹理配圆润造型。*Drahtseilakt*（默认）：我们的纹理配圆润造型。
*自选包*：玩家自己的纹理包配圆润造型。此处无法显示的外观不提供（没有副本：没有原版和重制版；没有导出：没有重制版；没有自己的包：没有自选包）；
旧设置文件保持原意（`look remastered` 即重制版，否则由 `textures` 决定）。原版关卡的圆润造型是它的导出 `levels/remastered/Level_NN`（圆角缓存），
除原版外的所有外观都使用它；没有导出的关卡保持原样。该行的帮助和已下载地图的备注都会说明：漂亮的外观需先用 `drahtseilakt-toolbox` 导入关卡
（游戏从不联网，也不会自己运行它）。

## 放得下的文字，会说明自己可滚动的列表

*2026-11-03，根据项目所有者的意见（“文字并非处处都放得下”、“矮按钮里有两行”、“看不出模组只显示了一半”）。*

- **标签始终留在其按钮上**，在每种语言和每种窗口下都是如此（菜单在原版的 640×480 画面中排版并整体缩放，因此尺寸或宽高比永远不会改变能放下的内容）。
  过宽的标签会缩小绘制，最小到原大小的四分之三；然后，仅在按钮高度足以容纳该大小或更大的两行时（大按钮、输入框），在最靠近中间的空格处
  分成两行；否则继续缩小，最小到 60 %，仍放不下的部分从中间省略为“..”。矮按钮永远不会有两行。
- **我们自己的界面使用更小一号的字体**，比原版的小字体还小：Extended 页面、Mods 界面、模组的页面、自定义关卡和 Controls 页面的各行，
  这样最长的德语、俄语和法语标签也能放下，并能显示更多行（13 个关卡、10 个模组）。Mods 界面和模组页面下方的帮助是一个段落：它会换行，
  必要时缩小以放进其框内。这些界面的行在绘制时不带按键栏中间的标记（Controls 页面上把按键名称与按键分开的“≡”；放在标签下看起来像条纹）。
- **每个可滚动的列表都会说明这一点**：其各行旁有一条细滚动条，滑块的长度与显示的部分相当并位于其所在位置，第一行上方显示“▲ 还有 3 项”，
  最后一行下方显示“▼ 还有 12 项”。滚轮滚动列表（自定义关卡、模组、模组的页面），单击滚动条则滚动到该处。
- 测试 `every_label_fits_its_button_in_every_language_and_window`（menu.rs）以全部七种语言，在 640×480、1280×720、1920×1080、3440×1440
  和 1080×1920 下，使用自有界面的字体绘制每个界面（每个模组的页面、选中每个模组时的 Mods 界面），当标签离开其画面或按钮、绘制小于 60 %，
  或在小于四分之三大小时分成两行时即失败。

## 玩家档案

原版没有（见上文）。对我们来说，一个档案会把名字、设置、已解锁的关卡、高分榜和回放打包在一起，让几个人可以共用一台电脑。经典模式不需要它，
经典模式保留原版在高分登记中的单个名字。留给项目所有者的待定问题：玩家档案以后是否作为扩展模式的功能加入。

## 每一行的说明

每个屏幕上每个可选行都会在说明面板中告诉你它是什么、怎么工作、有什么用:主菜单、“开始”及其子菜单、“设置”及其所有页面(图形、操作、声音、语言、扩展)、暂停、死亡和结束菜单,以及“是/否”提问。带值的行(开关、选项:*外观*、*纹理*、*模型*、*音效*、*界面*、*全屏?*、*云层?*、*视野*、*扩展模式?*、*开放全部关卡?*、*最佳时间?* 等)还会解释当前选中的值,按左右键改变值时说明随之更新。玩家自己的纹理包、模型包或声音主题,若其 `pack.txt` 或 `theme.txt` 有 `description`(`description.<语言>`,否则 `description.en`),就显示它;模组显示它自己的描述。文字在 `assets/own/Texts/menu.txt` 中(`help <行>`、`help <行> <值>`),共七种语言;某一行或某个值缺少某种语言的文字时测试会失败,“所有屏幕适配”测试会遍历每一行和每个值。面板高度等于该屏幕最长的说明,因此上方的行不会随光标移动;为腾出空间,带面板的页面(图形、操作、声音、扩展)把各行画成条目,主菜单、设置和开始的按钮排得更紧凑。

## 工作顺序

1. 已完成（2026-11-02）：默认经典模式；按上表实现的主菜单和 Options；*Extended* 页面。
2. 已完成：带 Yes / No 询问的暂停菜单；死亡菜单；关卡结束菜单。
3. 已完成：带名字输入的高分榜；关卡解锁。
4. 已完成：自定义关卡列表（levels.md）和扩展页面上的开关。
5. 已完成：Graphics 和 Controls；手柄。

`DRAHTSEILAKT_SHOT_MENU=base:keys` 配合 `--screenshot` 可以在没有窗口的情况下把菜单画在画面上（base 为 `main`、`pause`、`dead` 或 `end`；
keys 中 `u d l r` 表示方向键，`e` 表示 Enter，`b` 表示 Esc，`n` 表示名字输入，`w` 表示在等待按键时按下 W 键），用来查看每一个界面。不在关卡之上的菜单画在界面的菜单 3D 场景前，场景取其镜头环绕一周中的第 `DRAHTSEILAKT_SHOT_MENU_MS` 毫秒（默认 0）。

**外观各值的名称**（所有者 2026-11-09 的决定）：凡是在原版游戏的东西、我们的东西和重制版之间选择的地方，值一律叫 **Ballance**（玩家的副本）、**Drahtseilakt**（我们的）和 **Remaster**（toolbox 的重制版），各语言相同（专有名称，不翻译）。只改了显示的名称：设置键和文件夹名仍是 `own`、`original`、`remastered`，已保存的设置照常有效。玩家的包或主题保留自己的名称，最多显示 16 个字符（[packs.md](packs.md)）。
