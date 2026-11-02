# 界面资源

游戏按原版的目录结构请求文件；每个文件扮演一个**角色**（菜单字体、坠落音效、木球……）。根据“默认使用自有内容”的决定（[decisions.md](decisions.md)），一个角色可由三个来源提供：

- **自有**：随游戏提供、放在 `assets/own/` 中并按原版结构排列的我们自己的文件（目前还没有：第 2 阶段制作）；
- **原版**：玩家的拷贝；
- **重制**：toolbox 对拷贝的重制，位于 `levels/remastered/.interface/`。

扩展模式的 **Interface** 开关（own / original / remaster，设置项 `interface`）决定谁优先；从下次启动起生效。经典模式始终使用拷贝。没有任何来源提供的角色即为缺失：游戏会提示一次，然后在没有它的情况下继续运行。关卡自己的文件夹（`<关卡>/game/`，见 [open-format.md](open-format.md)）对其部件依然最优先。

游戏字体无法绘制的文字（它只有 Latin-1）使用随附的 Noto 字体（`assets/fonts`，SIL OFL）绘制：例如自制地图的中文和西里尔字母名称。

| 角色 | 原版文件 | 自有 |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | 计划中 |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | 计划中 |
| texts in the copy's languages | `3D Entities/Language.nmo` | 计划中 |
| menu texts, level table defaults | `base.cmo` | 计划中 |
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | 计划中 |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | 计划中 |
| transformer animation | `3D Entities/AnimTrafo.nmo` | 计划中 |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | 计划中 |
| game font texture | `Textures/Font_1.tga` | 计划中 |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | 计划中 |
| ball shadow | `Textures/HardShadow.bmp` | 计划中 |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | 计划中 |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | 计划中 |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | 计划中 |
| menu sounds | `Sounds/Menu_click, Menu_load` | 计划中 |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | 计划中 |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | 计划中 |
| original levels | `3D Entities/Level/Level_NN.NMO` | 计划中 |

自有内容分阶段推出：界面（字体、按钮、分数框、生命球、文字、菜单场景），然后是球、天空、粒子和音效（来自开放素材库），带中国元素、类似麻将游戏风格的音乐，最后是自己的模块和关卡。
