# 模组

模组为游戏增加行为和功能——粘性球、带分段的计时器、转动的视角——每个玩家可以逐一打开或关闭。它们采用我们自己的格式：
**纯数据**。一个模组是一个小小的规则文本文件，由引擎构件的固定库组成；游戏的运行时按规则所说的去做，就像原版的行为图是
由其运行时驱动的数据一样。模组中的任何东西都不会作为代码运行（decisions.md，“我们自己的纯数据模组格式”）。

游戏附带自己的模组，即*内置模组*（`assets/mods/`），正是用这种格式编写的：它们既是功能，也是模组作者的起点示例。玩家的
模组放在数据文件夹的 `mods/` 中。音效主题、纹理包和模型包以及关卡都不是模组：它们有各自的菜单和文件夹
（[sound-themes.md](sound-themes.md)、[packs.md](packs.md)、[levels.md](levels.md)）。

## 打开和关闭模组

*Options → Extended → Mods*（仅限扩展模式；经典模式不运行任何模组，与原版一样游玩）。该界面列出练习工具和无限生命（它们
以前位于 *Debug mode* 界面，并保留其设置 `extended.tools off`、`extended.endless-lives off`），然后是找到的每个模组：
先是内置模组，再是玩家的模组，每个显示 *Yes* 或 *No*，无法读取的模组显示 *refused*（列表下方的一行说明原因）。左右键
切换所选模组；Enter 打开它的页面：*Switched on?*、它的选项、它的描述、作者、版本和许可证，以及当它改变规则时的警告。
设置中保存 `mod.<key> on|off` 和 `mod.<key>.<option> <value>`。模组的视图部分立即生效；改变游玩的部分从关卡下一次开始时
生效。

## 内置模组

| 模组 | 仿照的 BML mod | 作用 | 改变游玩 |
|---|---|---|---|
| `sticky-ball` | BallSticky、StickyAccess | 球碰到的东西会粘住它；推向墙壁时会爬上去；选项：粘性、仅限木球 | 是（modded） |
| `physics-lab` | PhysicsEditor | 重力（0–60，原版 20）和物理的时间因子（0.5–4，原版 2） | 是（modded） |
| `dynamic-fov` | DynamicFov | 视野随球的速度变宽；选项：强度 | 否 |
| `free-view` | FreeViewRotation、AdvancedTravelCam | 鼠标让视角绕球转动，一个键把它转回；相机距离 | 否 |
| `segment-timer` | Segment、RealTimeMeter、WatchYourTime | 游玩时间、真实时间、每个小节相对最佳成绩的分段（领先为绿，落后为红） | 否 |
| `info-overlay` | PositionViewer、EnergyItemCounter、SysInfoDisplay | 位置、速度、本小节剩余的额外分和生命、每秒帧数、时钟（UTC） | 否 |
| `quick-restart` | SpeedrunShortcuts | 一个键（Backspace、Delete、F4 或 Q）立即重新开始关卡 | 否 |
| `screenshot` | ScreenCapturer | 一个键（F12、F10、Print Screen 或 P）把当前画面保存为 PNG，放在数据文件夹的 `screenshots/` 中 | 否 |
| `spirit-trail` | SpiritTrail | 关卡的最佳成绩以半透明球的形式在本次游玩旁边滚动；选项：不透明度 | 否 |
| `blackout` | Blackout | 关卡变暗，球上方有一束微光；选项：暗度、光 | 否 |

## modded 游玩与回放

改变游玩的模组（任何*游玩动作*，见下文）会使游玩成为 **modded**：它会被保存，但永远不算关卡的最佳成绩，永远不进高分榜，
永远不被幽灵球或分段拿来比较。它的回放在文件头中写明每个这样的模组——

    mod sticky-ball 9c41f0a2be1377d0 strength=60 wood_only=off

——它的键、它的 mod.txt 的哈希及其选项值，这样回放会重现同一次游玩，而且只与那个模组一起：模组缺失或已更改的回放会被拒绝
并说明原因。经典游玩的回放没有这样的行，与以前完全相同。只改变所见内容（相机、HUD、光）的模组让游玩保持经典。

## 编写你自己的模组

在数据文件夹的 `mods/` 中（Linux `~/.local/share/drahtseilakt/mods/`、Windows `%APPDATA%\drahtseilakt\mods\`、macOS
`~/Library/Application Support/drahtseilakt/mods/`）建一个名字简单的文件夹（字母、数字、`-`、`_`），并在其中放一个
`mod.txt`。打开扩展模式，在 Mods 界面中找到该模组。`assets/mods/` 中的内置模组就是示例；复制一个并修改即可。

最小的模组：

    name.en Moon gravity
    on start
    do world.gravity 4

### 文件

UTF-8 文本，每行一条语句，`#` 开始注释行。按以下顺序：

- **文件头** — `name.<lang>`（至少一个；语言 `en ru de zh fr es it`）、`description.<lang>`、`author`、`version`、
  `licence`、`requires <game version>`（如 `0.1`；比游戏更新的版本会导致游戏拒绝该模组）。
- **选项**，玩家在模组页面上设置：
  - `option <name> switch on|off`
  - `option <name> number <min> <max> <step> <default>`
  - `option <name> key <Key1,Key2,...>` — 按 winit 的名称指定按键（`KeyQ`、`F12`、`Backspace`、`Home`……）；第一个
    是默认值。
- **标签** — `label <name> <lang> <text>`：选项的标签与选项同名；HUD 行为自己的标签命名。
- **变量** — `var <name> <number>`：规则保存的数值，每次开始时从初始值开始。
- **规则** — `on <event>`，然后是任意条 `if <condition>` 行（必须全部成立），然后是按顺序执行的 `do <action>` 行。

名称（选项、标签、变量）由小写字母、数字和 `_` 组成，以字母开头。

### 事件

游玩的事件，按游玩的 tick 发生（确定性的）：`start`（第一个 tick 之前）、`tick`（每个 tick，在物理之前）、`sector`
（到达检查点）、`lost`（球丢失）、`newball`、`transform`、`extra`（拿到额外分或额外生命）、`finish`。

视图的事件：`frame`（每绘制一帧）、`key <option>`（按下模组某个按键选项的键）。

### 值

| 值 | | 值 | |
|---|---|---|---|
| `ball.x` `ball.y` `ball.z` | 球的位置 | `real.time` | 游玩开始以来的真实秒数 * |
| `ball.speed` `ball.vy` | 它的速度、它的竖直速度 | `frame.dt` `frame.fps` | 帧的时长、每秒帧数 * |
| `ball.kind` | 0 木球、1 石球、2 纸球 | `clock.hour` `clock.minute` `clock.second` | 时钟，UTC * |
| `ball.touching` `ball.wall` | 它碰到了某物；碰到了陡峭的东西 | `mouse.dx` `mouse.dy` | 本帧鼠标的移动 * |
| `run.tick` `run.time` | 游玩的 tick 数、秒数 | `extras.points` `extras.points_all` | 本小节剩余的额外分，以及全部 * |
| `run.sector` `run.lives` `run.points` | 小节（从 1 开始）、生命、分数 | `extras.lives` `extras.lives_all` | 剩余的额外生命，以及全部 * |
| `input.forward` `input.backward` `input.left` `input.right` `input.rotation` `input.overview` | 按住的游戏按键（1 或 0） | `best.time` | 关卡的最佳时间，秒（-1：无） * |

`opt.<name>` 是选项的值；变量按其名称读取。标有 * 的值属于视图：基于游玩事件的规则不能读取它们（游玩不能依赖所见的
内容），也不能读取视图规则所设置的变量。

### 表达式

数字、值、`+ - * /`、比较 `< <= > >= == !=`、`and or not`、`true false`、括号，以及函数 `min(a, b)`、`max(a, b)`、
`abs(x)`、`clamp(x, lo, hi)`、`if(c, a, b)`、`floor(x)`、`sqrt(x)`。条件不为 0 时成立。每个表达式都是全函数的：除以零
得 0，任何非有限数也得 0；没有循环；嵌套和大小都有上限。

### 动作

游玩动作（只能用于游玩的事件；它们使模组成为 *modded*）：

- `do ball.stick <0..1>` — 球以此强度粘住它碰到的东西，抵抗重力，推向墙壁时会沿墙向上转。
- `do world.gravity <0..60>` — 重力的拉力（原版为 20）。
- `do world.time <0.5..4>` — 物理的时间因子（原版为 2）。

视图动作（只能用于 `frame` 或 `key`）：

- `do camera.fov <degrees>` — 视野加宽这么多度（-30 到 60）。
- `do camera.distance <0.5..2>` — 相机拉近或拉远。
- `do camera.yaw <degrees>` — 视角绕球转动（方向键仍沿相机自身的朝向推动）。
- `do light <0..1>` — 关卡的光；`do lantern <0..1>` — 球上方的灯光。
- `do show <corner> <label|-> <format> <value>[, <value>[, <value>]]` — 本帧的一行 HUD；角落 `top-left top-right
  bottom-left bottom-right`；格式 `time`（由秒数得到 mm:ss.hh）、`int`、`dec1`、`dec2`、`clock`（hh:mm）、`ratio`（a/b）。
- `do splits <corner>` — 各小节相对最佳成绩的分段。
- `do ghost <0..1>` — 以半透明球显示最佳成绩，不透明度为此值。
- `do screenshot`、`do restart`。

两者皆可：`do set <variable> = <expression>`。视图规则设置的变量属于视图；游玩的规则不得设置或读取它。

### 示例

粘性球（`assets/mods/sticky-ball/mod.txt`，它的规则）：

    option strength number 10 100 10 60
    option wood_only switch off
    on tick
    do ball.stick if(opt.wood_only and ball.kind != 0, 0, opt.strength / 100)

自由视角：一个由鼠标转动的视图变量，一个把它转回的键：

    option sensitivity number 1 20 1 4
    option reset key KeyV,KeyB,KeyN,Home
    var yaw 0
    on frame
    do set yaw = yaw + mouse.dx * opt.sensitivity / 20
    do camera.yaw yaw
    on key reset
    do set yaw = 0

速度警告，仅在超过 30 时显示：

    name.en Speed warning
    label fast en Fast!
    on frame
    if ball.speed > 30
    do show bottom-right fast dec1 ball.speed

## 模组可以包含什么，以及上限

模组来自陌生人，所以游戏把它当作恶意内容读取，失败时一律拒绝：

- 模组文件夹只包含 `mod.txt`，最多再加一个 `readme.txt`/`README.md` 和一个 `licence.txt`/`license.txt`（每个
  64 KiB，游戏从不读取）。链接、子文件夹或任何其他文件——DLL、脚本、图片——都会导致整个模组被拒绝；
- mod.txt：UTF-8，最多 32 KiB 和 512 行；每一行都必须是格式认识的（未知的键、事件、值、动作、选项、角落或格式都会
  导致模组被拒绝，错误会指出该行）；文本最多 160 个字符，不含控制字符或双向覆盖字符；
- 最多 16 个选项、32 个变量、128 个标签、64 条规则，每条规则最多 8 个条件和 16 个动作，总共 256 个动作；一个表达式最多
  64 个部分、16 层深；一个按键选项最多 8 个键；每个文件夹最多 32 个模组；
- 没有路径、没有链接、没有文件名、没有网络：模组只能指名上述构件。

## 尚未实现（排队中）

- **同伴 / 两个球**（DualBallControl）：用按键切换到的第二个球需要游玩中的第二个玩家物体——它自己的一个构件，并把切换
  记录在回放中。
- **专门的粘性球种类**及其变球器（BallSticky 的 `P_Trafo_Sticky`）：目前任何球都可以变得有粘性。
- **随转动的视角转向**（FreeViewRotation 也会转动操控方向）：这需要把鼠标的转动记录在回放中；目前 `camera.yaw` 只转动
  视角。
- **诱饵球**（Interference）、**变形球**（DeformedWB）：用于额外球和网格噪声的绘制构件。
- **传送到出生点、在游玩中设置小节**（NewSpawn、SpeedrunShortcuts）：练习工具的 F5/F9 和 `--sector` 已覆盖练习用途；
  游玩中的构件会使其成为模组。
- **每关游戏时长**（WatchYourTime 的统计）：一个保存的计数器构件。
- mod.txt 的模糊测试目标（解析器严格、有上限并有单元测试；模糊测试随发布工作一起进行）。

## 调查过的 BML mod

三十个 BML/BML+ mod（`ref/Mods`，Windows DLL）经过静态阅读——只看它们的字符串和 readme；没有运行任何一个。
每个 mod 的作用、我们是否已有，以及它如何对应到我们的引擎：

| BML mod | 作用 | 我们的 |
|---|---|---|
| AdvancedTravelCam | 可以飞到任何地方的漫游相机，用鼠标选点，电影式平滑 | 练习工具的自由相机（F2）；`free-view`（距离、转动）；平滑排队中 |
| BallanceOptiFine | 为旧的自制地图强制开启阴影和过渡柱；纹理格式 | 渲染器在每个地面上绘制球的阴影；不是模组 |
| BaseCmoCfg | 编辑 base.cmo 的 GameSettings（God mode、Debug） | 扩展模式；Mods 界面上的无限生命 |
| Blackout | 让一切变暗，球上方有一束微光 | 内置 `blackout` |
| Coredump | 把运行中的 Virtools 上下文转储为 .nmo | 不需要（我们的运行时是自己的；`--export` 写出关卡） |
| CursorClipper | 把光标限制在窗口内 | 不需要：游玩不从光标获取任何东西，`mouse.dx` 读取原始移动 |
| DeformedWB | 随机使木球的外观变形 | 排队中 |
| DualBallControl | 双球地图，用一个键切换球 | 排队中（同伴构件） |
| DynamicFov | 视野随球的速度变化 | 内置 `dynamic-fov`；固定视野在 Graphics 中 |
| EnergyItemCounter | 本小节剩余的额外分和生命 | 内置 `info-overlay` |
| ExtraSector | 加载超过八个小节的地图 | 引擎：最多 64 个小节，无需加载器 |
| FontCraft | 界面的另一种字体 | 我们自己的界面和自由许可的字体，支持任何文字；不是模组 |
| FreeViewRotation | 鼠标转动视角 | 内置 `free-view`（仅视角；转向排队中） |
| Interference | 球周围的半透明诱饵球 | 排队中 |
| LanternFader | 对灯笼材质做 alpha 测试以防掉帧 | 不需要（我们的渲染器） |
| MapScripts | 运行地图的回调脚本 | 引擎把地图的行为图作为数据运行；不是模组 |
| NewSpawn | 设置出生点、传送到那里、录制灵魂球 | 练习工具（F5/F9、`--sector`、`--at`）；`spirit-trail` |
| PhysicsEditor | 重力、物理时间因子、游戏速度 | 内置 `physics-lab`（modded）；游戏速度是练习工具的 `[` `]` |
| PositionViewer | 球的坐标 | 内置 `info-overlay` |
| RealTimeMeter | 跨关卡的真实游戏时间 | 内置 `segment-timer`（真实时间） |
| ScreenCapturer | 一个键保存截图（PNG/JPEG、剪贴板） | 内置 `screenshot`（PNG；无剪贴板） |
| Segment | 每个小节相对纪录的时间 | 内置 `segment-timer` |
| SpeedrunShortcuts | 作弊模式、设置出生点、重开、设置小节 | 内置 `quick-restart`；练习工具；设置小节排队中 |
| SpiritTrail | 以半透明球显示纪录游玩 | 内置 `spirit-trail` |
| StickyAccess | 在作弊模式下切换到粘性球 | 内置 `sticky-ball` |
| SysInfoDisplay | 屏幕上显示系统时间和电量 | 内置 `info-overlay`（时钟，UTC）；无电量 |
| TASSupport | 录制和播放输入、跳过渲染、结束时退出 | 引擎：回放是确定性的输入录制（`--record`、`--replay`、`--headless`），以及练习工具的慢动作 |
| WatchYourTime | 每关游戏时长 | 排队中（统计） |
| BallSticky | 能爬墙的粘性球类型 | 内置 `sticky-ball`（modded） |
| BMLModuls | 新模块（木制落板和旋转板） | 关卡部件，不是模组：开放格式的部件（[open-format.md](open-format.md)） |

这些 mod 所绕开的原版限制（帧率上限、4:3、低分辨率、八个小节、位图字体、脆弱的变球时机）不是我们要照搬的。


## 审查后的补充说明

- 数值选项：上下限在 ±1 000 000 以内，步长不小于 0.001，默认值须落在从 `min` 起的步长上。
- 按键选项不能占用游戏自身的按键：Escape、Enter、方向键、Shift、Space、Tab、Alt、F11，以及练习工具的按键
  （R、F2、F3、F5、F9、`[` `]`）；玩家在 Controls 页面绑定的按键仍归游戏所有。
- `do screenshot` 与 `do restart` 只能用于 `on key` 规则（不能每帧执行）；每秒最多一张截图。
- 回放中模组选项缺失、未知或超出范围时，回放会被拒绝并给出原因。
- 计算预算：所有已启用模组的表达式节点总数不超过 20 000（读取模组时统计）；会超出预算的模组不会被启用，Mods 页面会说明原因。
