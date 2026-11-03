# How close the own interface comes to the original

Made by `metric.py` (its docstring says how a score is made: 40 % Lab colour histogram overlap, 30 %
SSIM of the heavily blurred lightness, 30 % SSIM of the edge maps; 0 to 100). *Before* is the previous
version (v2: the menu scene's textures drawn from noise; v1, the first own interface, scored 51.7 for the
elements and 34 to 63 for the menu screens' regions), *now* this one (v3: its concrete, brick, wood and
iron from CC0 photo textures, graded to the measured palette and grain). Screens are 1280x720
renders of the same moment (`DRAHTSEILAKT_SHOT_MENU`, the menu scene `DRAHTSEILAKT_SHOT_MENU_MS` into
its round), English, the original's from the player's copy. The original's pictures never enter the
repository; only these numbers do.

## Elements of the interface textures

| element | before | now | colour | shape | edges |
|---|---:|---:|---:|---:|---:|
| main button, chosen | 91.3 | **91.3** | 0.99 | 0.932 | 0.79 |
| main button | 90.9 | **90.9** | 0.993 | 0.922 | 0.786 |
| field, chosen | 89.6 | **89.6** | 0.991 | 0.903 | 0.761 |
| field | 91.7 | **91.7** | 0.994 | 0.917 | 0.812 |
| level button, chosen | 88.4 | **88.4** | 0.979 | 0.873 | 0.77 |
| level button | 90.2 | **90.2** | 0.98 | 0.877 | 0.821 |
| arrow left | 73.0 | **73.0** | 0.968 | 0.593 | 0.548 |
| arrow right | 73.5 | **73.5** | 0.968 | 0.61 | 0.547 |
| key row, chosen | 77.2 | **77.2** | 0.974 | 0.716 | 0.559 |
| key row | 77.8 | **77.8** | 0.977 | 0.738 | 0.554 |
| back button, chosen | 79.3 | **79.3** | 0.992 | 0.774 | 0.548 |
| back button | 81.4 | **81.4** | 0.992 | 0.81 | 0.583 |
| yes / no | 88.1 | **88.1** | 0.992 | 0.82 | 0.795 |
| highscore place | 82.8 | **82.8** | 0.974 | 0.843 | 0.619 |
| level not open | 91.6 | **91.6** | 0.977 | 0.899 | 0.853 |
| life ball | 66.8 | **66.8** | 0.933 | 0.592 | 0.392 |
| lives end piece | 69.2 | **69.2** | 0.99 | 0.467 | 0.521 |
| lives frame | 89.0 | **89.0** | 0.998 | 0.839 | 0.799 |
| points box | 81.7 | **81.7** | 0.976 | 0.745 | 0.678 |
| end screen bar | 78.7 | **78.7** | 0.941 | 0.823 | 0.548 |
| *mean* | 82.6 | **82.6** | | | |

## Screens

| screen | region | before | now |
|---|---|---:|---:|
| credits | whole | 77.1 | **76.3** |
| credits | menu column | 78.4 | **76.6** |
| credits | scene left | 68.1 | **68.1** |
| credits | scene right | 75.0 | **75.0** |
| end | whole | 98.1 | **98.1** |
| end | end panel | 93.3 | **93.3** |
| end | points box | 93.5 | **93.5** |
| end | lives | 96.6 | **96.6** |
| highscore | whole | 75.9 | **76.1** |
| highscore | menu column | 84.1 | **84.0** |
| highscore | scene left | 71.8 | **71.8** |
| highscore | scene right | 64.1 | **64.1** |
| hud | whole | 99.5 | **99.5** |
| hud | points box | 94.1 | **94.1** |
| hud | lives | 91.9 | **91.9** |
| levels | whole | 71.9 | **72.0** |
| levels | menu column | 79.4 | **79.5** |
| levels | scene left | 65.0 | **65.1** |
| levels | scene right | 61.7 | **61.4** |
| main_0 | whole | 76.5 | **76.8** |
| main_0 | menu column | 84.7 | **84.8** |
| main_0 | scene left | 63.8 | **63.8** |
| main_0 | scene right | 71.3 | **72.2** |
| main_15000 | whole | 77.4 | **77.3** |
| main_15000 | menu column | 81.7 | **81.6** |
| main_15000 | scene left | 69.6 | **69.9** |
| main_15000 | scene right | 70.6 | **70.6** |
| main_30000 | whole | 77.0 | **77.3** |
| main_30000 | menu column | 83.4 | **83.5** |
| main_30000 | scene left | 76.7 | **76.6** |
| main_30000 | scene right | 64.1 | **64.5** |
| options | whole | 75.5 | **75.5** |
| options | menu column | 83.8 | **83.9** |
| options | scene left | 65.3 | **65.6** |
| options | scene right | 64.5 | **64.7** |
| pause | whole | 98.3 | **98.3** |
| pause | menu column | 97.1 | **97.1** |
| pause | scene left | 99.2 | **99.2** |
| pause | scene right | 99.1 | **99.1** |

## Yardstick: the original against itself

The original's main menu at one moment of its camera's round against another moment (the same scene
and buttons, seen from elsewhere): what a score means for a scene that is the same kind of place.

| screens | whole | menu column | scene left | scene right |
|---|---:|---:|---:|---:|
| main_0 / main_15000 | 66.5 | 79.9 | 61.9 | 56.9 |
| main_0 / main_30000 | 68.8 | 82.2 | 59.2 | 58.1 |
| main_15000 / main_30000 | 68.7 | 84.5 | 64.0 | 56.6 |
