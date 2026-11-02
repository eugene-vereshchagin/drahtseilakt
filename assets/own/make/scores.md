# How close the own interface comes to the original

Made by `metric.py` (its docstring says how a score is made: 40 % Lab colour histogram overlap, 30 %
SSIM of the heavily blurred lightness, 30 % SSIM of the edge maps; 0 to 100). *Before* is the first own
interface (purple pill buttons, a flat picture behind the menus), *now* this one. Screens are 1280x720
renders of the same moment (`DRAHTSEILAKT_SHOT_MENU`, the menu scene `DRAHTSEILAKT_SHOT_MENU_MS` into
its round), English, the original's from the player's copy. The original's pictures never enter the
repository; only these numbers do.

## Elements of the interface textures

| element | before | now | colour | shape | edges |
|---|---:|---:|---:|---:|---:|
| main button, chosen | 47.2 | **91.3** | 0.99 | 0.932 | 0.79 |
| main button | 47.8 | **90.9** | 0.993 | 0.922 | 0.786 |
| field, chosen | 43.3 | **89.6** | 0.991 | 0.903 | 0.761 |
| field | 38.9 | **91.7** | 0.994 | 0.917 | 0.812 |
| level button, chosen | 55.5 | **88.4** | 0.979 | 0.873 | 0.77 |
| level button | 54.0 | **90.2** | 0.98 | 0.877 | 0.821 |
| arrow left | 45.5 | **73.0** | 0.968 | 0.593 | 0.548 |
| arrow right | 45.7 | **73.5** | 0.968 | 0.61 | 0.547 |
| key row, chosen | 47.5 | **77.2** | 0.974 | 0.716 | 0.559 |
| key row | 51.3 | **77.8** | 0.977 | 0.738 | 0.554 |
| back button, chosen | 50.4 | **79.3** | 0.992 | 0.774 | 0.548 |
| back button | 51.0 | **81.4** | 0.992 | 0.81 | 0.583 |
| yes / no | 46.8 | **88.1** | 0.992 | 0.82 | 0.795 |
| highscore place | 60.8 | **82.8** | 0.974 | 0.843 | 0.619 |
| level not open | 65.5 | **91.6** | 0.977 | 0.899 | 0.853 |
| life ball | 71.5 | **66.8** | 0.933 | 0.592 | 0.392 |
| lives end piece | 46.8 | **69.2** | 0.99 | 0.467 | 0.521 |
| lives frame | 58.2 | **89.0** | 0.998 | 0.839 | 0.799 |
| points box | 53.1 | **81.7** | 0.976 | 0.745 | 0.678 |
| end screen bar | 53.5 | **78.7** | 0.941 | 0.823 | 0.548 |
| *mean* | 51.7 | **82.6** | | | |

## Screens

| screen | region | before | now |
|---|---|---:|---:|
| credits | whole | 48.7 | **77.1** |
| credits | menu column | 60.0 | **78.4** |
| credits | scene left | 34.1 | **68.1** |
| credits | scene right | 46.4 | **75.0** |
| end | whole | 95.7 | **98.1** |
| end | end panel | 88.6 | **93.3** |
| end | points box | 80.4 | **93.5** |
| end | lives | 88.6 | **96.6** |
| highscore | whole | 49.1 | **75.9** |
| highscore | menu column | 63.9 | **84.1** |
| highscore | scene left | 35.4 | **71.8** |
| highscore | scene right | 47.6 | **64.1** |
| hud | whole | 98.7 | **99.5** |
| hud | points box | 85.1 | **94.1** |
| hud | lives | 86.7 | **91.9** |
| levels | whole | 49.4 | **71.9** |
| levels | menu column | 57.6 | **79.4** |
| levels | scene left | 43.4 | **65.0** |
| levels | scene right | 41.1 | **61.7** |
| main_0 | whole | 50.0 | **76.5** |
| main_0 | menu column | 58.6 | **84.7** |
| main_0 | scene left | 40.9 | **63.8** |
| main_0 | scene right | 47.4 | **71.3** |
| main_15000 | whole | 47.4 | **77.4** |
| main_15000 | menu column | 59.2 | **81.7** |
| main_15000 | scene left | 40.3 | **69.6** |
| main_15000 | scene right | 41.4 | **70.6** |
| main_30000 | whole | 48.1 | **77.0** |
| main_30000 | menu column | 59.8 | **83.4** |
| main_30000 | scene left | 40.8 | **76.7** |
| main_30000 | scene right | 41.4 | **64.1** |
| options | whole | 46.5 | **75.5** |
| options | menu column | 58.8 | **83.8** |
| options | scene left | 39.7 | **65.3** |
| options | scene right | 38.1 | **64.5** |
| pause | whole | 92.4 | **98.3** |
| pause | menu column | 81.1 | **97.1** |
| pause | scene left | 98.0 | **99.2** |
| pause | scene right | 98.0 | **99.1** |

## Yardstick: the original against itself

The original's main menu at one moment of its camera's round against another moment (the same scene
and buttons, seen from elsewhere): what a score means for a scene that is the same kind of place.

| screens | whole | menu column | scene left | scene right |
|---|---:|---:|---:|---:|
| main_0 / main_15000 | 66.5 | 79.9 | 61.9 | 56.9 |
| main_0 / main_30000 | 68.8 | 82.2 | 59.2 | 58.1 |
| main_15000 / main_30000 | 68.7 | 84.5 | 64.0 | 56.6 |
