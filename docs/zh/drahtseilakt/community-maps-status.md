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
