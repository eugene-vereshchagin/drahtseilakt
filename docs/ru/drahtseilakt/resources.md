# Ресурсы интерфейса

Игра просит свои файлы по раскладке оригинала; каждый файл играет **роль** (шрифт меню, звук падения, деревянный
шар...). По решению «Своё по умолчанию» ([decisions.md](decisions.md)) роль могут заполнить три источника:

- **своё**: наши файлы, поставляемые с игрой в `assets/own/` в раскладке оригинала (пока нет: их делает этап 2);
- **оригинал**: копия игрока;
- **ремастер**: ремастер копии от toolbox, в `levels/remastered/.interface/`.

Переключатель **Interface** расширенного режима (own / original / remaster, настройка `interface`) выбирает, кто
первый; действует со следующего запуска. Классический режим всегда играет копию. Роль, которую не заполнил никто,
отсутствует: игра один раз говорит об этом и продолжает без неё. Своя папка уровня (`<уровень>/game/`, см.
[open-format.md](open-format.md)) по-прежнему первая для его частей.

Текст, который шрифт игры не умеет (в нём только Latin-1), рисуется поставляемыми шрифтами Noto (`assets/fonts`,
SIL OFL): китайские и кириллические названия карт сообщества.

| Роль | Файл оригинала | Своё |
|---|---|---|
| menu font and glyph table, buttons | `3D Entities/Menu.nmo (Font_1, M_FontData_01)` | в планах |
| menu 3D scene | `3D Entities/MenuLevel.nmo` | в планах |
| texts in the copy's languages | `3D Entities/Language.nmo` | в планах |
| menu texts, level table defaults | `base.cmo` | в планах |
| level table (sky, start ball, music, light, bonus) | `3D Entities/Levelinit.nmo` | в планах |
| balls, pieces, shadows | `3D Entities/Balls.nmo` | в планах |
| transformer animation | `3D Entities/AnimTrafo.nmo` | в планах |
| modules (boxes, fans, pushers...) | `3D Entities/PH/*.nmo` | в планах |
| game font texture | `Textures/Font_1.tga` | в планах |
| buttons and the points box | `Textures/Button01_special/select/deselect.tga` | в планах |
| ball shadow | `Textures/HardShadow.bmp` | в планах |
| skies | `Textures/Sky/Sky_<letter>_<face>.bmp` | в планах |
| particles | `Textures/Particles: Particle_Flames, Particle_Smoke, ExtraParticle, ExtraBall, P_Extra_Life_Particle` | в планах |
| event sounds | `Sounds/: Misc_StartLevel, Misc_Checkpoint, Misc_Fall, Misc_Lightning, Misc_Trafo, Misc_extraball, Extra_Life_Blob, Extra_Start, Extra_Hit, Misc_RopeTears, Misc_Ventilator, Music_thunder` | звуковые темы (assets/sounds) |
| menu sounds | `Sounds/Menu_click, Menu_load` | звуковые темы (assets/sounds) |
| ball hit and roll sounds | `Sounds/Hit_<ball>_<floor>, Roll_<ball>_<floor>` | звуковые темы (assets/sounds) |
| music | `Sounds/Music_Atmo_1..3, Music_Theme_<n>_1..3, Music_Final, Music_LastFinal, Music_EndCheckpoint` | свои: в *system* девять пьес, Music/music.txt раскладывает их по пяти темам; в *neuro* пятнадцать синтезированных фраз, по одной на часть, и своя музыка финиша; атмосферной части нет |
| original levels | `3D Entities/Level/Level_NN.NMO` | в планах |

Своё появляется по этапам: интерфейс (шрифты, кнопки, окно очков, шарики жизней, тексты, сцена меню), затем шары,
небо, частицы и звуки (из открытых библиотек), музыка в духе маджонга с китайскими мотивами, затем свои модули и
уровни.

Звуки и музыка приходят **звуковыми темами**, каждая — папка (`Sounds/` и `Music/` по именам, которые просит игра,
Ogg Vorbis или WAV, таблица `music.txt` для тем уровней, `theme.txt` с названием на четырёх языках, автором,
лицензией и `fallback`, `none` — без другой темы). С игрой поставляются две, в `assets/sounds/`: *system* (записи под свободными лицензиями и
музыка, выбранная владельцем, их делает `assets/sounds/system/prepare.py`, CC0 или CC-BY, авторы и изменения в её
`CREDITS.md` и на экране Credits) и *neuro* (эффекты и музыка уровней, синтезированные для игры, GPL-3.0-only;
музыка уровней — короткие щипковые фразы в духе гучжэна, своя для каждой из трёх частей пяти тем; в ней есть всё, что игра
запрашивает, и она ничего не берёт из других тем, `fallback none`). Свои темы игрок кладёт в `sounds/<имя>/` папки данных; среди них *original* делает toolbox из
копии игрока, она никогда не поставляется (пока её нет, выбор её играет звуки копии). Строка *Sounds* на странице
Extended выбирает тему (`sound-theme ИМЯ`); пока игрок не выбрал другую, и всегда в классическом режиме, играет
*neuro* (decisions.md, «Сгенерированная звуковая тема — по умолчанию»); сделанный выбор сохраняется. Звук, которого нет ни в теме, ни в
её запасной, берётся из копии (собственные звуки уровня); музыка — никогда. Папка темы игрока недоверенная: только
простые имена, никаких ссылок, ограниченное число тем, звуки декодируются через ограничения звукового слоя. Музыка
уровня декодируется заранее в отдельном потоке.
