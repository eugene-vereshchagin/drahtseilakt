# Drahtseilakt

[English](README.md) · **Русский** · [Deutsch](README.de.md)

*Drahtseilakt* (нем. «номер канатоходца») — так немцы называют любое балансирование, где шаг в сторону
означает падение. Ballance, сделанный в 2004 году небольшой немецкой студией Cyparade, ровно об этом: шар
на рейке над пропастью. Название — наш поклон игре и людям, которые её сделали.

Drahtseilakt — форк [OpenBallance](https://github.com/emdzej/openballance) от emdzej, точной реимплементации
**Ballance** (Cyparade / Atari, 2004), которая исполняет собственные Virtools-скрипты оригинальной игры
на новом рантайме, с портом её физического движка Ipion. Цель форка — полноценная нативная игра: настоящее
окно на Linux, Windows и macOS, любое разрешение и соотношение сторон до 8K, действительно круглый шар,
13-й уровень и меню любительских карт, а позже — открытый документированный формат уровней.

Состояние: в работе, ни одна из собственных целей форка пока не достигнута. См. [план работ](docs/ru/drahtseilakt/roadmap.md)
и [решения](docs/ru/drahtseilakt/decisions.md), стоящие за ним.

## Как это работает

Почти вся логика Ballance хранится в графах поведения Virtools внутри файлов `.cmo` / `.nmo`. Движок загружает
эти файлы в объектную модель, совместимую с CK2, и исполняет графы; Building Blocks, которые они вызывают,
портированы из оригинальных DLL, а физика — порт движка Ipion (IVP) из `physics_RT.dll`. Внутреннее устройство
описано в [`docs/`](docs/) (сайт документации апстрима: https://openballance.emdzej.pl; пока на английском).

## Сборка

Сейчас сборка — от апстрима (нативный бэкенд — фаза 1 плана):

```sh
tools/fetch-gasm-sdk.sh
cmake -S . -B build-gasm -DOPENBALLANCE_PLATFORM=gasm -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE=.deps/gasm-c-sdk/cmake/gasm-toolchain.cmake -DWASI_SDK_PREFIX="$PWD/.deps/wasi-sdk"
cmake --build build-gasm -j        # build-gasm/openballance.wasm

cmake -S . -B build && cmake --build build -j   # библиотека движка + тесты (нужны данные вашей игры)
```

## Правовое

Drahtseilakt распространяется под GPL-3.0, как и OpenBallance, от которого он отделился. В нём нет кода
или ассетов Ballance: нужна ваша собственная копия игры. Ballance © 2004 Cyparade / Atari. Проект с ними
не связан.
