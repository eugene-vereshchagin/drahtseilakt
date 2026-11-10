# 模组

模组为游戏增加行为和功能——粘性球、带分段的计时器、转动的视角——每个玩家可以逐一打开或关闭。它们采用我们自己的格式：
**纯数据**。一个模组是一个小小的规则文本文件，由引擎构件的固定库组成；游戏的运行时按规则所说的去做，就像原版的行为图是
由其运行时驱动的数据一样。模组中的任何东西都不会作为代码运行（decisions.md，“我们自己的纯数据模组格式”）。

游戏附带自己的模组，即*内置模组*（`assets/mods/`），正是用这种格式编写的：它们既是功能，也是模组作者的起点示例。玩家的
模组放在数据文件夹的 `mods/` 中。音效主题、纹理包和模型包以及关卡都不是模组：它们有各自的菜单和文件夹
（[sound-themes.md](sound-themes.md)、[packs.md](packs.md)、[levels.md](levels.md)）。

## 打开和关闭模组

*Options → Extended → Mods*（仅限扩展模式；经典模式不运行任何模组，与原版一样游玩）。该界面列出找到的每个模组：
先是内置模组，调试运行中接着是开发用模组（上帝模式，见下文），再是玩家的模组，每个显示 *Yes* 或 *No*，无法读取的模组显示
*refused*（列表下方的帮助说明原因）。较长的列表可以滚动，旁边有滚动条和“▲ / ▼ 还有 N 项”（menu.md）。左右键切换所选
模组；Enter 打开它的页面：*Switched on?*、它的选项（按键选项显示其按键）、它的描述、作者、版本和许可证，以及当它改变规则
时的警告。
设置中保存 `mod.<key> on|off` 和 `mod.<key>.<option> <value>`。模组的视图部分立即生效；改变游玩的部分从关卡下一次开始时
生效。

## 内置模组

| 模组 | 仿照的 BML mod | 作用 | 改变游玩 |
|---|---|---|---|
| `sticky-ball` | BallSticky、StickyAccess | 球碰到的东西会粘住它；推向墙壁时会爬上去；选项：粘性、仅限木球 | 是（modded） |
| `physics-lab` | PhysicsEditor | 重力向量：向下（0–60，原版 20）以及沿世界 x、z 轴的侧向分量（各 −20–20，原版 0）——以及物理的时间因子（0.5–4，原版 2） | 是（modded） |
| `dynamic-fov` | DynamicFov | 视野随球的速度变宽；选项：强度 | 否 |
| `free-view` | FreeViewRotation、AdvancedTravelCam | 鼠标让视角绕球转动，一个键把它转回；相机距离 | 否 |
| `segment-timer` | Segment、RealTimeMeter、WatchYourTime | 游玩时间、真实时间、每个小节相对最佳成绩的分段（领先为绿，落后为红） | 否 |
| `info-overlay` | PositionViewer、EnergyItemCounter、SysInfoDisplay | 位置、速度、本小节剩余的额外分和生命、每秒帧数、时钟（UTC） | 否 |
| `quick-restart` | SpeedrunShortcuts | 一个键（Backspace、Delete、F4 或 Q）立即重新开始关卡 | 否 |
| `screenshot` | ScreenCapturer | 一个键（F12、F10、Print Screen 或 P）把当前画面保存为 PNG，放在数据文件夹的 `screenshots/` 中 | 否 |
| `spirit-trail` | SpiritTrail | 关卡的最佳成绩以半透明球的形式在本次游玩旁边滚动；选项：不透明度 | 否 |
| `blackout` | Blackout | 关卡变暗，球上方有一束微光；选项：暗度、光 | 否 |
| `smooth-camera` | AdvancedTravelCam（电影式平滑） | 镜头在转动、距离和视野上平滑过渡，自由相机滑行；选项：平滑度 | 否 |
| `interference` | Interference | 半透明的诱饵球绕球游走；选项：数量 | 否 |
| `deformed-ball` | DeformedWB | 木球的外观随机被压扁、拉长（物理仍是圆球）；选项：程度 | 否 |
| `back-to-2004` | （玩笑；撤销我们的画面改进） | 像2004年的电脑：低分辨率（640x480、800x600或1024x768）的硬像素，4:3画面带黑边，纹理无mipmap和各向异性（双线性或最近邻：远处闪烁），帧率上限（关、60、30），可选16位色彩条纹；附赠一副用于流血泪的眼镜 | 否 |
| `play-time` | WatchYourTime | 每关和总计的游戏时长，显示在屏幕上，并保存在数据文件夹的 `playtime.txt`；选项：是否显示 | 否 |
| `sector-jump` | SpeedrunShortcuts（设置小节） | 按键（F7 / PageUp、F6 / PageDown）从下一个或上一个小节重新开始关卡 | 是（modded） |
| `spawn-point` | NewSpawn、SpeedrunShortcuts（设置出生点） | 一个键（F8、H 或 J）在球所在处设出生点，另一个键（G、F4 或 C）把球静止地送回那里，恢复当时的球种；小节及其物体保持原样 | 是（modded） |
| `free-steering` | FreeViewRotation（含操控） | 类似 `free-view`，但方向键沿转动后的视角推动；视角的转动记录在回放中 | 是（modded） |

## 上帝模式

*2026-11-03，所有者的决定*（decisions.md，“上帝模式是一个开发用模组”）。原来扩展模式的练习工具和无限生命合成一个模组
`god-mode`，正是用这种格式、由下文的构件编写的。它是一个**开发用模组**：放在 `assets/mods-dev/` 中，游戏只有以 `--debug`
运行时才读取该文件夹，发布包不包含它，因此游戏在没有它的情况下游玩。它的页面（Options → Extended → Mods → God mode）显示
它的开关和按键；开启时，它的按键显示在左下角（选项 *Keys on screen?*），小节和 tick 显示在左上角。

| 选项 | 默认 | 作用 |
|---|---|---|
| Endless lives? | yes | 最后一个球永远不会丢失 |
| Seconds back | 5 | *Back* 后退多远（1–30） |
| Back | F3（B、Z） | 后退这么多秒；再按一次，再往前退（游戏保留最近半分钟，每秒一个时刻） |
| Save / Load | F5（K）/ F9（L） | 保存当前时刻，回到该时刻；失败的游玩会等待 Load |
| Restart | R（T） | 重新开始关卡 |
| Slower / Faster | `[`（`-`）/ `]`（`=`） | 游戏以一半的速度运行，最低到八分之一，以及恢复 |
| Free camera | F2（C） | 相机自由飞行（方向键飞行和转向，A/D 横移，PageUp/PageDown，Home/End，Shift 加快）；游玩暂停等待 |
| Keys on screen? | yes | 显示按键的提示 |

上帝模式会改变游玩：它的游玩是 modded（见下文）——会被保存，但不解锁任何关卡，永远不算最佳成绩或高分——其回放写明该模组，
因此回放只能在启用相同上帝模式的调试运行中播放。旧设置 `extended.tools` 和 `extended.endless-lives` 不再被读取。

## modded 游玩与回放

改变游玩的模组（任何*游玩动作*，见下文）会使游玩成为 **modded**：它会被保存，但永远不算关卡的最佳成绩，永远不进高分榜，
永远不被幽灵球或分段拿来比较，也不会解锁下一关（`Level_NN-<ticks>-modded.replay`；调试运行的 “All levels open” 可解锁全部）。它的回放在文件头中写明每个这样的模组——

    mod sticky-ball 9c41f0a2be1377d0 strength=60 wood_only=off

——它的键、它的 mod.txt 的哈希及其选项值，这样回放会重现同一次游玩，而且只与那个模组一起：模组缺失或已更改、或者选项缺失、未知或超出范围的回放会被拒绝
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
  - `option <name> number <min> <max> <step> <default>`（在 ±1,000,000 以内，步长不小于 0.001，默认值须落在从 `min`
    起的步长上）
  - `option <name> key <Key1,Key2,...>` — 按 winit 的名称指定按键（`KeyQ`、`F12`、`Backspace`、`Home`……）；第一个
    是默认值。游戏自身的按键会被拒绝：Escape、Enter、方向键、Shift、Space、Tab、Alt、F11 和 Backquote；玩家在
    Controls 页面绑定的按键仍归游戏所有。（R、F2、F3、F5、F9、`[` `]` 原是练习工具的按键；现在它们是上帝模式的按键，
    任何模组都可以使用。）
  - `option <name> choice <default> <name1> <name2> ...` — 2 到 12 个有名字的选项之一（小写字母、数字、`_`）；规则里该选项的值是选择的序号（第一个为 0），如 `if opt.quality == 1`；设置和回放保存名字（`mod.<mod>.<option> <name>`、`<option>=<name>`）。每个选择需要 `label <option>.<name> <lang> <text>`（必须有英文）：模组页把该标签显示为值；请保持简短，说明写进 description。
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
| `practice.speed` | 游戏的速度（1，最低到 1/8） * | `practice.saved` | 已保存一个时刻（1 或 0） * |
| `run.sector` `run.lives` `run.points` | 小节（从 1 开始）、生命、分数 | `extras.lives` `extras.lives_all` | 剩余的额外生命，以及全部 * |
| `input.turn` | 由 `view.turn` 传给游玩的视角转动，度 | `playtime.level` `playtime.all` | 本关、所有关卡已玩的秒数 * |
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
- `do world.gravity_xz <x> <z>` — 沿世界 x、z 轴的侧向重力，各 −60 到 60（原版为 0、0；两个不含空格的表达式）。与 `world.gravity` 一起，重力成为向量，如 PhysicsEditor 所设。
- `do world.time <0.5..4>` — 物理的时间因子（原版为 2）。
- `do lives.endless <0|1>` — 最后一个球永远不会丢失（god-mode）。
- `do steer.yaw <度>` — 方向键的推力绕竖直轴转过这么多度（相机以同样的旋转转动，所以方向键沿转动后的视角推动）。
  配合 `input.turn`，它跟随视角（见下）。

练习动作（只能用于 `key`；它们从游玩外部介入游玩，因此含有这类动作的模组同样是 modded，并且在它开启时，游戏会保存游玩的
时刻，最近半分钟内每秒一个）：

- `do rewind <1..30>` — 后退这么多秒；`do save`、`do load` — 保存当前时刻，回到该时刻；
- `do slower`、`do faster` — 游戏速度减半（最低到 1/8）或加倍（最高恢复原速）；
- `do freecam` — 开启或关闭自由相机。

跳转（仅用于 `key`；使模组成为 modded，但不保留练习的时刻）：

- `do sector.set <n>` — 游玩从小节 n（1 为第一个）重新开始，使用该小节的球，如同 `--sector`；回放记录起点。
  下一次普通重开又是关卡的起点。

出生点（仅在 `key` 上；使 mod 成为 modded，并归入游玩一侧，回放会记录它）：

- `do spawn.set` — 在球所在处设出生点（位置、朝向、球种）；
- `do spawn.go` — 球静止地回到出生点，恢复当时的球种；小节及其物体保持原样（NewSpawn 的简单传送）。球丢失、在变球器中或正在变球时两者都不起作用。

  两者都由游玩本身完成：按键在一个 tick 内按下一个输入位（64 设点，128 传送），回放与方向键一起保存它，因此回放结果相同。没有这样的 mod 时游玩忽略这些位。

视图动作（只能用于 `frame` 或 `key`）：

- `do camera.fov <degrees>` — 视野加宽这么多度（-30 到 60）。
- `do camera.distance <0.5..2>` — 相机拉近或拉远。
- `do camera.yaw <degrees>` — 视角绕球转动（方向键仍沿相机自身的朝向推动）。
- `do render.scale <宽> <高>` — 以此大小绘制画面（160x120 至 4096x4096，不超过窗口），再用最近邻像素放大到窗口。`do render.aspect <宽>:<高>` — 画面保持此比例，窗口更宽或更高处为黑边。
- `do render.filter default|bilinear|nearest` — 纹理过滤：我们的（mipmap、各向异性）、仅取最大级的双线性（无mipmap，远处闪烁）或最近邻纹素。
- `do render.fps_cap <n>` — 每秒最多 n 帧（0：不限；对局的 tick 仍按自己的时钟）。`do render.colour_bits 16` — 颜色截为 5-6-5 位且不抖动（其他值：全彩）。
  这些动作只影响画面：对局保持经典，回放不变。游戏中没有可撤销的边缘抗锯齿。
- `do light <0..1>` — 关卡的光；`do lantern <0..1>` — 球上方的灯光。
- `do show <corner> <label|-> <format> <value>[, <value>[, <value>]]` — 本帧的一行 HUD；角落 `top-left top-right
  bottom-left bottom-right`；格式 `time`（由秒数得到 mm:ss.hh）、`int`、`dec1`、`dec2`、`clock`（hh:mm）、`ratio`（a/b）。
- `do splits <corner>` — 各小节相对最佳成绩的分段。
- `do keys <corner>` — 以 HUD 行显示模组自己的按键选项，“标签: 按键”（上帝模式的提示）。
- `do ghost <0..1>` — 以半透明球显示最佳成绩，不透明度为此值。
- `do screenshot`、`do restart` — 只能用于 `key`（不能每帧执行）；每秒最多一张截图。

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
- 计算预算：所有一起启用的模组的表达式节点总数不超过 20,000（每个模组的节点在读取时统计）；会超出预算的模组不会被启用，
  Mods 页面会说明原因；
- 没有路径、没有链接、没有文件名、没有网络：模组只能指名上述构件。

## 尚未实现（排队中）

- **同伴 / 双球**（DualBallControl）。从其字符串可见它的做法：在带第二个起点（`PS_FourFlames_01_Dual`，并有自己的
  `PC_TwoFlames_0*` / `PR_Resetpoint_0*`）的地图上，它再次加载 `Balls.nmo`，物理化第二个游戏球和第二个 `Cam_MF`，
  切换键关闭一个球的操控并把另一个设为 BallManager 的球。我们需要（全部在 Sim 中）：第二个球的状态和刚体（如今球是世界的
  实体 0：`replace_ball(0, ..)`、实体 1 的接触、推力、粘力），未激活的球作为被动动态刚体，小节的检查点、重置点、死亡、
  变球器和额外生命作用于哪个球，第二套相机或一套跳到另一个球的相机，以及作为回放保存的输入位的切换（同出生点）。
  手头的社区地图中尚不知有双起点的（文件是压缩的，简单搜索说明不了什么）；没有可玩的地图，此改动等到出现这样的地图再做。
- **独立的粘性球类型**及其变球器（BallSticky 附带 `Ball_Sticky.nmo`、`PH/P_Ball_Sticky.nmo` 和
  `PH/P_Trafo_Sticky.nmo`）。行为已经有了（`ball.stick` 配合 `ball.kind`）；工作在于第四种 `BallKind`：它的物理参数
  （在他们的 `P_Ball_Sticky` 里，不在 DLL 字符串中；需测量或选定），它在 `BallKind::from_name` 中的名字（变球器组
  `P_Trafo_<种类>` 和小节球随之而来），以及每个按种类分支处的一支：球的网格（scene.rs `Ball_*_Mesh`、app.rs 中的幽灵）、
  碎片（pieces.rs）、滚动与撞击声（ballsound.rs、audio.rs）、该种类的散球、自动驾驶、提示和部件检查；还有我们自己的
  美术——一个球、它的碎片和一个变球器模块——因为不附带他们的任何东西。留待带有这些部件的改动。
- mod.txt 的 fuzz 目标（解析器严格、有上限并有单元测试；fuzz 随发布工作一起做）。

## 调查过的 BML mod

三十个 BML/BML+ mod（`ref/Mods`，Windows DLL）经过静态阅读——只看它们的字符串和 readme；没有运行任何一个。
每个 mod 的作用、我们是否已有，以及它如何对应到我们的引擎：

| BML mod | 作用 | 我们的 |
|---|---|---|
| AdvancedTravelCam | 可以飞到任何地方的漫游相机，用鼠标选点，电影式平滑 | 上帝模式的自由相机（F2）；`free-view`（距离、转动）；内置 `smooth-camera`（电影式平滑） |
| BallanceOptiFine | 为旧的自制地图强制开启阴影和过渡柱；纹理格式 | 渲染器在每个地面上绘制球的阴影；不是模组 |
| BaseCmoCfg | 编辑 base.cmo 的 GameSettings（God mode、Debug） | `--debug` 及其上帝模式（无限生命等） |
| Blackout | 让一切变暗，球上方有一束微光 | 内置 `blackout` |
| Coredump | 把运行中的 Virtools 上下文转储为 .nmo | 不需要（我们的运行时是自己的；`--export` 写出关卡） |
| CursorClipper | 把光标限制在窗口内 | 不需要：游玩不从光标获取任何东西，`mouse.dx` 读取原始移动 |
| DeformedWB | 随机使木球的外观变形 | 内置 `deformed-ball` |
| DualBallControl | 双球地图，用一个键切换球 | 排队中（需要第二个玩家刚体；具体见上文） |
| DynamicFov | 视野随球的速度变化 | 内置 `dynamic-fov`；固定视野在 Graphics 中 |
| EnergyItemCounter | 本小节剩余的额外分和生命 | 内置 `info-overlay` |
| ExtraSector | 加载超过八个小节的地图 | 引擎：最多 64 个小节，无需加载器 |
| FontCraft | 界面的另一种字体 | 我们自己的界面和自由许可的字体，支持任何文字；不是模组 |
| FreeViewRotation | 鼠标转动视角 | 内置 `free-view`（仅视角），`free-steering`（转向跟随；modded） |
| Interference | 球周围的半透明诱饵球 | 内置 `interference` |
| LanternFader | 对灯笼材质做 alpha 测试以防掉帧 | 不需要（我们的渲染器） |
| MapScripts | 运行地图的回调脚本 | 计划中：地图脚本解释器（scripts.md）；不是模组 |
| NewSpawn | 设置出生点、传送到那里、录制灵魂球 | 内置 `spawn-point`（modded）；上帝模式（F5/F9）、`--sector`、`--at`；`spirit-trail` |
| PhysicsEditor | 重力（向量）、物理时间因子、游戏速度 | 内置 `physics-lab`（modded；向下与侧向重力）；游戏速度是上帝模式的 `[` `]` |
| PositionViewer | 球的坐标 | 内置 `info-overlay` |
| RealTimeMeter | 跨关卡的真实游戏时间 | 内置 `segment-timer`（真实时间） |
| ScreenCapturer | 一个键保存截图（PNG/JPEG、剪贴板） | 内置 `screenshot`（PNG；无剪贴板） |
| Segment | 每个小节相对纪录的时间 | 内置 `segment-timer` |
| SpeedrunShortcuts | 作弊模式、设置出生点、重开、设置小节 | 内置 `quick-restart`；上帝模式；`spawn-point`（设置出生点；modded）；`sector-jump`（设置小节；modded） |
| SpiritTrail | 以半透明球显示纪录游玩 | 内置 `spirit-trail` |
| StickyAccess | 在作弊模式下切换到粘性球 | 内置 `sticky-ball` |
| SysInfoDisplay | 屏幕上显示系统时间和电量 | 内置 `info-overlay`（时钟，UTC）；无电量 |
| TASSupport | 录制和播放输入、跳过渲染、结束时退出 | 引擎：回放是确定性的输入录制（`--record`、`--replay`、`--headless`），以及上帝模式的慢动作 |
| WatchYourTime | 每关游戏时长 | 内置 `play-time`（`playtime.txt`；另有 `segment-timer`） |
| BallSticky | 能爬墙的粘性球类型 | 内置 `sticky-ball`（modded；任意球或仅木球）；带变球器的独立种类排队中 |
| BMLModuls | 新模块（木制落板和旋转板） | 关卡部件，不是模组：开放格式的部件（[open-format.md](open-format.md)） |

这些 mod 所绕开的原版限制（帧率上限、4:3、低分辨率、八个小节、位图字体、脆弱的变球时机）不是我们要照搬的。

**显示的名称。** 模组的 `name.<语言>` 最多显示 40 个字符：更长的在词边界处截断并以“…”结尾，游戏会在日志中给出警告。
