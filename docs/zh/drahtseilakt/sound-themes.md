# 音效主题

Drahtseilakt 中的每个音效和每段音乐都来自一个**音效主题**：一个文件夹，其中的文件按游戏播放的**角色**命名（掉落、
检查点、木球在石头上滚动、关卡音乐……）。游戏自带两个主题，任何人都可以再做一个：另一套录音、另一种音乐、一个重制版。
本页面向主题作者：文件夹里放什么，每个角色是什么，原版中该角色的声音有多长、多响，以及如何分享主题让别人使用。

## 主题放在哪里

- **随游戏提供**，在 `assets/sounds/` 中：*system*（自由许可的录音和音乐，所有其他主题的后备主题）和 *neuro*
  （为游戏合成的音效和音乐，默认主题）。
- **玩家自己的主题**，在数据文件夹的 `sounds/<名称>/` 中：Linux 上是 `~/.local/share/drahtseilakt/sounds/`，
  Windows 上是 `%APPDATA%\drahtseilakt\sounds\`，macOS 上是 `~/Library/Application Support/drahtseilakt/sounds/`。
  安装主题就是把它的文件夹复制到那里。

文件夹名就是主题的键（设置中保存的值）：字母、数字、`_`、`-` 和 `.`，不能以点开头，最多 64 个字符。玩家文件夹若与
随附主题同名（`system`、`neuro`）则被忽略；`original` 是 toolbox 用玩家自己的 Ballance 拷贝制作的主题的名称。最多读取
32 个主题。游戏在启动时查找主题：添加主题后请重新启动。

## 文件夹

```
sounds/
  calm/
    theme.txt          名称、作者、许可证、后备主题
    CREDITS.md         每个文件的作者和许可证（建议提供）
    Sounds/            音效：Misc_Fall.ogg、Hit_Wood_Wood.ogg ……
      sounds.txt       可选：每个角色多个文件（池）
    Music/             音乐：Music_Atmo_1.ogg、Music_Theme_1_1.ogg ……
      music.txt        可选：每个角色多个文件（池），或任意命名的文件
```

角色的文件是 `Sounds/<角色>.ogg` 或 `.wav`（或 `Music/` 中的同名文件；游戏两处都查找）。主题不必填满所有角色：缺少的
角色取自它的后备主题（见 `theme.txt`）。

只读取普通文件：名称由字母、数字、`_`、`-` 和 `.` 组成；从不跟随链接（符号链接、junction）；`Sounds/` 和 `Music/`
以外的子文件夹被忽略。

## theme.txt

UTF-8 纯文本，最多 4 KiB，每行一个 `键 值`；以 `#` 开头的行是注释。

| 键 | 含义 |
|---|---|
| `name.en`、`name.ru`、`name.de`、`name.zh` | 设置中各语言的主题名称（最多 64 个字符）。缺少某语言时显示英文名称，否则显示文件夹名。 |
| `author` | 作者（供人阅读；游戏暂不显示）。 |
| `licence` | 许可证，例如 `CC0 1.0` 或 `CC-BY 4.0，各文件见 CREDITS.md`（供人阅读）。 |
| `fallback` | 本主题缺少的角色从哪个主题获取：一个主题的键，或 `none` 表示不使用其他主题（主题是完整的）。省略时为 *system*。 |

```
# Calm：适合长关卡的柔和录音。
name.en Calm
name.ru Тихая
name.de Ruhig
name.zh 平静
author Jane Doe
licence CC-BY 4.0 (CREDITS.md)
fallback system
```

查找角色的顺序：所选主题、它的后备主题、然后 *system*（`fallback none`：只有所选主题）。一个角色的所有文件都来自同一个
主题：第一个拥有该角色的主题。所有主题都没有的声音取自关卡自己的音效或玩家的拷贝（仅限音效）；音乐从不如此。

## 角色

下面原版的数据是在其文件上测得的：长度，以及声音足够长时的综合响度（EBU R 128，LUFS）；很短的声音标为“短”。它们是
参考而非规定：主题整体可以更响或更轻，但各声音之间应大致保持这些比例，因为游戏像原版一样混合它们（例如滚动声的音量
和速度随球速变化）。

**循环**表示游戏反复播放该文件：它必须无缝衔接（结尾接回开头，例如用交叉淡化）。其他声音都是**单次**播放。

### 音效（`Sounds/`）

| 角色 | 何时播放 | 类型 | 原版 |
|---|---|---|---|
| `Misc_StartLevel` | 一次运行开始 | 单次 | 4.2 秒，−18 LUFS |
| `Misc_Checkpoint` | 到达检查点 | 单次 | 3.5 秒，−16 LUFS |
| `Misc_Fall` | 球丢失 | 单次 | 5.6 秒，−18 LUFS |
| `Misc_Lightning` | 新球出现 | 单次 | 3.5 秒，−25 LUFS |
| `Misc_Trafo` | 变换器更换球 | 单次 | 3.1 秒，−22 LUFS |
| `Extra_Life_Blob` | 碰到额外生命 | 单次 | 0.3 秒，短 |
| `Misc_extraball` | 获得额外生命（`Extra_Life_Blob` 之后 0.3 秒） | 单次 | 0.9 秒，短 |
| `Extra_Start` | 额外分数被释放 | 单次 | 3.0 秒，−28 LUFS |
| `Extra_Hit` | 收集一个额外分数 | 单次 | 0.5 秒，−36 LUFS |
| `Misc_RopeTears` | 绳子断裂 | 单次 | 0.2 秒，短 |
| `Misc_Ventilator` | 风扇吹风（音量随距离变化） | 循环 | 1.1 秒，−23 LUFS |
| `Music_thunder` | 雷暴关卡中闪电后的雷声（虽然名字如此，但属于音效：使用音效音量） | 单次 | 12 秒，−18 LUFS |
| `Menu_click` | 菜单选择 | 单次 | 0.03 秒，短 |
| `Menu_load` | 关卡加载中 | 单次 | 5.2 秒，−19 LUFS |
| `Roll_<球>_<地面>` | 球在滚动（音量和速度随球速变化） | 循环 | 1–2 秒，−27 至 −41 LUFS |
| `Hit_<球>_<地面>` | 球撞到地面或墙 | 单次 | 0.1–0.3 秒，短 |

球：`Wood`、`Stone` 和 `Paper`；地面：`Stone`、`Wood`、`Metal`。滚动：`Roll_Wood_Stone`、`Roll_Wood_Wood`、
`Roll_Wood_Metal`、`Roll_Stone_Stone`、`Roll_Stone_Wood`、`Roll_Stone_Metal`，以及纸球在任何地面上的 `Roll_Paper`。
撞击：`Hit_Wood_Stone`、`Hit_Wood_Wood`、`Hit_Wood_Metal`、`Hit_Stone_Stone`、`Hit_Stone_Wood`、`Hit_Stone_Metal`，
圆顶 `Hit_Wood_Dome`（2.0 秒，−18 LUFS）和 `Hit_Stone_Kuppel`（1.2 秒，−21 LUFS），以及纸球的 `Hit_Paper`。原版的
滚动循环从轻到响：纸和木在石头上约 −41，木在金属上 −37，石在石上 −36，石在木上 −29，木在木上 −27，石在金属上
−28 LUFS。

关卡可以自带音效（社区关卡的 `Sounds/` 文件夹）；它们优先于任何主题。

### 音乐（`Music/`）

音乐有自己的音量设置。关卡如何播放音乐（与原版的音乐管理器相同）：

| 角色 | 何时播放 | 类型 | 原版 |
|---|---|---|---|
| `Music_Atmo_1`、`_2`、`_3` | 氛围：从运行开始，随机停顿最多 15 秒后播放其中之一，反复进行 | 单次 | 26–28 秒，−30 至 −34 LUFS，一次缓慢的渐强和渐弱，没有节奏和低音 |
| `Music_Theme_<n>_1`、`_2`、`_3` | 关卡主题（`n` 来自关卡表，原版关卡为 1 到 5）：从第 7 秒起，随机停顿最多 50 秒后播放其中一个部分，反复进行；最后一个检查点之后停止 | 单次 | 4–23 秒，−13 至 −19 LUFS |
| `Music_EndCheckpoint` | 最后一个检查点之后、接近关卡终点时，直到完成 | 循环 | 14 秒，−13 LUFS |
| `Music_Final` | 关卡完成 | 单次 | 26.5 秒，−16 LUFS |
| `Music_LastFinal` | 最后一关完成（代替 `Music_Final`） | 单次 | 7.7 秒，−17 LUFS |

氛围和主题同时播放，各有自己的停顿，所以氛围应当非常安静、平和。原版的主题部分是短乐句；更长的乐曲也可以（*system*
的是长达 100 秒、约 −17 LUFS 的完整乐曲），但关卡进行时每首乐曲都以解码后的形式保存在内存中（100 秒立体声约
35 MB），所以最好控制在两分钟左右以内。同时最多保存 384 MB 解码后的音乐；超出预算或无法解码的乐曲在本次运行中从其池中移除，改播另一首。`Music_Highscore`（原版 2 秒的短曲）会被接受，但暂不播放。

## 池：一个角色多个文件

一个角色可以有多个文件，即一个**池**：游戏每次播放该角色时随机选其中一个，从不连续两次选同一个。池由表
`Music/music.txt` 和 `Sounds/sounds.txt` 给出，两者格式相同：

```
# 角色 文件 [文件 ...]
Music_Atmo_1 bowl.ogg flute.ogg
Music_Atmo_1 wind.ogg
Music_Theme_4_1 river.ogg
Music_Theme_4 garden.ogg dusk.ogg
Music_Final fanfare_a.ogg fanfare_b.ogg
```

- 每行一个角色：先写角色，再写一个或多个文件名，用空格或制表符分隔。同一角色写在多行时会合并所有文件。旧的单文件行
  （`Music_Theme_1_1 piece.ogg`）也是同一格式。
- 以角色命名的文件（`Music/Music_Atmo_1.ogg`）如果存在，就是其池中的第一个；表格在此基础上添加。表格只能指向自己
  文件夹中的文件：`music.txt` 指向 `Music/` 中的文件，`sounds.txt` 指向 `Sounds/` 中的文件。
- 氛围和关卡主题在其所有部分的**全部**文件中一起选择：氛围在 `Music_Atmo_1`、`_2`、`_3` 及分组 `Music_Atmo` 的池中
  选择；主题 `n` 在 `Music_Theme_<n>_1` 至 `_3` 及分组 `Music_Theme_<n>` 的池中选择。因此一个主题可以有任意数量的
  氛围和乐曲，分不分部分都可以。出现在多处的文件只算一次。
- `Music_EndCheckpoint`、`Music_Final` 和 `Music_LastFinal` 每个关卡从各自的池中选一个；每个音效每次播放都从自己的池
  中选一个（例如多种撞击声）。关卡自带的音效会代替池播放。
- 音乐的选择来自一个种子：回放（replay）每次都以相同顺序播放音乐，新的运行则是新的顺序。音乐从不改变游戏本身。
- 角色名比较时不区分大小写，文件名则完全匹配。

表格作为不受信任的文件读取（主题由陌生人分享），并经过严格检查：

- 最多 16 KiB 和 512 行，必须是 UTF-8（允许开头带字节顺序标记）；超出或不是 UTF-8 的表格整体忽略；
- 文件名是以 `.ogg` 或 `.wav` 结尾的简单名称（字母、数字、`_`、`-`、`.`，不能以点开头）：不能有文件夹、路径或
  盘符。指向其他内容的行整行跳过；
- 游戏不认识的角色被忽略；每个角色最多 16 个文件，最多 256 个角色；
- 列出但不存在的文件（或链接）被略去。

每一行被跳过的内容都会在终端（标准错误）中报告，例如
`sound theme table .../Music/music.txt: line 3: My_Song is not a role the game plays: ignored`。

## 格式

- **Ogg Vorbis**（`.ogg`，推荐）或未压缩的 PCM **WAV**（`.wav`，8 或 16 位）；单声道或立体声；采样率 1 至 192 kHz
  均可（推荐 44.1 kHz；游戏会重新采样）。音效用单声道即可，音乐适合立体声。
- 文件最大 64 MB，解码后最长十分钟。损坏的文件会被跳过，改用下一个主题的文件。
- Ogg Vorbis 质量 3–5（`ffmpeg -c:a libvorbis -q:a 4`）就足够了。
- 通过测量来匹配响度，例如 `ffmpeg -i file.ogg -af ebur128 -f null -`（最后一个 `I:` 是综合响度）。*system* 的
  `prepare.py`（在 `assets/sounds/system/` 中）展示了完整的制作过程：剪切、淡入淡出、无缝循环、响度匹配和编码。

## 测试主题

1. 把文件夹放进数据文件夹的 `sounds/` 中，然后启动游戏。
2. 切换到扩展模式，在扩展选项页的 **Sounds** 一行选择该主题（即 `settings.txt` 中的设置 `sound-theme 名称`）。
   经典模式在其 Sound 页有同样的选择（**Sound theme** 一行）。
3. 玩一个关卡，留意终端中关于表格的提示。缺少的角色由后备主题补上，所以 `fallback none` 的主题应检查是否有遗漏：
   上面表格中的所有角色。
4. 想不玩就试听，可以运行回放并把混音写入文件：
   `DRAHTSEILAKT_SOUND_FILE=mix.wav drahtseilakt --replay run.replay`（混音写入 `mix.wav`，而不是送到扬声器）。

## 许可与分享

- 使用自己的录音和音乐，或采用 **CC0** 或 **CC-BY**（3.0 或 4.0）许可的作品，这样任何人都能分享这个主题。避免
  NonCommercial（NC）和 NoDerivatives（ND）许可：游戏会剪切并调整每个文件的响度。
- 在主题中放一个 `CREDITS.md`：为每个文件写明其角色、作品标题、作者、许可证、来源网址以及你做了哪些改动（剪切、
  淡入淡出、调整响度、滤波）。CC-BY 要求这样署名；*system* 的 `CREDITS.md` 是一个示例。
- **绝不要包含原版游戏的音效或音乐**（或任何其他游戏的），即使经过编辑也不行。原版的声音留在每位玩家自己的电脑上：
  *original* 主题由 toolbox 在本地用玩家自己的拷贝制作，从不分享。

## 随附的主题

- *system*（`assets/sounds/system/`）：自由许可的音效（Kenney、freesound 和 OpenGameArt 的作者），分为三个部分的
  八段氛围，十八首关卡乐曲（主题 1 到 3 各三首，主题 4 和 5 另有九首，放在池中）以及终点音乐；由 `prepare.py`
  制作；作者、许可证和改动见其 `CREDITS.md`。
- *neuro*（`assets/sounds/neuro/`）：所有角色都由代码为游戏合成，GPL-3.0-only，`fallback none`。

**显示的名称。** 主题 `original` 在所有语言中显示为 **Ballance**；*neuro* 和 *system* 保持原名。玩家的主题显示其 `name.<语言>`（或文件夹名），**最多 16 个字符**：更长的名称在词边界处截断并以“…”结尾，游戏会在日志中给出警告。文件夹键的上限（64 个字符）不变。
