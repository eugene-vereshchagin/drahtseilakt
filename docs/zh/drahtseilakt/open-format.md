# 开放关卡格式（版本 1）

引擎自己的关卡格式：开放、有文档、可用普通工具编辑。游戏能加载的每个关卡——玩家副本中的原版关卡、自制地图——都在玩家的机器上转换成这种格式（`drahtseilakt --level N --export DIR`、`--map FILE --export DIR`），游戏再加以游玩（`drahtseilakt --open DIR`）。转换后的关卡由玩家自己的数据生成，不对外分发。使用 `--textures own` 导出时，凡是我们有对应纹理的地方都写入我们的自由纹理（resources.md，“自有纹理”）而不是拷贝的纹理，这样关卡的外观只用自由资源即可分享。

## 文件

一个文件夹，包含：

- `level.gltf` —— [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)（JSON）；
- `level.bin` —— 其二进制缓冲区（唯一的缓冲区，按此名称引用）；
- `textures/NNNN_name.png` —— 每个纹理一张图片；图片的 `uri` 必须直接指向 `textures/` 中的普通文件（不能有更深的文件夹），或者是 `game:<名称>`，否则关卡会被拒绝。`game:<名称>` 引用的不是文件，而是**游戏中该名称的纹理**：不为它读取任何文件；游戏用所选纹理包绘制它（[packs.md](packs.md)）：先找关卡自己 `game/Textures/` 中该名称的图片，再找纹理包及其后备，再找我们的，最后找拷贝的（名称按纹理名称比较：忽略大小写、`_`、`-`、空格和扩展名）。名称长 1 到 128 个字符，不含 `/`、`\`、`:` 或控制字符，且不以点开头；它只与纹理名称比较，绝不会构成路径。我们的关卡用这种方式引用共用的纹理（轨道侧面、边框砖、木材、栏杆、立柱、检查点地板），所以纹理包也会改变它们；它们自己的特殊纹理仍以文件保存。`--export` 原样保留 `game:` 图片。以前制作的关卡照常可用：它们的文件仍是它们自己的；
- `level.txt`（可选）——以 `键 = 值` 行写出标题、作者和版本；`levels/` 中的关卡是只含一个关卡的文件夹，以此标题命名（[levels.md](levels.md)，“一个关卡就是一个文件夹”）；
- `game/` —— 关卡使用的游戏文件，按其在玩家拷贝中的路径存放，使关卡无需拷贝中的关卡文件也能游玩：它的部件采用同样的开放格式，每个部件是一个以文件名（去掉 `.nmo`）命名的文件夹（`game/3D Entities/Balls/`、`game/3D Entities/AnimTrafo/`、`game/3D Entities/PH/P_Box/`……：球、变球器以及每个以其命名的对象所对应的模块），另外原样存放其天空的各面（`game/Textures/Sky/`）、粒子纹理（`game/Textures/Particles/`）、球的阴影以及一次游玩会播放的声音（`game/Sounds/`，不含菜单声音和其他关卡的音乐）。游戏先在那里查找；对于为我们自己的部件而做的关卡（其设置写有 `"parts": "own"`），接着到我们自己的部件（`assets/own/`，[resources.md](resources.md)）中、再到拷贝中查找，因此有没有拷贝都玩起来一样；对于其他任何关卡，接着到拷贝中、再到我们的部件中查找。`game/` 中路径上任何一环是链接时都找不到文件。`game/` 只能替代关卡自己的部件、天空、粒子、阴影和一次游玩的声音，界面文件（菜单、字体、关卡表、菜单声音）从不来自关卡：它们属于界面（我们自己的，或拷贝的），每个文件的读取都有大小上限。任何关卡都可以声明任意 `source_hash`，因此能在其上播放的回放并不能证明在原版上完成过游玩。没有设置的关卡（在开始保存设置之前制作的）使用默认值并给出提示；重制的 `Level_NN` 则从拷贝的表中获取设置。`--export` 和 toolbox 会写入它。

## 内容

- **节点（Nodes）**：关卡中每个对象一个，按顺序排列，带有名称、世界 `matrix`，若需绘制还带有 `mesh`。`extras.drahtseilakt.visible` 表示该对象是否显示。
- **网格（Meshes）**：每个有几何体的网格一个；每种材质一个图元，同一网格的所有图元共用相同的 `POSITION`、`NORMAL` 和 `TEXCOORD_0` 访问器（浮点），`indices` 为 32 位整数。网格的材质通道（用另一材质对整个网格再绘制一遍）位于 `extras.drahtseilakt.channels`：每个通道包含其 `material`、`blend`（源与目标因子名，见下文）和 `texcoord` k，即存放其坐标的 `TEXCOORD_k` 集；最多 16 个。网格可以带一个更精细的外观，绘制时代替它（`extras.drahtseilakt.look`：另一个网格的索引；toolbox 用它保存更圆的形状）：这些网格排在关卡自身所有网格之后，每个只作为一个网格的外观，节点从不放置它们；物理和规则使用网格本身。
- **材质（Materials）**：`baseColorFactor`（漫反射）、`baseColorTexture`、`normalTexture`（切线空间的法线贴图，与 glTF 一样绿色朝上：toolbox 为重制关卡生成的凹凸细节；必须指向关卡的某个纹理；带有 `extras.drahtseilakt.parallax`（以纹理坐标计的深度，大于 0 且不超过 0.25）时，其 alpha 通道是高度，1 为顶部，渲染器以视差绘制，使凹凸在掠射视角下仍保持深度）、`emissiveFactor`、`doubleSided`；`alphaMode` 为 `OPAQUE`、`MASK`（带 `alphaCutoff`）或 `BLEND`；无光照材质使用 `KHR_materials_unlit`。
  `extras.drahtseilakt` 存放 glTF 没有对应字段的内容：`ambient`、`specular` 与 `power`（高光）、`z_write`、`blend`（源与目标因子名：`zero`、`one`、`src_color`、`inv_src_color`、`src_alpha`、`inv_src_alpha`、`dest_alpha`、`inv_dest_alpha`、`dest_color`、`inv_dest_color`）、混合与 alpha 测试同时使用时的 `alpha_cutoff`，以及纹理坐标由视角生成时的 `texgen`（`reflect` 或 `chrome`）。
- **场景附加数据** `extras.drahtseilakt`：`format`（1）、`groups`（名称和成员节点索引——游戏规则读取这些，见 [nmo-format.md](nmo-format.md)）、`sky`（天空字母）、`start_ball` 和 `settings`：原版从其关卡表和按关卡编号的规则中获取的内容，使关卡在没有游戏文件时也能同样游玩——`music`（主题 1 到 1000）、`light`（三个 0..1 的数）、`level_bonus`（分数）、`tutorial` 和 `last_level`（true 或 false：原版的第一关和最后一关），以及 `source_hash`（16 位十六进制：转换来源 `.nmo` 的 FNV-1a）以及 `parts`（`"own"`：关卡为游戏自己的部件而做，优先于拷贝的部件使用它们；`"copy"` 或不写：优先使用拷贝的部件；我们的关卡工具写入 `"own"`）。在该文件上录制的回放可以在转换后的关卡上播放（同一关卡，因此同一过程）；格式错误的设置会使关卡被拒绝。
- **雾** `settings.fog`（我们的关卡），只用于绘制：离相机远的东西融入空气的颜色——朝地平线方向为 `color`，正下方为 `color_down`（三个 0..1 的数；没有 `color_down` 时四周同一种颜色），每单位距离的雾量 `density`（1 - e^(-density × 距离)，0..1），低于 `below` 的东西也融入其中，在其下方 `depth` 处完全融入。只作用于受光照的表面（天空、精灵和云海保持自己的颜色）；原版关卡没有雾。`color`、`density`、`below` 和 `depth` 是必需的；超出范围的值会被限制到范围内，格式错误的值会使关卡被拒绝。
- **云海** `settings.clouds`（我们的关卡；见 levels.md“云海”），只用于绘制：`top`（云顶高度；必需）、`thickness`、`cover`（0..1）、`scale`（单朵云的大小）、`sun`（指向太阳的方向）、`sun_color`、`sky`（来自上方的光）、`shadow`（阴影中；颜色为线性光，1 为白）、`haze`（每单位距离的地平线雾霭）、`under`（0..1：层下的雾霭遮住下方世界的程度）和 `wind`（每秒 x、z）。只有 `top` 是必需的；超出范围的值会被限制到范围内，格式错误的值会使关卡被拒绝。

## 坐标

glTF 是右手系，Virtools 是左手系，两者都以 Y 轴向上：Z 取反（位置、法线和矩阵，矩阵的 Z 行和 Z 列变号），并且每个三角形的绕序反转。取反是精确的，因此一个关卡转换后再读回，逐位完全相同：第 1 关及其开放格式副本在相同输入下给出完全相同的轨迹和完全相同的画面。

## 读取

共享的关卡来自陌生人，因此读取器把文件夹视为恶意输入：`level.gltf` 最大 64 MiB，`level.bin` 最大 1 GiB，每个访问器都对照其视图和缓冲区进行检查，索引对照顶点数检查，图片路径限制在该文件夹内，超出上述子集的任何内容都以错误拒绝。
