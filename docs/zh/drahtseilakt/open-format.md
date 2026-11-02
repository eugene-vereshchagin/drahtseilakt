# 开放关卡格式（版本 1）

引擎自己的关卡格式：开放、有文档、可用普通工具编辑。游戏能加载的每个关卡——玩家副本中的原版关卡、自制地图——都在玩家的机器上转换成这种格式（`drahtseilakt --level N --export DIR`、`--map FILE --export DIR`），游戏再加以游玩（`drahtseilakt --open DIR`）。转换后的关卡由玩家自己的数据生成，不对外分发。

## 文件

一个文件夹，包含：

- `level.gltf` —— [glTF 2.0](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html)（JSON）；
- `level.bin` —— 其二进制缓冲区（唯一的缓冲区，按此名称引用）；
- `textures/NNNN_name.png` —— 每个纹理一张图片。

## 内容

- **节点（Nodes）**：关卡中每个对象一个，按顺序排列，带有名称、世界 `matrix`，若需绘制还带有 `mesh`。`extras.drahtseilakt.visible` 表示该对象是否显示。
- **网格（Meshes）**：每个有几何体的网格一个；每种材质一个图元，同一网格的所有图元共用相同的 `POSITION`、`NORMAL` 和 `TEXCOORD_0` 访问器（浮点），`indices` 为 32 位整数。网格的材质通道（用另一材质对整个网格再绘制一遍）位于 `extras.drahtseilakt.channels`：每个通道包含其 `material`、`blend`（源与目标因子名，见下文）和 `texcoord` k，即存放其坐标的 `TEXCOORD_k` 集；最多 16 个。
- **材质（Materials）**：`baseColorFactor`（漫反射）、`baseColorTexture`、`emissiveFactor`、`doubleSided`；`alphaMode` 为 `OPAQUE`、`MASK`（带 `alphaCutoff`）或 `BLEND`；无光照材质使用 `KHR_materials_unlit`。
  `extras.drahtseilakt` 存放 glTF 没有对应字段的内容：`ambient`、`z_write`、`blend`（源与目标因子名：`zero`、`one`、`src_color`、`inv_src_color`、`src_alpha`、`inv_src_alpha`、`dest_alpha`、`inv_dest_alpha`、`dest_color`、`inv_dest_color`）、混合与 alpha 测试同时使用时的 `alpha_cutoff`，以及纹理坐标由视角生成时的 `texgen`（`reflect` 或 `chrome`）。
- **场景附加数据** `extras.drahtseilakt`：`format`（1）、`groups`（名称和成员节点索引——游戏规则读取这些，见 [nmo-format.md](nmo-format.md)）、`sky`（天空字母）和 `start_ball`。

## 坐标

glTF 是右手系，Virtools 是左手系，两者都以 Y 轴向上：Z 取反（位置、法线和矩阵，矩阵的 Z 行和 Z 列变号），并且每个三角形的绕序反转。取反是精确的，因此一个关卡转换后再读回，逐位完全相同：第 1 关及其开放格式副本在相同输入下给出完全相同的轨迹和完全相同的画面。

## 读取

共享的关卡来自陌生人，因此读取器把文件夹视为恶意输入：`level.gltf` 最大 64 MiB，`level.bin` 最大 1 GiB，每个访问器都对照其视图和缓冲区进行检查，索引对照顶点数检查，图片路径限制在该文件夹内，超出上述子集的任何内容都以错误拒绝。
