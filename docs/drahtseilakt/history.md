# Why Ballance needed a second life

Ballance did not fade away because it got old. It was stranded: everything it ran on belonged to companies
that moved on, and a game built on a closed engine dies with the engine.

## The engine was bought and switched off

Ballance was made with **Virtools**, a commercial tool from Virtools SA, a Paris company founded in 1993. In
Virtools, game logic was assembled visually from ready-made Building Blocks wired into behaviour graphs;
programmers only wrote the blocks. Ballance's whole game lives in those graphs.

In July 2005 Dassault Systèmes bought Virtools for about €12 million. From 2006 the product was sold as
*3DVIA Virtools* under Dassault's 3DVIA brand; after that it stopped being developed and was taken off the
market around 2009. The file format was never published. The runtime only ever shipped for Windows.

## The studio was gone too

Cyparade, the small German studio behind Ballance, stopped updating the game's website around 2006 and the
site went offline in 2009. There was nobody left to port the game, and no engine left to port it with.

## The game outlived both

The game itself never quite died. A community kept it going: hundreds of custom levels made since 2009, a mod
loader, Blender tools for map making, speedruns. In January 2024 Ballance was re-released on Steam and GOG
(Ziggurat Interactive) — the same 2004 Windows build, capped at 1600×1200, with the original controls.

In 2026 emdzej's [OpenBallance](https://github.com/emdzej/openballance) did what the engine's owners never
would: it reimplemented the Virtools runtime, the Building Blocks the game uses and its Ipion physics, and ran
the original game data on them. Drahtseilakt forks it to make that a proper native game.

## A familiar pattern

The same owner did the same to a tool people relied on. Dassault's DraftSight, a free 2D CAD program, got a
Linux beta in 2011; the Linux version stayed a beta for years, the program later became paid, and the Linux
build was gone. Users who had built their work on it were left to find something else — open projects such
as OpenCADStudio are what such users turn to now.

The lesson the fork is built on: what matters to people should not depend on one vendor's goodwill. Hence
open code, documented formats (the [roadmap](roadmap.md)'s open level format), and the original format
supported forever, so the community's work keeps running whatever happens to anyone's business plans.
