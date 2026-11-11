# 自制地图：现状

对项目所有者 `levels/community/` 中 579 张自制地图的调查（2026-11-02；[CustomMapList](community-maps.md) 中的地图：Custom Maps 408、Pro Racing 97、Reverse Maps 35、45-Degree Rotated Maps 13、Invisible Maps 13、Original Maps 13）。每张地图都在无窗口模式下加载并运行 300 个 tick，导出为开放格式后再次加载，并列出其 Virtools 脚本（按名称列出构建块）。这是脚本解释器和模组的计划；**2026-11-09：**解释器第一阶段已完成（[scripts.md](scripts.md)）：核心、模板的构建块以及其余最常用的构建块；184 张带脚本的地图（连同 `community-unsorted/` 共 584 张）中有 173 张的所有构建块均已知，36 张自带脚本的地图中有 8 张的每个构建块都已完整生效。

| | 地图数 |
|---|---|
| 按游戏内置规则加载并游玩 | 579 |
| 具有标准组（起点、终点、检查点、重置点） | 579 |
| 可导出为开放格式 | 579 |
| 可从开放格式再次加载 | 578 |
| 带有自己的 Virtools 脚本 | 188 |

**无法再次加载的那张：**`Custom Maps/Beautiful.NMO` 中有一个对象的矩阵含有非有限数；游戏可以游玩 `.nmo`，但开放格式拒绝这样的矩阵。待办：导出时按游戏的使用方式写出该对象（或将其略去），并给出警告。

**脚本。**188 张地图带有自己的行为图，除 4 张 Reverse Maps 外都来自 Custom Maps。其中 142 张带有同样的九个构建块（Activate Script、Binary Switch、Change Texture Video Format、Deactivate Script、Get Cell、Object Delete、Op、Point Particle System、TT Scaleable Proximity），另有 7 张在此基础上多了 Wait Message：这是地图制作者的模板为火焰粒子带来的脚本，而游戏本身就会绘制这些粒子。这些地图缺少这些脚本是否会损失什么，需要实际游玩来验证；内置规则已能带每张地图走完起点、检查点和终点。其余 39 张地图使用自己的脚本：总共 107 个构建块，大多数只出现在一两张地图中。

**按使用地图数排列的构建块**（解释器的实现顺序）：Op 184, Get Cell 183, Object Delete 183, Activate Script 182, Binary Switch 182, Deactivate Script 182, Point Particle System 182, Change Texture Video Format 180, TT Scaleable Proximity 179, Wait Message 35, Group Iterator 22, Hide 20, Physicalize 19, Box Box Intersection 19, Set Position 18, Iterator 17, Show 16, Switch On Parameter 16, Add Row 15, Delayer 15, PlanarParticleSystem 14, Get Key Row 13, Object Copy 13, Send Message To Group 12, Test 11, Text Display 11, Set Cell 9, Wave Player 9, Nop 8, Restore IC 8；之后是 77 个构建块，每个出现在少于 8 张地图中（其中包括物理力和冲量、文本和字体、按键和鼠标、计时器和计数器、灯光、纹理和声音）。

**这意味着什么。**内置规则能游玩每一张地图；为模板的九个构建块以及接下来的二十个实现解释器，就能覆盖带脚本的地图所增加的内容（移动和隐藏的对象、物理变化、复制、消息、文本、声音）。地图绝不会只运行一半：需要尚未实现的构建块的地图会报告该块的名称。

## 审查：地图需要什么、得到什么（2026-11-10）

`drahtseilakt --audit FILE|DIR [--audit-csv FILE]` 不开窗口地检查一张地图、一个开放格式的关卡文件夹（含 `level.gltf`）或它们的文件夹，分别在有玩家副本和没有副本的情况下，为每张地图列出：物理引擎由关卡对象生成但没有被绘制的每个物体（隐形障碍，附名称、位置和原因）、没有生成机关的占位物、谁都没有的机关和贴图、加载器跳过的类的对象、渲染器不遵循的 alpha 测试，以及未知的脚本模块。CSV 中每张地图、每种副本情况、每类问题占一行并附名称；汇总按受影响的地图数量给问题种类排序。`--show-colliders` 在游戏和截图中把地图隐藏的地板和轨道（红色）以及隐藏的挡板（黄色）画成半透明形状。

对 605 张地图（随游戏提供的 26 张和收藏中的 579 张）的第一次检查找到了“隐形物体”的原因，并为所有地图修改了三条规则：

| 原因 | 地图数 | 现在 |
|---|---|---|
| 占位物靠名称识别（`P_Modul_26_01`），而且只认没有材质的可见网格；Blender 制作的地图把它命名为 `P_Modul_26_01_.001`、给它材质或把它隐藏，于是没有生成机关，占位物成了看不见的或纯白的实体 | 71 + 29 + 22 | 机关组的成员就是占位物，与原版关卡初始化的做法一致 |
| 没有材质的地板、轨道或挡板会碰撞但不绘制 | 42 | 以纯色绘制 |
| 不是游戏机关的组（模组的 `P_Modul_Portal`）使其成员成为实体 | 13 | 永不成为物体；按对象本身绘制 |

剩下的问题，按受影响的地图数（有副本时）：文件中隐藏但会碰撞的地板和轨道 204 张，隐藏的挡板 160 张（地图自带的隐形碰撞体，与原版相同；`--show-colliders` 可显示它们）；未读取的类的对象 58 张（2D 对象 34、曲线 16、声音 9、面片网格 2）；既没有文件名也没有像素的贴图 35 张；含无材质面的网格 30 张（画成灰色）；模组的机关 17 张；未知脚本模块 11 张；完全透明的地板 6 张。

### 与原版核对：隐藏的碰撞体和对象动画（2026-11-10）

**隐藏的地板和轨道保持为实体。** 原版的关卡初始化会把 `Phys_Floors` 和 `Phys_FloorRails` 的每个成员物理化，而 Physicalize 并不看对象是否显示（参考实现中该模块的移植，`src/bb/bb_physics.c`）。原版自己的关卡就依赖这一点：12 个关卡文件中有 7 个在这些组里放了 19 个隐藏成员（第 6 关的 `A02_inv_Coll_00`..`04`、第 11 关的 `A07_Floor_Invisible_Col`、第 12 关的 `A7_invisibleCollCube`）。规则没有改动。在 Go Ahead 中，隐藏的墙成对地沿着路线立在变球器两侧（`Box04` / `Box05`）：这是一条不让球绕过变球器的通道，而不是横在路上的墙；这张地图没有脚本，所以没有任何东西会移走它们。

**对象动画：没有任何东西播放它们。** 88 张地图里共有 9871 个动画对象（类 15 和 18）。605 张地图中没有一张含有播放动画的模块（Play Animation 3D Entity、Play Global Animation），也没有保存动画的参数，而 Virtools 自己不会播放任何动画。其中 9287 个是导出器的烘焙：位置、旋转、缩放和缩放轴各 35 个关键帧，每个关键帧都与第一个相同。356 个没有自己的关键帧，8 个是游戏自带气球的手臂。原版里什么都不动，所以游戏也不移动任何东西。审查把它们单独计数（`animation-unplayed`），指出关键帧会移动对象的动画，并在地图里有东西可以播放动画时报告 `animation-played`。`nmo-dump --anims FILE...` 打印每个动画及其对象、控制器和关键帧的移动范围。社区地图中真正会动的东西是由它们的脚本移动的（Set Position、Rotate、Physicalize、Set Physics Slider；大约二十张地图），这是脚本运行时的下一阶段（[scripts.md](scripts.md)）。
