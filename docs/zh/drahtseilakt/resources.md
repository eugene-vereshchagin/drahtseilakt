# 界面资源

游戏按原版的目录结构请求文件；每个文件扮演一个**角色**（菜单字体、坠落音效、木球……）。根据“默认使用自有内容”的决定（[decisions.md](decisions.md)），一个角色可由三个来源提供：

- **自有**：随游戏提供、放在 `assets/own/` 中并按原版结构排列的我们自己的文件（界面自第 2 阶段起已有：见下文“自有界面”）；
- **原版**：玩家的拷贝；
- **重制**：toolbox 对拷贝的重制，位于 `levels/remastered/.interface/`。

扩展模式的 **Interface** 开关（own / original / remaster，设置项 `interface`）决定谁优先；从下次启动起生效。默认以及经典模式使用的是拷贝自带的界面（若拷贝有其菜单），否则为**自有**（decisions.md，“在自有界面重做期间默认使用拷贝的界面”）。没有任何来源提供的角色即为缺失：游戏会提示一次，然后在没有它的情况下继续运行。关卡自己的文件夹（`<关卡>/game/`，见 [open-format.md](open-format.md)）对其部件依然最优先。

拷贝的字体无法绘制的文字（它只有 Latin-1）使用随附的字体（`assets/fonts`，SIL OFL）绘制，正如自有界面绘制它的所有文字：选择原版界面时的俄文和中文菜单，以及自制地图的中文和西里尔字母名称。

| 角色 | 原版文件 | 自有 |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | Tektur（`assets/fonts`），由游戏绘制 |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | 我们自己的，开放格式：`3D Entities/MenuLevel/` |
| texts in the copy's languages | `3D Entities/Language.nmo` | `Texts/menu.txt`，七种语言 |
| menu texts, level table defaults | `base.cmo` | 计划中 |
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | 计划中 |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | 计划中 |
| transformer animation | `3D Entities/AnimTrafo.nmo` | 计划中 |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | 计划中 |
| game font texture | `Textures/Font_1.tga` | Tektur，由游戏光栅化 |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | 我们自己的，同名 |
| ball shadow | `Textures/HardShadow.bmp` | 计划中 |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | 计划中 |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | 计划中 |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | 音效主题（assets/sounds） |
| menu sounds | `Sounds/Menu_click, Menu_load` | 音效主题（assets/sounds） |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | 音效主题（assets/sounds） |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | 自有：*system* 有十八首乐曲和八段氛围，Music/music.txt 为每个部分给出一个池（主题 1–3 用前九首，主题 4 和 5 用另外九首）；*neuro* 有十五段合成乐句，每个部分一段，以及自己的氛围和终点音乐 |
| original levels | `3D Entities/Level/Level_NN.NMO` | 计划中 |

## 自有界面

第 2 阶段（2026-11-03）：菜单、关卡内界面（分数框、生命球、“球掉落了”）、关卡结束画面和加载画面都是我们自己的。没有原版界面文件的玩家也能看到全部界面；使用自有界面时，拷贝的界面文件根本不会被读取。

- **纹理**（`assets/own/Textures/`）：`Button01_select.tga`、`Button01_deselect.tga` 和 `Button01_special.tga`，每个部件都放在游戏代码取用它的位置（`menu.rs`、`hud.rs`、`endscreen.rs` 中的 UV 矩形），与原版同一类外观，但用我们自己的方式绘制（第 2 版，2026-11-03）：半透明玻璃，未选中为中性灰、选中为暗红；钢管按受光圆柱体着色并带拉丝纹理；由钢管组成、填充着色（未选中为淡紫灰，选中为金色）的凹槽环绕着一颗映出天空的镀铬圆球；高分榜名次带一颗珍珠，生命球及其框架，带导轨的分数框，结束画面发光的高亮条，深色条带。
- **菜单场景**（`assets/own/3D Entities/MenuLevel/`，开放关卡格式）：菜单和加载画面背后的场景；游戏像绘制原版 MenuLevel.nmo 一样绘制它，镜头以相同的镜头参数沿同一圆周环绕。它是一座悬在黄昏天空中的轨道部件之岛：带深色圆碗的中央方块、带凸起边沿的轨道分支、有四个杯座和一颗石球的起点、三个变形器、高杆上的两座灯塔、钢轨和螺旋、木制坡道、向下伸入云层的立柱以及远处的平台。镜头的曲线和目标是节点 `I_MenuLevel_Curve_0..3` 和 `Cam_MenuLevel_Target`，火焰在 `FourFlames_Flame_A..D` 处燃烧，它自己的天空是围绕镜头绘制的 `Sky_*` 对象。
- **制作方式**：由 `assets/own/make/make_interface.py`（纹理）和 `make_menu_scene.py`（场景及其纹理和天空）生成，Python，需要 numpy 和 Pillow，仅凭脚本本身和随附的 Tektur 字体；生成结果已提交到仓库，构建不需要 Python。与生成音效的做法相同：测量原版（调色板、线宽、比例和布局；场景中平台的高度和范围、按仰角的天空颜色），按这些数字绘制我们的版本，再由 `metric.py` 评估每个元素和每个画面的接近程度（Lab 颜色直方图重叠度、模糊亮度和边缘图的 SSIM）；分数记录在 `scores.md` 中。没有复制、描摹或重新采样原版的任何内容，原版的图片从不进入仓库。
- **照片纹理**（菜单场景第 3 版，2026-11-03）：其中的混凝土（平台、方块、立柱）、砖砌边沿、木制坡道和木箱以及铁件（箍环、底座、灯柱）由自由许可的照片纹理（CC0，ambientCG）制成，由 `make_menu_scene.py` 按原版同一用途纹理的实测调色板和纹理颗粒调整，并带有法线贴图。`assets/own/make/free_textures.py` 以 SHA-256 固定每个源压缩包（`fetch` 下载缺少的压缩包，不解压也不运行其中任何内容）；源文件不随游戏提供，由它们制成的纹理随游戏提供。来源、作者和许可：`assets/own/CREDITS.md`；随附文件的校验和：`assets/own/SHA256SUMS`。
- **字体**：Tektur（SIL OFL，`assets/fonts/Tektur-Variable.ttf`，版本和校验和见 `assets/fonts/README.md`），一种方正的科技感字体，气质接近原版宽体菜单字体；小写字母以小型大写字母绘制，按测得的原版比例加宽并加大字距（标签宽度误差在 2% 以内）。它缺少的字符取自 Noto Sans，中文取自 Noto Sans SC。字形在运行时光栅化到图集中，并按与游戏字体相同的矩形、对齐、换行和阴影排版，因此各画面保持原版的布局（`crates/game/src/fallback.rs`）。
- **文字**（`assets/own/Texts/menu.txt`，内置于游戏）：菜单、关卡内和结束画面的全部文字，有英语、俄语、德语、简体中文、法语、西班牙语和意大利语，原版 `Language.nmo` 有对应键时沿用其键名。使用原版界面时，优先采用拷贝在其五种语言中的文字；俄语和中文（拷贝没有）使用我们的文字，以随附字体绘制。

自有界面的默认高分榜也是我们自己的（十个名字，3000 到 750 分）。无论使用哪种界面，仍来自拷贝的有：默认按键（base.cmo 的 DB_Options；没有时为方向键、Shift 和空格），以及关卡的部件（球、粒子、球的阴影、天空、机关），之后的阶段会替换它们。

## 自有纹理

第 3 阶段（2026-11-03）：关卡、关卡部件和球默认使用我们自己的纹理绘制（decisions.md，“默认使用自有纹理”）：无论哪个关卡请求——原版关卡、社区地图、模块、球——`assets/own/Textures/<名称>.png` 都会替换同名的原版纹理；名称比较时忽略大小写、`_`、`-` 和空格（`BallWood.bmp` 即 `Ball_Wood`）。十三个关卡使用的 63 个纹理中有 45 个是我们的（平台及其砖条、检查点、立柱、砖墙、所有木材、金属、屋顶、标记、反射贴图、三种球）；天空、粒子、光晕、阴影和少数图片仍取自拷贝。`assets/own/TEXTURES.md` 列出每个纹理的用途、尺寸、使用频率以及是否已有我们的替代。它们由 `assets/own/make/make_textures.py` 用 CC0 照片纹理（`free_textures.py`，固定版本并校验；`assets/own/CREDITS.md`）和脚本绘制的图形制作，按同名原版纹理实测的调色板、颗粒和布局（砖条、箍带、圆环和明暗所占边长的比例）调整，分辨率加倍，原版平铺处无缝平铺，球的包裹方式与原版相同。扩展页面的 *Textures* 开关（`textures own|original`）改用拷贝的纹理；经典模式始终使用我们的纹理。开放格式关卡自带的纹理（其文件夹中的文件）仍归其作者。`--textures original|own` 为单次运行选择；`--export` 默认保留拷贝的纹理，加上 `--textures own` 则把我们的纹理写入导出的关卡。我们的文件随游戏提供，读取时与任何纹理一样受尺寸上限约束；关卡中的名称只在其中挑选，绝不会构成路径。

自有内容分阶段推出：界面（字体、按钮、分数框、生命球、文字、菜单场景），然后是球、天空、粒子和音效（来自开放素材库），带中国元素、类似麻将游戏风格的音乐，最后是自己的模块和关卡。

音效和音乐以**音效主题**提供，每个主题是一个文件夹（`Sounds/` 和 `Music/` 中按游戏请求的名称存放，Ogg Vorbis 或 WAV，`Music/music.txt` 和 `Sounds/sounds.txt` 表（为一个角色给出多个文件，即游戏从中随机选择的池），以及包含四种语言名称、作者、许可证和 `fallback`（`none` 表示不使用其他主题）的 `theme.txt`）。游戏在 `assets/sounds/` 中随附两个主题：*system*（自由许可的录音和项目所有者选择的音乐，由 `assets/sounds/system/prepare.py` 制作，CC0 或 CC-BY，作者和改动见其 `CREDITS.md`，也显示在 Credits 画面上）和 *neuro*（为游戏合成的音效和关卡音乐，GPL-3.0-only；关卡音乐是古筝风格的简短弹拨乐句，五个主题的三个部分各有一段；它包含游戏请求的所有声音，不从其他主题获取任何内容，`fallback none`）。玩家可以在数据文件夹的 `sounds/<名称>/` 中添加自己的主题；其中 *original* 由 toolbox 从玩家的拷贝制作，从不随游戏分发（在它存在之前，选择它会播放拷贝中的音效）。Extended 页面上的 *Sounds* 一行用于选择主题（`sound-theme 名称`）；在玩家选择其他主题之前（经典模式下始终）播放 *neuro*（decisions.md，“生成的音效主题为默认”）；玩家做出的选择会被保留。主题及其后备主题都没有的音效取自拷贝（关卡自己的音效）；音乐从不如此。玩家的主题文件夹不受信任：只接受简单名称，不跟随链接，主题数量有上限，音效通过音频层的限制解码。接下来可能播放的音乐会在单独的线程中预先解码。如何制作主题、所有角色和表格格式：[音效主题](sound-themes.md)。
