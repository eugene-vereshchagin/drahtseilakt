# Drahtseilakt

[English](README.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · **简体中文**

*Drahtseilakt*（德语，意为“走钢丝”）是德国人对一切走错一步就会坠落的平衡表演的称呼。Ballance（平衡球）
由德国小工作室 Cyparade 于 2004 年制作，讲的正是这件事：一个球在深渊之上的轨道上。这个名字是我们向这款游戏
以及制作它的人们致敬。

Drahtseilakt 是 **Ballance**（Cyparade / Atari，2004）的一个新引擎，用 Rust 从零写成。它运行你自己的游戏拷贝
中的数据 — 原样的原版关卡以及自制地图 — 支持 Linux、Windows 和 macOS，任意分辨率和宽高比，最高 8K，
带有第 13 关和自制地图菜单，之后还会有一种开放、有文档的关卡格式及其转换器。

本项目最初是 emdzej 的 [OpenBallance](https://github.com/emdzej/openballance) 的一个分支，OpenBallance 在自己的
运行时上运行原版 Virtools 脚本；它的历史记录和逆向工程笔记得到保留，而它的代码正在被替换
（原因见[决策](docs/zh/drahtseilakt/decisions.md)）。

状态：开发中 — [路线图](docs/zh/drahtseilakt/roadmap.md)的阶段 1，Virtools 文件读取器。这款游戏究竟为什么
需要这些：[历史](docs/zh/drahtseilakt/history.md)。

## 构建和游玩

Rust（stable，1.87 或更新版本）和 cargo；在 Linux 上需要 Vulkan 驱动。你需要自己的 Ballance 拷贝。

```sh
cargo build --release
target/release/drahtseilakt --data /path/to/Ballance --level 1
```

`drahtseilakt --import DIR` 会把 DIR 中你已安装的 Ballance 原版游戏（或你的游戏 CD 镜像：`--import Ballance.cue`、`.bin` 或 `.iso`；没有找到游戏时启动的窗口提供同样的功能：电脑上找到的内容、拖到窗口上的文件夹或镜像、输入的路径）复制到游戏的 `levels/original/`；所有关卡都放在
游戏数据文件夹（Linux 上为 `~/.local/share/drahtseilakt/`）的 `levels/` 中，每个文件夹里都有说明应放什么的 README
（[关卡存放位置](docs/zh/drahtseilakt/levels.md)）。`--data`（或 `OPENBALLANCE_DATA`）则改为从另一个 Ballance 文件夹游玩。独立程序 `drahtseilakt-toolbox remaster --scale 8` 会把原版关卡转换到 `levels/remastered/`（开放格式），形状更圆，纹理放大八倍：Real-ESRGAN 位于 toolbox 旁的 `realesrgan/`（版本固定在其仓库 drahtseilakt-toolbox 中）或在 PATH 中时使用它（会补出细节），否则使用 Lanczos；它们在 Start → Remastered 中游玩。游戏本身两者都不需要。`--level N` 游玩原版第 N 关，`--map FILE.nmo`
游玩任意关卡文件或自制地图，`--open DIR` 游玩[开放格式](docs/zh/drahtseilakt/open-format.md)的关卡；
`--export DIR` 把所选关卡转换为该格式。方向键相对于相机滚动球，Shift+左/右
转动相机，空格键抬高相机，F11 切换全屏，Esc 暂停。不指定关卡时，游戏以
主菜单启动。菜单就是原版的菜单（[菜单](docs/zh/drahtseilakt/menu.md)），使用这份拷贝自己的文字：Start
（完成前一关后下一关才会解锁）、Highscore（每关十个名次，保存在我们的数据文件夹中）、
Options、Credits、Exit；在关卡中按 Esc 打开暂停菜单，失败的一局打开死亡菜单，完成的一局显示分数、
新高分的姓名输入以及结束菜单。

每一局通关都会保存为回放（在 Linux 上位于 `~/.local/share/drahtseilakt/replays/<level>/`，
最快的一局保存为 `best.replay`）；`--replay FILE` 播放回放，`--headless` 在无窗口的情况下检验它，`--screenshot
FILE.png` 渲染一帧。要测试某个位置，`--sector N` 从某个检查点开始，`--at X,Y,Z`（或关卡
对象的名称）可从任意位置开始，配合 `--ball wood|stone|paper`；这样的一局会把起点保存在回放中，并且永远不会
算作最佳成绩。在开启扩展模式之前（Options、Extended，或用 `--debug` 开启一次），游戏会尽可能
贴近原版。*扩展模式*会解锁所有关卡，显示最佳时间，在音乐音量旁显示音效音量，并在 Start 下按类别列出关卡
（`levels/` 中的原版、重制、自制地图和开放格式关卡），还会
加入我们的工具：R 重新开始，F2 启用自由相机在关卡中飞行，
同时这一局暂停等待，`[` 和 `]` 让游戏变慢和重新加快（这一局本身逐 tick
保持不变），F5 保存当前时刻，F9 回到该时刻（这一局的记录会截断到那里，因此长关卡可以
分段录制），并显示小节和 tick。以这种方式辅助的一局会被保存，但永远不算最佳成绩。用于测试时，`--autopilot` 让测试玩家在无窗口的情况下尝试通关
（配合 `--record FILE` 保存其过程）；`--follow FILE` 让它沿着玩家回放中球的路线前进；卡住时它会写报告，并遵循提示（[测试玩家](docs/zh/drahtseilakt/autopilot.md)）。`--locate DIR --record TRACK` 在原版视频的帧中找出球的路线（DIR/s0001.jpg 起，每秒四帧，例如 `ffmpeg -vf fps=4,scale=640:480`），方法是从球可能所在的位置绘制关卡并进行比较；测试玩家用 `--follow TRACK` 沿着这样的轨迹前进。`--frames DIR --replay FILE` 把回放绘制成这样的帧，带有真实位置，用于检验。`drahtseilakt --help` 列出全部选项。

## 法律信息

从 OpenBallance（GPL-3.0，emdzej 及贡献者）移植的文件在其文件头中注明了这一点；`ivp` crate 是物理引擎的移植。

Drahtseilakt 以 GPL-3.0 许可证发布，与它起步时所基于的 OpenBallance 相同。它需要你自己的
游戏拷贝，并且从不分发 Ballance 的数据。（在本仓库保持私有期间，`levels/` 中保存着原版关卡的
备份，供开发使用；公开仓库中不会包含它。）Ballance 版权所有 © 2004 Cyparade /
Atari。本项目与它们没有任何关联。
