# 纹理包与模型包

关卡的外观来自两种**包**，分别选择，就像声音来自[音效主题](sound-themes.md)：

- **纹理包**提供绘制关卡、关卡部件和球所用的图片；
- **模型包**提供部件和球的外观：为箱子、风扇、木球绘制的网格。

模型包只改变**绘制**的内容。物理始终使用游戏测得的形状（拷贝中的关卡用原版的，我们自己的关卡用我们的：
[resources.md](resources.md)，“我们自己的部件”），所以包永远不会改变关卡的玩法，每个回放（replay）使用任何包都玩得一样。
本页面向包的作者和玩家。

## 选择包

Options → Graphics 页（两种模式）有 **纹理** 和 **模型** 两行，Options → Extended 页重复这两行；箭头循环切换找到的每个包，每个包以菜单语言下的名称
显示。选择从下一个加载的关卡起生效，并保存在设置中（`settings.txt` 中的 `textures KEY`、`models KEY`）。经典模式在其 Graphics 页有同样的行；默认是我们的纹理和每个关卡自带的部件。仅运行一次：`--textures KEY` 和 `--models KEY`。

| 类型 | 内置包 | 默认 |
|---|---|---|
| 纹理 | `own`（“自带”：我们的，自由许可）、`original`（“原版”：玩家的拷贝）、`remastered`（“重制”：重制版关卡自带的放大图片，否则用拷贝中的） | `own` |
| 模型 | `level`（“关卡自带”：每个关卡使用它自己的部件）、`own`（“自带”：我们的部件的外观）、`original`（“原版”：拷贝中部件的外观） | `level` |

随纹理包一起更改（2026-11-03）：在重制关卡中，`original` 现在绘制拷贝的图片（带拷贝的透明色）；重制关卡以前在 `original` 下的外观——它自己的放大图片——现在是 `remastered` 选项。

## 包放在哪里

在数据文件夹中，与 `sounds/` 并列：`textures/<包>/` 和 `models/<包>/`（Linux 上是 `~/.local/share/drahtseilakt/`，
Windows 上是 `%APPDATA%\drahtseilakt\`，macOS 上是 `~/Library/Application Support/drahtseilakt/`）。安装包就是把它的
文件夹复制到那里；游戏在启动时查找包，所以添加后请重新启动。

文件夹名就是包的键（设置中保存的值）：字母、数字、`_`、`-` 和 `.`，不能以点开头，最多 64 个字符。与内置包同名
（`own`、`original`、`remastered`、`level`）或名为 `none` 的文件夹会被忽略。每种类型最多读取 32 个包（内置包计入）。

## pack.txt

与音效主题的 `theme.txt` 形式相同：UTF-8 纯文本，最多 4 KiB，每行一个 `键 值`，`#` 开始注释。

| 键 | 含义 |
|---|---|
| `name.en`、`name.ru`、`name.de`、`name.zh` | 菜单中各语言的包名称（最多 64 个字符）；缺少时显示英文名称，否则显示文件夹名。 |
| `author`、`licence` | 作者和许可证（供人阅读）。 |
| `fallback` | 本包缺少的内容从哪个包获取：一个包的键，或 `none`。省略时：纹理为 `own`，模型为 `level`。 |

查找纹理或部件的顺序：所选的包、它的后备包、该后备包的后备包（最多八个，每个只用一次），然后是默认值（纹理：我们的，
再是关卡自己的图片；模型：关卡自己的部件）。所以包只替换它所拥有的内容。

## 纹理包

```
textures/
  stone/
    pack.txt
    CREDITS.md             who made each picture, under which licence (recommended)
    Floor_Top_Border.png   a picture for the original's texture Floor_Top_Border
    Wood_Plain.jpg
    Sky/Sky_A_Front.png    the sky's faces (also accepted at the top)
    Particles/Particle_Flames.png
```

- 图片以**纹理的名称**命名：原版的纹理名称（`assets/own/TEXTURES.md` 列出了关卡绘制所用的每一个，它是什么以及尺寸）、
  赛道的（`Track_Side`、`Track_floor_border` ……）以及关卡提到的任何其他纹理。比较名称时忽略大小写、`_`、`-` 和空格，
  也不含扩展名：`ball_wood.png` 就是 `Ball_Wood.bmp`。
- PNG、JPEG、BMP 或 TGA；每个文件最大 32 MB，每边最多 8192 像素；每个包最多 4096 张图片。Alpha 有效（原版的
  “透明色”不会应用到包中的图片）。
- 只读取包文件夹本身、`Sky/` 和 `Particles/` 中的普通文件；从不跟随链接。无法读取或解码的图片会被跳过，改用下一个包。
  关卡文件夹中的关卡图片与各包中的图片共用 2.56 亿像素的预算；一张图片替代多个纹理时，每个都计入。

包会替换哪些纹理（与我们自己的纹理规则相同，[resources.md](resources.md)，“自有纹理”）：拷贝中的关卡、部件和球及其
重制版的每个纹理；在陌生人的地图中，只替换它所用的拷贝自己的纹理（绝不替换地图自带的图片）；在开放格式的关卡中，
只替换它指明为游戏自带的纹理（`game:` 图片，[open-format.md](open-format.md)），绝不替换它自己文件夹里的文件。关卡自己的
`game/Textures/` 中同名的图片优先于任何包。天空、粒子和球的阴影，如果玩家的包中有，就先取自玩家的包，然后才是内置包。

### 示例：一个纹理包

1. 在数据文件夹中创建文件夹 `textures/myfloors/`。
2. 编写 `pack.txt`：
   ```
   name.en My floors
   name.ru Мои полы
   name.zh 我的地板
   author Jane Doe
   licence CC0 1.0
   fallback own
   ```
3. 绘制 `Floor_Top_Border.png` 和 `Floor_Side.png`（原版的是 256 × 256 且可平铺；我们的在 `assets/own/Textures/` 中，是个
   不错的起点：复制一张再重新绘制）。
4. 启动游戏，Options → Extended → 纹理：选 *我的地板*，然后加载一个关卡。现在每个带边框的地板都是你的；其余都是我们的
   （后备包）。

## 模型包

```
models/
  round/
    pack.txt
    PH/P_Box/level.gltf     the box's look, a part in the open format
    PH/P_Box/level.bin
    PH/P_Box/textures/...   its own pictures (kept as they are by any texture pack)
    Balls/level.gltf        the balls, their pieces and shadows
    AnimTrafo/level.gltf    the transformer
```

- 布局与游戏自己的部件相同（`assets/own/3D Entities/`）：一个部件就是一个[开放关卡格式](open-format.md)的文件夹，
  以原版文件去掉 `.nmo` 后的名称命名（`PH/P_Modul_18/`、`Balls/`）。最简单的起步方法是从 `assets/own/3D Entities/`
  复制该部件，再修改它的网格。
- 部件的网格会被绘制在游戏所用部件中**同名对象**的网格的位置上（即游戏代码使用的名称：`P_Box_MF`、`Ball_Wood`，
  碎片 `Ball_Wood_Piece_01` ……），并移动到该对象的位置；否则取代同名的网格。游戏部件中包未匹配的网格照原样绘制。
  多个对象共用的网格，只有当它们全都在同一位置匹配同一个网格时才会换上新外观。
- 包的材质和纹理属于它自己：其 `textures/` 文件永远不会被纹理包替换；其 `game:` 图片则跟随纹理包。
- 按不受信任的内容检查（包会被分享）：只接受真正的文件夹和文件，`level.gltf` 最大 8 MB，`level.bin` 最大 64 MB，由
  开放格式的读取器读取（每个 accessor 都经检查），每个部件最多 1,048,576 个顶点和 2,097,152 个三角形，每个数值都必须是
  有限的。未通过任何检查的部件会被舍弃（在终端中说明一次），由链中的下一个包提供；游戏绝不会因包而失败。
- 只改外观：碰撞形状、质量、关节和其他所有数值都仍由游戏决定。

### 示例：一个模型包

1. 把（游戏文件夹中的）`assets/own/3D Entities/PH/P_Box/` 复制到数据文件夹的 `models/bigbox/PH/P_Box/`。
2. 编写 `models/bigbox/pack.txt`，内容为 `name.en Big box`。
3. 编辑箱子的网格（Blender 可以导入和导出 glTF；保留对象名称，导出为 glTF Separate，纹理放在 `textures/` 中，然后把
   场景的 `extras.drahtseilakt` 恢复成原文件中的样子）。
4. Options → Extended → 模型：*Big box*。箱子会以你的外观绘制；它仍然像游戏中的那样滚动、下落和碰撞。

## 许可与分享

- 使用自己的图片和网格，或采用 CC0 或 CC-BY 许可的作品；把它们列在 `CREDITS.md` 中。
- **绝不要包含原版游戏的纹理或模型**（或任何其他游戏的），即使经过编辑也不行。内置的 `original` 包在每位玩家自己的电脑上，
  从他自己的拷贝中获取它们。

## 参考

- 代码：`crates/gamedata/src/packs.rs`（查找和选择包、pack.txt）、`crates/gamedata/src/assets.rs`
  （`put_pack_textures`、`game_texture`、`put_pack_looks`、`load_pack_part`）。
- 已做模糊测试：`pack_txt`（[fuzzing.md](fuzzing.md)）；模型包的部件会经过开放格式的读取器（`open_level`）。
