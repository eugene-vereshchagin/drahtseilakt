# 脚本：地图自带的行为图

自制地图可以携带自己的 Virtools 行为图（“脚本”）：地图作者模板里的火焰粒子、传送球的传送门、会隐藏、移动或旋转的对象、文字和声音。游戏按内置规则游玩每一张地图（起点、检查点、终点、模块）；脚本是在此之上的补充。本页是英文页 [scripts.md](../../drahtseilakt/scripts.md) 的简要镜像，完整的构建块表和逐图表格（[scripts-maps.csv](../../drahtseilakt/scripts-maps.csv)）在那里。

第一阶段（2026-11-09）：解释器核心、模板的构建块以及其余最常用的构建块；在真实地图上无窗口验证。代码：`crates/nmo/src/behavior.rs`（文件侧）、`crates/script`（图模型、运行时、构建块）、`crates/game/src/scripting.rs`（引擎侧）。格式参考：[ck-runtime.md](../../ck-runtime.md)。

第二阶段（2026-11-11）：“This”——脚本自身的对象（没有值的局部参数，只有空的 `0x200` 段）读作 `Owner`（开放格式写作 `["this", 0]`），运行时启动时绑定到脚本的对象；此前这样的参数会丢失。**Object Copy** 会生成副本：本次运行中的一个对象，带有原对象的位置、网格和包围盒、自己的名字（原名加三位编号；原版中名字的确切形式未知），以及原对象每个脚本的副本，绑定到该副本；每次运行最多 256 个副本（被拒绝的尝试也计数），被复制的脚本最多 16384 个节点；副本的节点从该 tick 的工作预算中支付，放不进预算或图的上限的副本整体被拒绝；从第一个副本起运行拥有自己的图，关卡的图不变。副本目前**不绘制，也没有物理体**。**Box Box Intersection** 对没有网格的对象，把它的位置当作空包围盒来检测。这样**传送门**地图的传送可以工作。组在运行中可变（Add To Group、Remove From Group、Group Clear）。渐变：Linear Progression、Bezier Progression（其 2D 曲线未解码：按直线）、Interpolator、Get / Set Component、TT_Timer。Random 与 Random Switch：运行状态里一个固定种子的生成器，因此运行与其回放一致。另有：Set Euler Orientation、Scale、Get Nearest In Group、Row Search、Remove Row、Object Rename。尚未完成：脚本对关卡对象的物理（被脚本移动的平台在新位置绘制，却在旧位置碰撞）、副本的绘制、Attach Script、Object Create、经声音引擎的声音；测试玩家（`--autopilot`）不认识传送门。在此改动之前录制的带脚本地图的回放可能播放不同。

## 文件里有什么

一张行为图是几类对象的集合，各有自己的状态块：8 Behavior（脚本、图或带原型 GUID 的构建块；标志、优先级、目标参数，以及子行为、连线、运算、输入/输出/局部参数、输入口和输出口的列表）、6 BehaviorLink（以帧计的延迟、源 IO 和目标 IO）、9 BehaviorIO、2 ParameterIn（类型和来源）、3 ParameterOut 与 45 ParameterLocal（类型、保存的值，输出还有复制目标）、4 ParameterOperation（运算 GUID、两个输入、一个输出）、52 DataArray（表格），以及任何 CKBeObject 的脚本列表（标识符 `0x800`）。引脚名就是参数对象名，所以构建块按名称读取引脚。文件说不出的：场景启动时哪些脚本处于激活状态（全部按激活并重置启动），以及游戏自身的对象（`Ball_Pos_Frame`、`CurrentLevel` 表），由引擎作答。

## 解释器模型

`script::Graph` 是作为数据的文件行为图；`script::Runtime` 是状态（行为与 IO 的激活、连线的倒计时、参数值、表格行、消息）和一个 tick，如同原版行为管理器的一帧：每个激活的脚本执行；构建块读取并清除激活的输入、拉取引脚（运算值先算）、通过 `Host` 作用、写输出并激活输出口；图触发激活 IO 上的连线（无延迟立即、有延迟入队），按优先级处理激活子行为的栈，最后让延迟连线倒数。消息在下一 tick 到达等待者。脚本每 tick 在物理之前运行一次，确定性；格式 8 之前录制的回放不运行脚本，如录制时一样。无脚本的关卡不受影响。

引擎侧：对象的矩阵与可见性（渲染器读取的叠加层）、球的位置（对球执行 Set Position 会在该处放一个静止的新球）、包围盒、组、游戏的表格（`CurrentLevel`）、动作日志（`--headless` 以 `script:` 行打印）。第一阶段不做的：脚本对对象的物理（球除外）、对象复制、经引擎的文字与声音（目前只记日志）、游戏键位之外的按键、原版的 2D 界面。

## 敌意输入

所有列表有上限，所有索引经过检查，递归有界；未知构建块只报告一次并清除其输入，地图不会卡在它上面。读取器对文件数据绝不 panic。

## 地图现状

184 张带脚本的地图（连同 `community-unsorted/` 共 584 张）：148 张只有模板，36 张有自己的脚本。173 张的所有构建块均为运行时所知；36 张自带脚本的地图中 8 张的每个构建块完整生效（AAAAAA Class Islands Scenery、Be Besieged on All Sides 2.0、Painstaking Journey、Queen Ball (Easy Ver.)、Take the leap、Traveling on Rubber Walls、Trial of the Ballance King、反向Level_11），17 张带有受限构建块（九张传送门地图：对象复制尚未实现，球的传送本身可用；脚本物理；纹理；按键）。11 张地图含未知构建块；工作顺序（组与渐变、脚本物理、复制、经引擎的声音与文字、外观、其余逻辑，“永不”：鼠标、字体、2D）见英文页。

## 第 0 阶段（2026-11-11）：核心一致性

**原型注册表**（`crates/script/src/blocks/`）：每个原型一条声明——GUID、名称、播放器的库、引脚模式、标志、完成程度、作为数据的旧引脚布局、运行函数和可选的事件函数。模块与播放器的库一一对应（`logics.rs` = Logics.dll、`narratives.rs` 等）。

**播放器中没有的。** 地图中出现的八种构建块（Explode、Midi Player、Run VSL、Add/Remove/Select Mesh、Get Nearest Object、Object Rename `6f074ca7:45427431`）不在播放器的任何库中：原版里它们什么也不做。它们被声明为**具名空操作**（所有者 2026-11-11 的决定）：已知、清除输入、无输出、记一行日志。第二阶段写的 Object Rename 代码保留在 cargo 特性 `absent-as-authored`（关闭）之后，留给以后一个可选的“按作者本意”模组。
