# 解析器的模糊测试

凡是读取非游戏自己写出的文件的代码——`.nmo`/`.cmo` 关卡与社区地图、开放关卡格式、图片、声音、CD 镜像及其
InstallShield 压缩包、录像、提示文件、关卡文件夹的 `level.txt`、音效主题、设置与高分文件——都有一个模糊测试目标。
有两种运行方式。

## 稳定版模糊器（随处可运行，离线）

`crates/fuzzkit` 是一个没有依赖的小型变异模糊器。每个目标都是解析器其他测试旁边的一个 `#[ignore]` 测试；它用固定的
xorshift 生成器变异种子语料（翻转位、特殊值、插入、删除、复制、拼接、截断、调整长度字段，文本格式还会变动行、词和
数字），因此每个输入都能按编号重新生成。每个输入都在监护下运行：panic 会被捕获；超过时限或超过目标内存上限的输入
会结束本轮；中止（在地址空间限制下内存分配失败、栈溢出）则通过重放最后一段来重新找到。

```sh
scripts/fuzz.sh                    # 所有目标，每个 600 秒，同时 4 个
scripts/fuzz.sh 3600 2 nmo_file    # 单个目标，一小时
FUZZ_SECS=60 cargo test --release --offline -p level --test fuzz fuzz_nmo_file -- --ignored --nocapture
```

目标：`nmo_file`、`nmo_chunk`、`open_level`（crate `level`）；`image`、`level_txt`、`theme_txt`、`pack_txt`、`sound_table`
（`gamedata`）；`disc_image`、`cue_sheet`、`iscab`（`cdimage`）；`replay`、`hints`、`wav`、`ogg`、`highscore`、
`settings`、`level_sim`——加载关卡并运行一秒，让陌生人的地图越过读取器进入物理与规则——以及
`module_sim`——用一个模块文件替换原版关卡所用的模块（关卡文件夹可自带模块），需要玩家的游戏副本（游戏本身，
`crates/game/src/fuzz_tests.rs`）。

种子包括测试自己生成的小型合成文件、游戏自带的文件，以及（若存在）玩家的文件（`OPENBALLANCE_DATA`、
`OPENBALLANCE_MAPS`、配置目录中的原版与社区关卡）；它们在运行时读取，绝不写入仓库。`FUZZ_NO_REAL=1` 可排除它们。

设置：`FUZZ_SECS`（每个目标的时间）、`FUZZ_SEED`（选择哪些输入；`scripts/fuzz.sh` 每次取新的）、
`FUZZ_TIMEOUT_MS`（单个输入的时限，10 秒）、`FUZZ_RSS_MB`（内存上限）、`FUZZ_REPRO=<文件>`（运行单个输入并报告结果）。

发现的问题写入 `fuzz/artifacts/<目标>/`（git 忽略）：`crash-*`、`hang-*`、`oom-*` 或 `abort-*`，每个都有 `.bin`、
说明经过的 `.txt` 和最小化后的 `.min.bin`。每个修复都在解析器旁加一个回归测试，并在 `fuzz/regressions/<目标>/`
放一个合成的复现输入（绝不是游戏数据），由普通测试重放。

## cargo-fuzz（libFuzzer，nightly）

`fuzz/` 是工作区之外的 cargo-fuzz crate（游戏的构建从不需要它）。它的目标调用与稳定版模糊器测试相同的函数：

```sh
FUZZ_DUMP_CORPUS=1 cargo test --release --offline -p level --test fuzz fuzz_nmo_file -- --ignored  # 种子 -> fuzz/corpus/
cd fuzz && cargo +nightly fuzz run nmo_file corpus/nmo_file
```

游戏是二进制 crate，因此它自己的解析器中只有 `replay`（独立成文件）有 cargo-fuzz 目标；其余（`hints`、`wav`、
`ogg`、`highscore`、`settings`、`level_sim`、`module_sim`）由稳定版模糊器运行。
