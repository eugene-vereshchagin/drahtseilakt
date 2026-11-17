# Релизы и пакеты

Релиз состоит из трёх частей, которые меняются с разной скоростью, поэтому они собираются и поставляются
отдельно: обновление программы пересобирает только бинарные пакеты.

| Что | Меняется | Кто собирает | Выпускается как |
|---|---|---|---|
| **Бинарные пакеты**: программа | с каждой версией | workflow `Drahtseilakt release` (CI) | релиз с тегом `X.Y.Z` (черновик) |
| **Базовые данные**: `assets/` без 13 выбранных карт | редко | `scripts/pack-assets.sh` (машина владельца) | релиз с тегом `data-<DATA_VERSION>` |
| **Уровни**: 13 выбранных карт | редко | `scripts/pack-assets.sh` (машина владельца) | тот же релиз `data-<DATA_VERSION>` |

Данные и уровни не лежат в git («никаких бинарников в git»), поэтому в CI их не собрать; их делают на машине
владельца из локального `assets/` и загружают вручную.

## Базовые данные

`assets/` без выбранных карт (`assets/levels/drahtseilakt/NN-*`) и без файлов только для разработки (`mods-dev`,
`own/make`): свои детали, шрифты, звуки, фабричные моды, руководство, демонстрационный уровень (`00-Showcase`,
`Parts_Test`) и 13 карт сообщества `.nmo` (`assets/levels/community`), с которыми игра связана жёстко. Около 93 МБ.
Никаких файлов оригинального Ballance.

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

## Как сделать пакеты данных и уровней

```sh
scripts/pack-assets.sh          # OUT defaults to ../drahtseilakt-portable/bundles
```

Он создаёт в OUT (`V` = `scripts/DATA_VERSION`):

| Файл | Для |
|---|---|
| `drahtseilakt-data-V.tar.gz` | остальных систем; CI скачивает его для портативных архивов, установщика, .app |
| `drahtseilakt-data_V-1_all.deb`, `drahtseilakt-data-V-1.noarch.rpm` | Linux, базовые данные |
| `levels/NN-Name.zip` (13) | любой системы: распаковать в `assets/levels/drahtseilakt/` |
| `drahtseilakt-levels_V-1_all.deb`, `drahtseilakt-levels-V-1.noarch.rpm` | Linux, все 13 карт (зависит от пакета данных) |
| `SHA256SUMS` | всего перечисленного |

`.deb` и `.rpm` делает [nfpm](https://nfpm.goreleaser.com) 2.49.0 — один инструмент для обоих, так что тот же скрипт
работает на машине владельца (`rpmbuild` не нужен) и в CI. `scripts/fetch-nfpm.sh` скачивает его в
`third_party/` (в .gitignore; никогда в системные пути) и проверяет SHA-256, закреплённый в скрипте (из
`checksums.txt` самого релиза). `scripts/pack-assets.sh --no-packages` пропускает пакеты.

## Загрузка релиза данных

Выполняет владелец из рабочей копии; само ничего не запускается:

```sh
V=$(cat scripts/DATA_VERSION)
cd ../drahtseilakt-portable/bundles && sha256sum --check --quiet SHA256SUMS
gh release create "data-$V" --repo eugene-vereshchagin/drahtseilakt --draft --latest=false \
  --title "Drahtseilakt data $V" \
  --notes "Base data and the 13 picked maps for Drahtseilakt (data version $V). Programs: the releases X.Y.Z." \
  "drahtseilakt-data-$V.tar.gz" \
  "drahtseilakt-data_$V-1_all.deb" "drahtseilakt-data-$V-1.noarch.rpm" \
  "drahtseilakt-levels_$V-1_all.deb" "drahtseilakt-levels-$V-1.noarch.rpm" \
  levels/*.zip SHA256SUMS
```

Посмотрите черновик и опубликуйте его. Затем направьте CI на новые базовые данные (переменные репозитория, один раз
на версию данных). В закрытом репозитории файл релиза скачивается по его API-адресу (как bearer передаётся
`ASSETS_TOKEN`, если задан, иначе собственный токен запуска):

```sh
V=$(cat scripts/DATA_VERSION)
URL=$(gh api "repos/eugene-vereshchagin/drahtseilakt/releases/tags/data-$V" \
  --jq ".assets[] | select(.name == \"drahtseilakt-data-$V.tar.gz\") | .url")
SUM=$(sha256sum "../drahtseilakt-portable/bundles/drahtseilakt-data-$V.tar.gz" | cut -d' ' -f1)
gh variable set ASSETS_BUNDLE_URL --repo eugene-vereshchagin/drahtseilakt --body "$URL"
gh variable set ASSETS_BUNDLE_SHA256 --repo eugene-vereshchagin/drahtseilakt --body "$SUM"
```

(Для открытого релиза подойдёт и обычный адрес
`https://github.com/eugene-vereshchagin/drahtseilakt/releases/download/data-$V/drahtseilakt-data-$V.tar.gz`.)
`scripts/ci-assets.sh` предупреждает, если в адресе нет версии из `scripts/DATA_VERSION`.

## Бинарный релиз (CI)

Actions -> «Drahtseilakt release» -> Run workflow (файлы — артефакты запуска) или push тега `X.Y.Z`: запуск также
создаёт **черновик** релиза с файлами ниже и `SHA256SUMS` (без данных и уровней). Параметр `dev` добавляет
dev-вариант (в релиз не попадает никогда). Push в ветку ничего не запускает.

Где `<V>` — тег (или `git-<sha>` вне тега), делаются такие файлы:

| Цель | Файлы |
|---|---|
| Windows x86_64, arm64 | `drahtseilakt-<V>-windows-<arch>-setup.exe` (установщик), `drahtseilakt-<V>-windows-<arch>.zip` (портативная) |
| Linux x86_64, aarch64 | `drahtseilakt-<V>-linux-<arch>.tar.gz` (портативная), `drahtseilakt_<V>-1_<amd64\|arm64>.deb`, `drahtseilakt-<V>-1.<x86_64\|aarch64>.rpm` |
| Linux x86_64 DEV (параметр `dev`) | `drahtseilakt-<V>-linux-x86_64-dev.tar.gz`, `drahtseilakt-dev_<V>-1_amd64.deb` |
| macOS arm64, x86_64 | `drahtseilakt-<V>-macos-<arch>.dmg` (`.app` без подписи), `drahtseilakt-<V>-macos-<arch>.tar.gz` (портативная) |
| FreeBSD x86_64 (экспериментально) | `drahtseilakt-<V>-freebsd-x86_64.tar.gz` |

- **Портативные архивы** и **установщик Windows** включают базовые данные, поэтому работают сами по себе; выбранные
  карты — отдельные `.zip` из релиза данных. В портативном архиве есть и тулбокс.
- **Установщик Windows** (NSIS, собирается на Ubuntu из портативного zip): по умолчанию для одного пользователя
  (`%LOCALAPPDATA%\Programs\Drahtseilakt`, без администратора), папка в меню «Пуск», деинсталлятор в «Приложениях».
  `portable.txt` он не ставит, поэтому данные игрока идут в профиль, а удаление их не трогает. Без подписи: SmartScreen предупредит.
- **Пакеты Linux**: программа в `/usr/bin/drahtseilakt` (системные библиотеки: `libasound2`, `libudev1`), данные там,
  где их ищет игра, `/usr/share/drahtseilakt/assets` (программа находит их от `/usr/bin/../share`, затем в
  `$XDG_DATA_DIRS`: `files.md`). Файл `.desktop` и значок (favicon сайта, масштабированный `tools/icon.py`,
  новый файл в git не нужен). `drahtseilakt` зависит от `drahtseilakt-data (>= DATA_VERSION)`; `drahtseilakt-levels`
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
