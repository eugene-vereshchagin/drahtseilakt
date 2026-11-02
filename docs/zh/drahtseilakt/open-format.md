# 开放关卡格式（版本 1）

引擎自己的关卡格式：开放、有文档、可用普通工具编辑。游戏能加载的每个关卡——玩家副本中的原版关卡、自制地图——都在玩家的机器上转换成这种格式（`drahtseilakt --level N --export DIR`、`--map FILE --export DIR`），游戏再加以游玩（`drahtseilakt --open DIR`）。转换后的关卡由玩家自己的数据生成，不对外分发。

## 文件

一个文件夹，包含：

- `level.gltf` —— [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)（JSON）；
- `level.bin` —— 其二进制缓冲区（唯一的缓冲区，按此名称引用）；
- `textures/NNNN_name.png` —— 每个纹理一张图片；图片的 `uri` 必须直接指向 `textures/` 中的普通文件（不能有更深的文件夹），否则关卡会被拒绝；
- `game/` —— 关卡使用的游戏文件，按其在玩家拷贝中的路径存放，使关卡无需拷贝中的关卡文件也能游玩：它的部件采用同样的开放格式，每个部件是一个以文件名（去掉 `.nmo`）命名的文件夹（`game/3D Entities/Balls/`、`game/3D Entities/AnimTrafo/`、`game/3D Entities/PH/P_Box/`……：球、变球器以及每个以其命名的对象所对应的模块），另外原样存放其天空的各面（`game/Textures/Sky/`）、粒子纹理（`game/Textures/Particles/`）、球的阴影以及一次游玩会播放的声音（`game/Sounds/`，不含菜单声音和其他关卡的音乐）。游戏先在那里查找，再到拷贝中查找；`game/` 中路径上任何一环是链接时都找不到文件。只有界面（菜单、字体、按钮、菜单的声音和场景）仍来自拷贝。`--export` 和 toolbox 会写入它。

## 内容

- **节点（Nodes）**：关卡中每个对象一个，按顺序排列，带有名称、世界 `matrix`，若需绘制还带有 `mesh`。`extras.drahtseilakt.visible` 表示该对象是否显示。
- **网格（Meshes）**：每个有几何体的网格一个；每种材质一个图元，同一网格的所有图元共用相同的 `POSITION`、`NORMAL` 和 `TEXCOORD_0` 访问器（浮点），`indices` 为 32 位整数。网格的材质通道（用另一材质对整个网格再绘制一遍）位于 `extras.drahtseilakt.channels`：每个通道包含其 `material`、`blend`（源与目标因子名，见下文）和 `texcoord` k，即存放其坐标的 `TEXCOORD_k` 集；最多 16 个。网格可以带一个更精细的外观，绘制时代替它（`extras.drahtseilakt.look`：另一个网格的索引；toolbox 用它保存更圆的形状）：这些网格排在关卡自身所有网格之后，每个只作为一个网格的外观，节点从不放置它们；物理和规则使用网格本身。
- **材质（Materials）**：`baseColorFactor`（漫反射）、`baseColorTexture`、`normalTexture`（切线空间的法线贴图，与 glTF 一样绿色朝上：toolbox 为重制关卡生成的凹凸细节；必须指向关卡的某个纹理）、`emissiveFactor`、`doubleSided`；`alphaMode` 为 `OPAQUE`、`MASK`（带 `alphaCutoff`）或 `BLEND`；无光照材质使用 `KHR_materials_unlit`。
  `extras.drahtseilakt` 存放 glTF 没有对应字段的内容：`ambient`、`specular` 与 `power`（高光）、`z_write`、`blend`（源与目标因子名：`zero`、`one`、`src_color`、`inv_src_color`、`src_alpha`、`inv_src_alpha`、`dest_alpha`、`inv_dest_alpha`、`dest_color`、`inv_dest_color`）、混合与 alpha 测试同时使用时的 `alpha_cutoff`，以及纹理坐标由视角生成时的 `texgen`（`reflect` 或 `chrome`）。
- **场景附加数据** `extras.drahtseilakt`：`format`（1）、`groups`（名称和成员节点索引——游戏规则读取这些，见 [nmo-format.md](nmo-format.md)）、`sky`（天空字母）、`start_ball` 和 `settings`：原版从其关卡表和按关卡编号的规则中获取的内容，使关卡在没有游戏文件时也能同样游玩——`music`（主题 1 到 1000）、`light`（三个 0..1 的数）、`level_bonus`（分数）、`tutorial` 和 `last_level`（true 或 false：原版的第一关和最后一关），以及 `source_hash`（16 位十六进制：转换来源 `.nmo` 的 FNV-1a）。在该文件上录制的回放可以在转换后的关卡上播放（同一关卡，因此同一过程）；格式错误的设置会使关卡被拒绝。

## 坐标

glTF 是右手系，Virtools 是左手系，两者都以 Y 轴向上：Z 取反（位置、法线和矩阵，矩阵的 Z 行和 Z 列变号），并且每个三角形的绕序反转。取反是精确的，因此一个关卡转换后再读回，逐位完全相同：第 1 关及其开放格式副本在相同输入下给出完全相同的轨迹和完全相同的画面。

## 读取

共享的关卡来自陌生人，因此读取器把文件夹视为恶意输入：`level.gltf` 最大 64 MiB，`level.bin` 最大 1 GiB，每个访问器都对照其视图和缓冲区进行检查，索引对照顶点数检查，图片路径限制在该文件夹内，超出上述子集的任何内容都以错误拒绝。
