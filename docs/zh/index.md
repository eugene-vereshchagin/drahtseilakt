---
layout: home

hero:
  name: Drahtseilakt
  text: Ballance，再次滚动
  tagline: 一款基于 OpenBallance 的原生游戏。OpenBallance 是对这款 2004 年经典（平衡球）的忠实重新实现：原版游戏脚本运行在一个新的、与 Virtools 兼容的运行时上，配合移植自游戏本身的物理引擎，使用你自己的游戏拷贝中的数据。
  image:
    src: /screenshots/level01.jpg
    alt: 木球在第 1 关中滚动
  actions:
    - theme: brand
      text: 路线图
      link: /zh/drahtseilakt/roadmap
    - theme: alt
      text: 决策
      link: /zh/drahtseilakt/decisions

features:
  - title: 忠实还原
    details: Ballance 的游戏逻辑存在于 Virtools 行为图中，引擎运行的正是这些原版行为图。它们调用的 Building Blocks 逐个从原版 DLL 移植而来，连同所有怪癖。
  - title: 原版物理
    details: 球的手感来自 physics_RT.dll 中的 Ipion 物理引擎。它的核心、碰撞检测、摩擦和碰撞求解器都从反汇编中移植，常量对常量。
  - title: 你自己的数据
    details: 关卡、纹理、声音和音乐直接从你的光盘镜像、光盘本身或已安装的拷贝中读取。不附带任何资源文件。
  - title: 中文文档
    details: Drahtseilakt 的全部页面（路线图、决定、菜单、关卡、自制地图、文件格式）都有中文版。用户指南和内部实现说明目前只有英文。
---
