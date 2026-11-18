# 脚本：地图自带的行为图

自制地图可以携带自己的 Virtools 行为图（“脚本”）：地图作者模板里的火焰粒子、传送球的传送门、会隐藏、移动或旋转的对象、文字和声音。游戏按内置规则游玩每一张地图（起点、检查点、终点、模块）；脚本是在此之上的补充。本页是英文页 [scripts.md](../../drahtseilakt/scripts.md) 的简要镜像，完整的构建块表和逐图表格（[scripts-maps.csv](../../drahtseilakt/scripts-maps.csv)）在那里。

第一阶段（2026-11-09）：解释器核心、模板的构建块以及其余最常用的构建块；在真实地图上无窗口验证。代码：`crates/nmo/src/behavior.rs`（文件侧）、`crates/script`（图模型、运行时、构建块）、`crates/game/src/scripting.rs`（引擎侧）。格式参考：[ck-runtime.md](../../ck-runtime.md)。

第二阶段（2026-11-11）：“This”——脚本自身的对象（没有值的局部参数，只有空的 `0x200` 段）读作 `Owner`（开放格式写作 `["this", 0]`），运行时启动时绑定到脚本的对象；此前这样的参数会丢失。**Object Copy** 会生成副本：本次运行中的一个对象，带有原对象的位置、网格和包围盒、自己的名字（原名加三位编号；原版中名字的确切形式未知），以及原对象每个脚本的副本，绑定到该副本；每次运行最多 256 个副本（被拒绝的尝试也计数），被复制的脚本最多 16384 个节点；副本的节点从该 tick 的工作预算中支付，放不进预算或图的上限的副本整体被拒绝；从第一个副本起运行拥有自己的图，关卡的图不变。副本目前**不绘制，也没有物理体**。**Box Box Intersection** 对没有网格的对象，把它的位置当作空包围盒来检测。这样**传送门**地图的传送可以工作。组在运行中可变（Add To Group、Remove From Group、Group Clear）。渐变：Linear Progression、Bezier Progression（其 2D 曲线未解码：按直线）、Interpolator、Get / Set Component、TT_Timer。Random 与 Random Switch：运行状态里一个固定种子的生成器，因此运行与其回放一致。另有：Set Euler Orientation、Scale、Get Nearest In Group、Row Search、Remove Row、Object Rename。尚未完成：脚本对关卡对象的物理（被脚本移动的平台在新位置绘制，却在旧位置碰撞）、副本的绘制、Attach Script、Object Create、经声音引擎的声音。在此改动之前录制的带脚本地图的回放可能播放不同。

## 文件里有什么

一张行为图是几类对象的集合，各有自己的状态块：8 Behavior（脚本、图或带原型 GUID 的构建块；标志、优先级、目标参数，以及子行为、连线、运算、输入/输出/局部参数、输入口和输出口的列表）、6 BehaviorLink（以帧计的延迟、源 IO 和目标 IO）、9 BehaviorIO、2 ParameterIn（类型和来源）、3 ParameterOut 与 45 ParameterLocal（类型、保存的值，输出还有复制目标）、4 ParameterOperation（运算 GUID、两个输入、一个输出）、52 DataArray（表格），以及任何 CKBeObject 的脚本列表（标识符 `0x800`）。引脚名就是参数对象名，所以构建块按名称读取引脚。文件说不出的：场景启动时哪些脚本处于激活状态（全部按激活并重置启动），以及游戏自身的对象（`Ball_Pos_Frame`、`CurrentLevel` 表），由引擎作答。

## Stage 3.1 (2026-11-12): the run's entity table

（英文；翻译待补。）

One table, `crates/game/src/entities.rs`, owned by the run (`Scripts.ents`), read by the renderer and the scripts. Rows
0..N are the level's objects in the level's order; later rows are copies (Object Copy) and created objects (Object
Create). A row: name, class (level, copy of a row, created), parent, world transform, hidden, mesh, materials, alive.
It clones and rewinds with the run. Caps: `MAX_ENTITIES` rows in all (the same limit as the objects of the scripts) and
`MAX_DRAWN_EXTRA` (4096) copies and created objects drawn per frame; rows past it stay in the table, undrawn.

- **Hierarchy.** The level file's parent of each entity is read (`level::Object::parent`, from the entity chunk's
  parent reference); a loop or a parent that is no object is dropped. A child stores its transform relative to its
  parent and follows when the parent is moved or turned; moving the child itself sets its world place. Set Parent goes
  through `Host::set_parent` (a loop is refused); the block comes with stage 4.
- **Drawing.** Hidden or deleted rows are not drawn; a moved object is drawn at its transform (a child of a moved
  parent at its placement carried along); a copy is drawn with its source's mesh and materials at its own transform,
  only if the source is a plain mesh with a material (not a sprite, not the cloud layer; a copy of a collider is not
  seen). Object Create (`Host::create`, the block comes with stage 4) draws if it has a mesh, else it only exists.
- **Not here.** The collision and the ball read the level as before. The physics of moved and copied objects (static
  bodies added and removed live, followers of a moving parent) is the next task, 3.2. A level without scripts has no
  table and takes the old path unchanged.

## 解释器模型

`script::Graph` 是作为数据的文件行为图；`script::Runtime` 是状态（行为与 IO 的激活、连线的倒计时、参数值、表格行、消息）和一个 tick，如同原版行为管理器的一帧：每个激活的脚本执行；构建块读取并清除激活的输入、拉取引脚（运算值先算）、通过 `Host` 作用、写输出并激活输出口；图触发激活 IO 上的连线（无延迟立即、有延迟入队），按优先级处理激活子行为的栈，最后让延迟连线倒数。消息在下一 tick 到达等待者。脚本每 tick 在物理之前运行一次，确定性；格式 8 之前录制的回放不运行脚本，如录制时一样。无脚本的关卡不受影响。

引擎侧：对象的矩阵与可见性（渲染器读取的叠加层）、球的位置（对球执行 Set Position 会在该处放一个静止的新球）、包围盒、组、游戏的表格（`CurrentLevel`）、动作日志（`--headless` 以 `script:` 行打印）。第一阶段不做的：脚本对对象的物理（球除外）、对象复制、经引擎的文字与声音（目前只记日志）、游戏键位之外的按键、原版的 2D 界面。

## 敌意输入

所有列表有上限，所有索引经过检查，递归有界；未知构建块只报告一次并清除其输入，地图不会卡在它上面。读取器对文件数据绝不 panic。

## 地图现状

184 张带脚本的地图（连同 `community-unsorted/` 共 584 张）：148 张只有模板，36 张有自己的脚本。173 张的所有构建块均为运行时所知；36 张自带脚本的地图中 8 张的每个构建块完整生效（AAAAAA Class Islands Scenery、Be Besieged on All Sides 2.0、Painstaking Journey、Queen Ball (Easy Ver.)、Take the leap、Traveling on Rubber Walls、Trial of the Ballance King、反向Level_11；其中 Traveling on Rubber Walls 已于 2026-11-16 从收藏中删除，见下），17 张带有受限构建块（九张传送门地图：对象复制尚未实现，球的传送本身可用；脚本物理；纹理；按键）。11 张地图含未知构建块；工作顺序（组与渐变、脚本物理、复制、经引擎的声音与文字、外观、其余逻辑，“永不”：鼠标、字体、2D）见英文页。

*2026-11-16：* 三张地图的脚本引用了完全没有顶点的网格（它们是开放格式转换唯一没能保持一致的地图：导出把这种网格变成了普通对象）——20th Anniversary Commemorative Map、Fake Strength v4.4 和 Traveling on Rubber Walls——已从所有者的收藏（`ref/Maps`）中删除。上面的数字是删除之前普查的结果；转换器审计现在涵盖 589 张地图，全部一致（STATUS/convert-audit.md）。

## 第 0 阶段（2026-11-11）：核心一致性

**原型注册表**（`crates/script/src/blocks/`）：每个原型一条声明——GUID、名称、播放器的库、引脚模式、标志、完成程度、作为数据的旧引脚布局、运行函数和可选的事件函数。模块与播放器的库一一对应（`logics.rs` = Logics.dll、`narratives.rs` 等）。

**播放器中没有的。** 地图中出现的八种构建块（Explode、Midi Player、Run VSL、Add/Remove/Select Mesh、Get Nearest Object、Object Rename `6f074ca7:45427431`）不在播放器的任何库中：原版里它们什么也不做。它们被声明为**具名空操作**（所有者 2026-11-11 的决定）：已知、清除输入、无输出、记一行日志。第二阶段写的 Object Rename 代码保留在 cargo 特性 `absent-as-authored`（关闭）之后，留给以后一个可选的“按作者本意”模组。

**执行器**（`crates/script/src/exec.rs`），与原版的行为管理器一致：Activate Script 与 Deactivate Script 提交请求，由下一个 tick 在任何脚本运行之前应用（被另一个脚本激活的脚本晚一个 tick 开始）；重置与激活分开（不带 Reset? 再次激活的脚本从原处继续）；顺序：对象按优先级（`0x40`），对象的脚本按其列表；初始状态取自保存的场景活动（`0x200000` / `0x400`；地图中有 24 个脚本以停用状态开始），而不是行为标志；Deactivate Object 让对象休眠（其脚本不运行，状态保留），对对象执行 Object Delete 会连同其脚本一起删除；构建块回调（重置、脚本激活/停用、删除、复制）用于粒子发射器：停用脚本会立即撤下粒子；Group/Collection Iterator 把索引保存在输出 Index 或局部参数 0 中；Execute Script 在调用者的帧内立即执行脚本；可按开关输出跟踪（帧、行为运行、IO、参数写入）。带脚本地图的格式 9 回放被视为过时而拒绝（格式 10）；无脚本的关卡不受影响。

**普查**（`crates/script/src/census.rs`）：`script-dump --census 文件|文件夹...` 统计所有 `.nmo`/`.cmo` 并输出 CSV：汇总（文件、脚本、图、构建块、按延迟统计的连线、优先级、以停用状态开始的脚本），以及每种构建块一行（名称、GUID、注册表中的库与完成程度或 `?,unknown`、文件数、实例数）和每个运算一行。`script-dump --run N 文件` 在没有关卡的情况下运行文件的脚本 N 个 tick 并打印跟踪。由一个单元测试和 `crates/script/tests/census.rs`（设置了 `OPENBALLANCE_MAPS` 时）检验。

**C 预言机。** 旧的 C 运行时（`src/ck` + `src/bb`）仅作为测试参照保留（所有者 2026-11-11 的决定）：不复制其中任何代码，它只运行相同的图以便比较跟踪。第 1 级已就绪：`tests/oracle/ck_oracle.c` 根据文本描述构建由简单构建块（Nop、Binary Switch、Activate/Deactivate Script）组成的图，`crates/script/tests/oracle.rs` 生成随机图并逐帧比较跟踪（每帧运行了哪些构建块、顺序如何、帧后哪些处于激活状态）；5000 个图一致。构建与运行：`cmake -S . -B target/ck-oracle -DCMAKE_BUILD_TYPE=Release && cmake --build target/ck-oracle --target ck_oracle`，然后 `OPENBALLANCE_ORACLE=$PWD/target/ck-oracle/ck_oracle cargo test -p script --test oracle`。分歧由笔记裁定，而不是由预言机。

**C 预言机，第 2 级**（`crates/script/tests/oracle_blocks.rs`，`--test oracle_blocks`）：每种构建块一个合成图。预言机的描述现在支持任意原型的构建块及其输入、输出和局部参数（Boolean、Integer、Float、Time、String、消息、对象、脚本、组）、组、连到任意输入的链接，并在每帧后打印构建块的全部输出参数。每个用例把构建块放在一个脚本中，每个输出接一个 Nop 探针；其输入从脚本开始以边界延迟（0、1、2……13 帧）触发，输出回接到输入（包括延迟 0：同一帧内重入），第二个脚本在随机帧重置、停用并重新激活第一个脚本；引脚取边界值（负、零和小数时间，-2 到 7 的计数和步长，运算符 0-7，空组）。已覆盖：Delayer、Timer、Counter、Sequencer、Binary Switch、Wait For All、Identity、Parameter Selector、Binary Memory、Test、Switch On Parameter、Per Second、Group Iterator、Wait Message 与 Send Message、Op（Integer 与 Float 算术，包括除以 0）、Get Cell、Iterator、Get Key Row（Integer 列的数组，包括空数组）、Activate Script 和 Deactivate Script（作为驱动）；每种 2000 个用例一致。出现分歧的构建块现已写入 [gameplay_bbs.md](../../gameplay_bbs.md) 第 5 部分，我们的实现遵循该记录：Delayer 计入 In 到达的那一帧，并把时间保存在 Elapsed Time 中；Timer 从不保持激活（循环由图完成）；Counter 和 Sequencer 把值保存在输出参数 0 中（Sequencer 之前按 Streaming Event 运行，Wait For All 按 Nop）；构建块不处理的输入保持激活；Identity 在引脚与输出数量不等时什么也不做；Wait Message 登记等待后即结束，等待使其图保持激活，消息送达时触发 Out，与原版的消息管理器一致；Iterator 把行号保存在 Row Index 中并绕数组循环；Get Key Row 在数组自身的键列中查找（现从文件读取，标识 `0x4000`），而不是第一列，没有键列时什么也不做。Execute Script 不作比较：C 提交请求，而按我们的笔记脚本立即执行。C 在停止循环前比 BehaviorMaxIteration 多弹出一次栈；笔记说至多这么多次，因此触及上限的循环不作比较。下一步是第 3 级（整张地图）。

## 第 0b 阶段（2026-11-11）：类型、运算、消息

- **参数类型**（`crates/script/src/types.rs`）：GUID → 名称、种类、父类型、大小、默认值的注册表
  （[ck-runtime.md](../../ck-runtime.md)，“Parameter types”）；值新增四元数、矩形、包围盒和对象数组（开放格式写作
  `["q", ...]`、`["rect", ...]`、`["box", ...]`、`["oa", [...]]`）。
- **运算即数据**（`crates/script/src/ops.rs`）：按（运算、输出、输入 1、输入 2）索引的行，查找时像原版参数管理器一样
  依次尝试类型的父类型和交换输入（[ck-runtime.md](../../ck-runtime.md)，“Operations”）。社区地图和游戏中的 142 个
  签名有 114 个有函数；其余 28 个（16 个既无名称也无 C 函数的 GUID，见于四张地图）只记录一次日志：图中的运算把输入 1
  传下去，Op 块不写任何值。字符串转成对象时，只找输出类型所属类的对象。`script-dump --census` 列出每个签名及回答它的
  行，并按地图列出缺少的块和签名。C 预言机检查值运算的行（`op_table`）。
- **消息**带有发送者、接收者（或广播的类）和数据：新增 Broadcast Message、Switch On Message 和 Get Message Data
  （[gameplay_bbs.md](../../gameplay_bbs.md)，“Messages”）。
- **数组**（`crates/script/src/blocks/arrays.rs`）：带类型的键和单元格，以及 Set Row、Insert/Remove Column、Move Row、
  Sort Rows、Test Cell、Get Highest、Value Count、Remove Key Row、Remove Row If、Change Value If、Row Search；每次运行自带
  每个数组的列和键列。除 C 中没有的 Move Row 外，所有列类型都已对照 C 预言机检验
  （[gameplay_bbs.md](../../gameplay_bbs.md)，“Arrays”）。
- **组**可含图中的任意对象（实体、网格、材质、纹理、行为）：成员取自文件，运行自行保存（随回滚），Fill Group By Class 按类收集
  （[gameplay_bbs.md](../../gameplay_bbs.md)，“Fill Group By Class”）。对由实体组成的组，Group Iterator 和 Is In Group 的结果与以前相同。
- **属性**（`crates/script/src/blocks/attributes.rs`）：Has、Set、Remove Attribute 和 Objects With Attribute Iterator 作用于运行中每个对象的属性表
  （先是文件自带的，从标识符 `0x11` 谨慎读取，见 [ck-runtime.md](../../ck-runtime.md)；有上限；随运行回滚）。属性的名称和类型需要属性管理器的
  数据块，目前仍被跳过：属性即其文件索引。

**输入**（格式 11）：整个键盘（DirectInput 扫描码）和鼠标（原版 640x480 画面内的位置、按键）属于一次运行的输入流；游戏把它们的变化记入回放（`keys` 与 `mouse` 行），回放时脚本得到记录的输入，而不是实机输入。格式 10 及更早的回放给脚本空键盘和静止的鼠标。模块：Key Event、Key Waiter、Switch On Key、Mouse Waiter、Input String（最多 256 个字符）、Show Mouse Cursor、Mouse Cursor、Mouse Cursor System（仅窗口光标）。
