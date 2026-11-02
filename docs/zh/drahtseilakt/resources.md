# 界面资源

游戏按原版的目录结构请求文件；每个文件扮演一个**角色**（菜单字体、坠落音效、木球……）。根据“默认使用自有内容”的决定（[decisions.md](decisions.md)），一个角色可由三个来源提供：

- **自有**：随游戏提供、放在 `assets/own/` 中并按原版结构排列的我们自己的文件（界面自第 2 阶段起已有：见下文“自有界面”）；
- **原版**：玩家的拷贝；
- **重制**：toolbox 对拷贝的重制，位于 `levels/remastered/.interface/`。

扩展模式的 **Interface** 开关（own / original / remaster，设置项 `interface`）决定谁优先；从下次启动起生效。**自有**为默认，经典模式始终使用它（decisions.md，“自有界面为默认”）。没有任何来源提供的角色即为缺失：游戏会提示一次，然后在没有它的情况下继续运行。关卡自己的文件夹（`<关卡>/game/`，见 [open-format.md](open-format.md)）对其部件依然最优先。

拷贝的字体无法绘制的文字（它只有 Latin-1）使用随附的字体（`assets/fonts`，SIL OFL）绘制，正如自有界面绘制它的所有文字：选择原版界面时的俄文和中文菜单，以及自制地图的中文和西里尔字母名称。

| 角色 | 原版文件 | 自有 |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | Tektur（`assets/fonts`），由游戏绘制 |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | 一张图片：`Textures/Menu_Background.png` |
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
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | 自有：*system* 有九首乐曲，Music/music.txt 把它们分配到五个主题；*neuro* 有十五段合成乐句，每个部分一段，以及自己的终点音乐；没有氛围部分 |
| original levels | `3D Entities/Level/Level_NN.NMO` | 计划中 |

## 自有界面

第 2 阶段（2026-11-03）：菜单、关卡内界面（分数框、生命球、“球掉落了”）、关卡结束画面和加载画面都是我们自己的。没有原版界面文件的玩家也能看到全部界面；使用自有界面时，拷贝的界面文件根本不会被读取。

- **纹理**（`assets/own/Textures/`）：`Button01_select.tga`、`Button01_deselect.tga` 和 `Button01_special.tga`，每个部件都放在游戏代码取用它的位置（`menu.rs`、`hud.rs`、`endscreen.rs` 中的 UV 矩形）：钢管包边、带抛光圆球的紫色玻璃面板，选中时为玫瑰色配金色圆球，未解锁的关卡为灰色；高分榜的名次数字、生命球及其支架、分数框、结束画面的高亮条、深色条带。另有 `Menu_Background.png`，即菜单和加载画面背后的图片（云海上的黄昏，左边是放着球的木制轨道，右边是顶着球的石柱；中间被菜单的条带遮住）。它代替原版的 3D 菜单场景，后者只在使用原版界面时绘制。
- **制作方式**：由 `assets/own/make/make_interface.py`（Python，需要 numpy 和 Pillow）仅凭脚本本身和随附的 Tektur 字体生成；生成结果已提交到仓库，构建不需要 Python。外观只在颜色和比例上参考原版风格：没有复制、描摹或重新采样原版的任何内容。
- **字体**：Tektur（SIL OFL，`assets/fonts/Tektur-Variable.ttf`，版本和校验和见 `assets/fonts/README.md`），一种方正的科技感字体，气质接近原版宽体菜单字体；小写字母以小型大写字母绘制，略宽、字距略大，与原版一致。它缺少的字符取自 Noto Sans，中文取自 Noto Sans SC。字形在运行时光栅化到图集中，并按与游戏字体相同的矩形、对齐、换行和阴影排版，因此各画面保持原版的布局（`crates/game/src/fallback.rs`）。
- **文字**（`assets/own/Texts/menu.txt`，内置于游戏）：菜单、关卡内和结束画面的全部文字，有英语、俄语、德语、简体中文、法语、西班牙语和意大利语，原版 `Language.nmo` 有对应键时沿用其键名。使用原版界面时，优先采用拷贝在其五种语言中的文字；俄语和中文（拷贝没有）使用我们的文字，以随附字体绘制。

自有界面的默认高分榜也是我们自己的（十个名字，3000 到 750 分）。无论使用哪种界面，仍来自拷贝的有：默认按键（base.cmo 的 DB_Options；没有时为方向键、Shift 和空格），以及关卡的部件（球、粒子、球的阴影、天空、机关），之后的阶段会替换它们。

自有内容分阶段推出：界面（字体、按钮、分数框、生命球、文字、菜单场景），然后是球、天空、粒子和音效（来自开放素材库），带中国元素、类似麻将游戏风格的音乐，最后是自己的模块和关卡。

音效和音乐以**音效主题**提供，每个主题是一个文件夹（`Sounds/` 和 `Music/` 中按游戏请求的名称存放，Ogg Vorbis 或 WAV，关卡主题用的 `music.txt` 表，以及包含四种语言名称、作者、许可证和 `fallback`（`none` 表示不使用其他主题）的 `theme.txt`）。游戏在 `assets/sounds/` 中随附两个主题：*system*（自由许可的录音和项目所有者选择的音乐，由 `assets/sounds/system/prepare.py` 制作，CC0 或 CC-BY，作者和改动见其 `CREDITS.md`，也显示在 Credits 画面上）和 *neuro*（为游戏合成的音效和关卡音乐，GPL-3.0-only；关卡音乐是古筝风格的简短弹拨乐句，五个主题的三个部分各有一段；它包含游戏请求的所有声音，不从其他主题获取任何内容，`fallback none`）。玩家可以在数据文件夹的 `sounds/<名称>/` 中添加自己的主题；其中 *original* 由 toolbox 从玩家的拷贝制作，从不随游戏分发（在它存在之前，选择它会播放拷贝中的音效）。Extended 页面上的 *Sounds* 一行用于选择主题（`sound-theme 名称`）；在玩家选择其他主题之前（经典模式下始终）播放 *neuro*（decisions.md，“生成的音效主题为默认”）；玩家做出的选择会被保留。主题及其后备主题都没有的音效取自拷贝（关卡自己的音效）；音乐从不如此。玩家的主题文件夹不受信任：只接受简单名称，不跟随链接，主题数量有上限，音效通过音频层的限制解码。关卡音乐会在单独的线程中预先解码。
