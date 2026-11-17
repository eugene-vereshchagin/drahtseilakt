# Релизы и пакеты

Релиз состоит из трёх частей, которые меняются с разной скоростью, поэтому они собираются и поставляются
отдельно: обновление программы пересобирает только бинарные пакеты.

| Что | Меняется | Кто собирает | Выпускается как |
|---|---|---|---|
| **Бинарные пакеты**: программа и базовые данные в виде `.deb`/`.rpm` | с каждой версией | workflow `Drahtseilakt release` (CI) | релиз с тегом `X.Y.Z` (черновик) |
| **Уровни**: 13 выбранных карт | редко | `scripts/pack-assets.sh` (машина владельца) | релиз с тегом `data-<DATA_VERSION>` |

**Базовые данные лежат в git** (решение владельца 2026-11-17, decisions.md «Бинарники в git: базовые данные»), поэтому CI
берёт их из checkout и ничего не скачивает; пакеты `drahtseilakt-data` (`.deb`/`.rpm`) собираются из них в release-workflow.
В git нет только 13 выбранных карт (`level.bin` там больше лимита GitHub в 100 МБ на файл): их делают на машине владельца
из локального `assets/` и загружают вручную.

## Базовые данные

`assets/` без выбранных карт (`assets/levels/drahtseilakt/NN-*`, их `level.txt` остаётся в git) и без файлов только для
разработки (`mods-dev`, `own/make`): наши детали и текстуры, шрифты, звуки, фабричные моды, демонстрационный уровень
(`00-Showcase`, `Parts_Test`) и 13 общинных карт `.nmo` (`assets/levels/community`), на которых игра держится. Около 100 МБ
в git (355 файлов, самый большой 3,6 МБ), 93 МБ в пакете. Ни одного файла оригинального Ballance (при добавлении
проверено по хэшам относительно копии оригинала). Не в git, потому что CI делает дёшево: руководство игрока
(`assets/manual`, `tools/make_manual.py`, запускается `scripts/ci-assets.sh`). `.gitignore` игнорирует файлы выбранных карт,
кроме их `level.txt`, и производные `.dtx`, `level.source`, `*.remaster/`, `textures.original/`.

## Уровни

13 выбранных карт, по одному `.zip` (29-204 МБ, вместе 1,1 ГБ), и те же карты одним пакетом
`drahtseilakt-levels` (2,8 ГБ установленным, 1,1 ГБ в упаковке). Каждый файл меньше лимита GitHub в 2 ГиБ; скрипты
откажутся делать пакет больше.

## Версии

- `scripts/DATA_VERSION` — дата `ГГГГММДД` (например `20261117`). Это версия данных **и** версия, которой требуют
  пакеты программы: они зависят от `drahtseilakt-data (>= DATA_VERSION)`.
- Меняйте её, когда меняются данные или уровни (и коммитьте), затем упаковка, затем загрузка. Релиз программы, которому
  нужны более новые данные, чем в последнем релизе данных, требует сначала выпустить данные.
- Версия программы — тег `X.Y.Z`. Запуск workflow без тега даёт в пакетах `0~git<sha>`.
- Релиз данных получает тег `data-<DATA_VERSION>`; фильтр тегов workflow (`[0-9]+.[0-9]+.[0-9]+`) ему не
  соответствует, так что ничего не запускается.

## Как сделать пакеты уровней

```sh
scripts/pack-assets.sh          # OUT по умолчанию ../drahtseilakt-portable/bundles
```

Создаёт в OUT (`V` = `scripts/DATA_VERSION`):

| Файл | Для |
|---|---|
| `levels/NN-Name.zip` (13) | любой системы: распаковать в `assets/levels/drahtseilakt/` |
| `drahtseilakt-levels_V-1_all.deb`, `drahtseilakt-levels-V-1.noarch.rpm` | Linux, все 13 карт (зависит от пакета данных) |
| `SHA256SUMS` | всего перечисленного |

`--with-data` дополнительно делает старый `drahtseilakt-data-V.tar.gz` и пакеты данных из локального `assets/` — для
локальной проверки пакетов; их не загружают: CI собирает `drahtseilakt-data` из git
(`scripts/make-distro-packages.sh data --out DIR` из checkout; `--tar FILE` берёт архив).

`.deb` и `.rpm` делает [nfpm](https://nfpm.goreleaser.com) 2.49.0 — один инструмент для обоих, поэтому тот же скрипт
работает на машине владельца (`rpmbuild` не нужен) и в CI. `scripts/fetch-nfpm.sh` скачивает его в `third_party/`
(в gitignore; никогда в системный путь) и проверяет SHA-256, закреплённый в скрипте. `scripts/pack-assets.sh --no-packages`
пропускает пакеты.

## Загрузка релиза уровней

Выполняет владелец из checkout; само ничего не запускается:

```sh
V=$(cat scripts/DATA_VERSION)
cd ../drahtseilakt-portable/bundles && sha256sum --check --quiet SHA256SUMS
gh release create "data-$V" --repo eugene-vereshchagin/drahtseilakt --draft --latest=false \
  --title "Drahtseilakt levels $V" \
  --notes "The 13 picked maps for Drahtseilakt (data version $V). Programs and base data: the releases X.Y.Z." \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

Посмотрите черновик, затем опубликуйте. Больше ничего настраивать не нужно: CI не требует ни переменных, ни секретов, ни
скачивания данных.

## Бинарный релиз (CI)

Actions -> «Drahtseilakt release» -> Run workflow (файлы — артефакты запуска) или push тега `X.Y.Z`: запуск также
создаёт **черновик** релиза с файлами ниже и `SHA256SUMS` (без уровней). Параметр `dev` добавляет
dev-вариант (в релиз не попадает никогда). Push в ветку ничего не запускает.

Где `<V>` — тег (или `git-<sha>` вне тега), делаются такие файлы:

| Цель | Файлы |
|---|---|
| Windows x86_64, arm64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` (установщик), `drahtseilakt-<V>-windows-<arch>.zip` (портативная) |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` (портативная), `drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux (задание x86_64, один раз) | `drahtseilakt-data_<DATA_VERSION>-1_all.deb`, `drahtseilakt-data-<DATA_VERSION>-1.noarch.rpm` |
| Linux x86_64 DEV (параметр `dev`) | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` (`.app` без подписи), `drahtseilakt-<V>-macos-<arch>.tar.gz` (портативная) |
| FreeBSD x86_64 (экспериментально) | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **Портативные архивы** и **установщик Windows** включают базовые данные, поэтому работают сами по себе; выбранные
  карты — отдельные `.zip` из релиза данных (`data-<DATA_VERSION>`). В портативном архиве есть и тулбокс.
- **Установщик Windows** (NSIS, собирается на Ubuntu из портативного zip): по умолчанию для одного пользователя
  (`%LOCALAPPDATA%\Programs\Drahtseilakt`, без администратора), папка в меню «Пуск», деинсталлятор в «Приложениях».
  `portable.txt` он не ставит, поэтому данные игрока идут в профиль, а удаление их не трогает. Установщик, деинсталлятор и ярлыки несут наш значок (`scripts/packaging/icon.ico`, ставится рядом с программой). Без подписи: SmartScreen предупредит. **Пока нет:** значка внутри самого `drahtseilakt.exe` (Проводник, панель задач портативной копии): нужна build-зависимость для встраивания ресурсов, которой нет в офлайн-кэше крейтов; у окна значок есть (задаётся во время работы).
- **Портативная копия** (`portable.txt` рядом с программой) ищет ассеты только в своём `assets/`: не в дереве исходников, не в `/usr/share`, не в `XDG_DATA_DIRS`.
- **Пакеты Linux**: программа в `/usr/bin/drahtseilakt` (зависит от `libasound2`, `libudev1`; .rpm — от sonames `libasound.so.2`, `libudev.so.1`, `libc.so.6(GLIBC_2.35)`, которые есть и в Fedora, и в openSUSE, в отличие от имён пакетов вроде `systemd-libs`; рекомендует библиотеки оконной системы и графики, которые игра загружает во время работы: `libvulkan1`, `libxkbcommon-x11-0`, `libx11-6`, `libxcursor1`, `libxi6`, `libwayland-client0`), данные там,
  где их ищет игра, `/usr/share/drahtseilakt/assets` (программа находит их от `/usr/bin/../share`, затем в
  `$XDG_DATA_DIRS`: `files.md`). Файл `.desktop` и значок (`scripts/packaging/icon-512.png`, наш деревянный шар — та же картинка, что на сайте и в окне; 512 px и копия 256 px через `tools/icon.py`). `drahtseilakt` зависит от `drahtseilakt-data (>= DATA_VERSION)`; `drahtseilakt-levels`
  необязателен. Тулбокса в них нет (только в портативных).
  `drahtseilakt-dev` предоставляет `drahtseilakt` и конфликтует с ним (только `.deb`, из параметра `dev`).
- **macOS**: `.app` хранит данные в `Contents/Resources/assets`; без подписи и нотаризации, первый запуск —
  правой кнопкой -> «Открыть».

Установка на Debian/Ubuntu: `sudo apt install ./drahtseilakt-data_V-1_all.deb ./drahtseilakt_X.Y.Z-1_amd64.deb`
(и `./drahtseilakt-levels_V-1_all.deb`); Fedora: `sudo dnf install ./drahtseilakt-data-V-1.noarch.rpm ./drahtseilakt-X.Y.Z-1.x86_64.rpm`.

## Закрепление версий и права

Actions закреплены полным SHA коммита, Rust — точной версией, крейты — `Cargo.lock`, nfpm и Real-ESRGAN — по
SHA-256; у workflow `contents: read`, и только у задачи release `contents: write`. NSIS берётся из архива пакетов
Ubuntu 24.04 (подпись дистрибутива, без хеша; версия печатается в логе).
