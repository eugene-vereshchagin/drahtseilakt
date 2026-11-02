# Ballance 为什么需要第二次生命

Ballance 并不是因为老了才逐渐消失的。它是被困住了：它所依赖的一切都属于早已转向别处的公司，而建立在封闭引擎上的游戏，会随引擎一起死去。

## 引擎被收购，然后被关掉

Ballance 是用 **Virtools** 制作的，这是 Virtools SA 的一款商业工具，该公司 1993 年成立于巴黎。在 Virtools 中，游戏逻辑是用现成的
Building Blocks 连接成行为图、以可视化方式搭建出来的；程序员只负责编写这些块。Ballance 的整个游戏都在这些图里。

2005 年 7 月，Dassault Systèmes 以约 1200 万欧元收购了 Virtools。从 2006 年起，该产品以 *3DVIA Virtools* 之名在 Dassault 的 3DVIA
品牌下销售；此后它停止开发，大约在 2009 年退出市场。文件格式从未公开。运行时也只发布过 Windows 版。

## 工作室也不在了

制作 Ballance 的德国小工作室 Cyparade 大约在 2006 年停止更新游戏网站，网站于 2009 年下线。再也没有人来移植这款游戏，也没有引擎可以用来移植它。

## 游戏比两者都活得长

游戏本身从未真正死去。一个社区让它延续了下来：2009 年以来制作的数百张自制关卡、一个模组加载器、用于制作地图的 Blender 工具、速通。
2024 年 1 月，Ballance 在 Steam 和 GOG 上重新发行（Ziggurat Interactive）——还是 2004 年的那个 Windows 版本，最高 1600×1200，操作也是原版的。

2026 年，emdzej 的 [OpenBallance](https://github.com/emdzej/openballance) 做到了引擎的所有者永远不会去做的事：它重新实现了 Virtools 运行时、
游戏用到的 Building Blocks 以及 Ipion 物理，并在其上运行了原版游戏数据。Drahtseilakt 分叉了它，要把它做成一款真正的原生游戏。

## 似曾相识的模式

同一个所有者对另一款人们依赖的工具也做过同样的事。Dassault 的 DraftSight 是一款免费的 2D CAD 程序，2011 年推出了 Linux 测试版；Linux 版
一直停留在测试阶段好几年，后来程序改为收费，Linux 版本也没了。把工作建立在它之上的用户只能另寻他路——如今这样的用户会转向
OpenCADStudio 之类的开源项目。

这个分支所依据的教训是：对人们重要的东西，不应该依赖某一家厂商的善意。因此要有开源代码、有文档的格式（[路线图](roadmap.md)中的开放关卡格式），
以及对原版格式的永久支持，这样无论谁的商业计划发生什么变化，社区的成果都能继续运行。
