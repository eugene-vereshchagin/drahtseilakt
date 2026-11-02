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
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | 我们的关卡自带设置（open-format.md） |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | 我们自己的，开放格式：`3D Entities/Balls/` |
| transformer animation | `3D Entities/AnimTrafo.nmo` | 我们自己的：`3D Entities/AnimTrafo/` |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | 我们自己的：`3D Entities/PH/<module>/`，原版的每个模块 |
| game font texture | `Textures/Font_1.tga` | Tektur，由游戏光栅化 |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | 我们自己的，同名 |
| ball shadow | `Textures/HardShadow.bmp` | 我们自己的：`Textures/HardShadow.png` |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | 我们自己的，字母 A、B、C：`Textures/Sky/Sky_<letter>_<face>.png` |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | 我们自己的，同名的 `.png` |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | 音效主题（assets/sounds） |
| menu sounds | `Sounds/Menu_click, Menu_load` | 音效主题（assets/sounds） |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | 音效主题（assets/sounds） |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | 自有：*system* 有十八首乐曲和八段氛围，Music/music.txt 为每个部分给出一个池（主题 1–3 用前九首，主题 4 和 5 用另外九首）；*neuro* 有十五段合成乐句，每个部分一段，以及自己的氛围和终点音乐 |
| original levels | `3D Entities/Level/Level_NN.NMO` | 我们自己的关卡，`assets/levels/`（levels.md） |

## 自有界面

第 2 阶段（2026-11-03）：菜单、关卡内界面（分数框、生命球、“球掉落了”）、关卡结束画面和加载画面都是我们自己的。没有原版界面文件的玩家也能看到全部界面；使用自有界面时，拷贝的界面文件根本不会被读取。

- **纹理**（`assets/own/Textures/`）：`Button01_select.tga`、`Button01_deselect.tga` 和 `Button01_special.tga`，每个部件都放在游戏代码取用它的位置（`menu.rs`、`hud.rs`、`endscreen.rs` 中的 UV 矩形），与原版同一类外观，但用我们自己的方式绘制（第 2 版，2026-11-03）：半透明玻璃，未选中为中性灰、选中为暗红；钢管按受光圆柱体着色并带拉丝纹理；由钢管组成、填充着色（未选中为淡紫灰，选中为金色）的凹槽环绕着一颗映出天空的镀铬圆球；高分榜名次带一颗珍珠，生命球及其框架，带导轨的分数框，结束画面发光的高亮条，深色条带。
- **菜单场景**（`assets/own/3D Entities/MenuLevel/`，开放关卡格式）：菜单和加载画面背后的场景；游戏像绘制原版 MenuLevel.nmo 一样绘制它，镜头以相同的镜头参数沿同一圆周环绕。它是一座悬在黄昏天空中的轨道部件之岛：带深色圆碗的中央方块、带凸起边沿的轨道分支、有四个杯座和一颗石球的起点、三个变形器、高杆上的两座灯塔、钢轨和螺旋、木制坡道、向下伸入云层的立柱以及远处的平台。镜头的曲线和目标是节点 `I_MenuLevel_Curve_0..3` 和 `Cam_MenuLevel_Target`，火焰在 `FourFlames_Flame_A..D` 处燃烧，它自己的天空是围绕镜头绘制的 `Sky_*` 对象。
- **制作方式**：由 `assets/own/make/make_interface.py`（纹理）和 `make_menu_scene.py`（场景及其纹理和天空）生成，Python，需要 numpy 和 Pillow，仅凭脚本本身和随附的 Tektur 字体；生成结果已提交到仓库，构建不需要 Python。与生成音效的做法相同：测量原版（调色板、线宽、比例和布局；场景中平台的高度和范围、按仰角的天空颜色），按这些数字绘制我们的版本，再由 `metric.py` 评估每个元素和每个画面的接近程度（Lab 颜色直方图重叠度、模糊亮度和边缘图的 SSIM）；分数记录在 `scores.md` 中。没有复制、描摹或重新采样原版的任何内容，原版的图片从不进入仓库。
- **字体**：Tektur（SIL OFL，`assets/fonts/Tektur-Variable.ttf`，版本和校验和见 `assets/fonts/README.md`），一种方正的科技感字体，气质接近原版宽体菜单字体；小写字母以小型大写字母绘制，按测得的原版比例加宽并加大字距（标签宽度误差在 2% 以内）。它缺少的字符取自 Noto Sans，中文取自 Noto Sans SC。字形在运行时光栅化到图集中，并按与游戏字体相同的矩形、对齐、换行和阴影排版，因此各画面保持原版的布局（`crates/game/src/fallback.rs`）。
- **文字**（`assets/own/Texts/menu.txt`，内置于游戏）：菜单、关卡内和结束画面的全部文字，有英语、俄语、德语、简体中文、法语、西班牙语和意大利语，原版 `Language.nmo` 有对应键时沿用其键名。使用原版界面时，优先采用拷贝在其五种语言中的文字；俄语和中文（拷贝没有）使用我们的文字，以随附字体绘制。

自有界面的默认高分榜也是我们自己的（十个名字，3000 到 750 分）。有拷贝时仍来自拷贝的有：默认按键（base.cmo 的 DB_Options；没有时为方向键、Shift 和空格）。

## 我们自己的部件

第 3 阶段（2026-11-03，decisions.md，“游戏开箱即玩”）：关卡可以引用的每个部件都有我们自己的版本，以开放格式放在 `assets/own/3D Entities/`（`PH/` 中的模块、`Balls/`、`AnimTrafo/`）和 `assets/own/Textures/`（天空、粒子、球的阴影；以 `.bmp` 或 `.tga` 请求的图片，我们的版本可以是 `.png`）中，由 `assets/own/make/make_parts.py` 仅凭脚本本身生成。它们按原版测得的尺寸制作：游戏代码引用的每个对象（部件、关节框架、火焰帧、球的网格——物理使用其包围盒）都有测得的位姿，每个碰撞网格都以我们自己的形状（长方体、棱台、棱柱、椭球体）具有测得的尺寸，物理数值（质量、摩擦、关节、力）沿用游戏自己的，不作改动（crates/game/src/modules.rs）；只有外观是新的。`crates/game/src/partcheck.rs` 对第 1 到 12 关的每个模块，分别用拷贝的部件和我们的部件，从相同的起点、以相同的输入运行，并比较两者的轨迹。

当关卡是新内容时（不是从 `.nmo` 转换而来的开放格式关卡：其设置中没有 `source_hash`），它优先使用我们的部件，因此有没有拷贝都玩起来一样；拷贝中的关卡、自制地图、转换或重制的关卡则优先使用拷贝的部件，与其制作时一致，拷贝缺少的部分才用我们的（没有拷贝时全用我们的）。关卡自己的 `game/` 文件夹优先于两者。

自有内容分阶段推出：界面（字体、按钮、分数框、生命球、文字、菜单场景），然后是球、天空、粒子和音效（来自开放素材库），带中国元素、类似麻将游戏风格的音乐，最后是自己的模块和关卡。

音效和音乐以**音效主题**提供，每个主题是一个文件夹（`Sounds/` 和 `Music/` 中按游戏请求的名称存放，Ogg Vorbis 或 WAV，`Music/music.txt` 和 `Sounds/sounds.txt` 表（为一个角色给出多个文件，即游戏从中随机选择的池），以及包含四种语言名称、作者、许可证和 `fallback`（`none` 表示不使用其他主题）的 `theme.txt`）。游戏在 `assets/sounds/` 中随附两个主题：*system*（自由许可的录音和项目所有者选择的音乐，由 `assets/sounds/system/prepare.py` 制作，CC0 或 CC-BY，作者和改动见其 `CREDITS.md`，也显示在 Credits 画面上）和 *neuro*（为游戏合成的音效和关卡音乐，GPL-3.0-only；关卡音乐是古筝风格的简短弹拨乐句，五个主题的三个部分各有一段；它包含游戏请求的所有声音，不从其他主题获取任何内容，`fallback none`）。玩家可以在数据文件夹的 `sounds/<名称>/` 中添加自己的主题；其中 *original* 由 toolbox 从玩家的拷贝制作，从不随游戏分发（在它存在之前，选择它会播放拷贝中的音效）。Extended 页面上的 *Sounds* 一行用于选择主题（`sound-theme 名称`）；在玩家选择其他主题之前（经典模式下始终）播放 *neuro*（decisions.md，“生成的音效主题为默认”）；玩家做出的选择会被保留。主题及其后备主题都没有的音效取自拷贝（关卡自己的音效）；音乐从不如此。玩家的主题文件夹不受信任：只接受简单名称，不跟随链接，主题数量有上限，音效通过音频层的限制解码。接下来可能播放的音乐会在单独的线程中预先解码。如何制作主题、所有角色和表格格式：[音效主题](sound-themes.md)。
