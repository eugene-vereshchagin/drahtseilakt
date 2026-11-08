#!/usr/bin/env python3
# The level statistics report (in Russian, for the owner): joins the measurements of `drahtseilakt --level-stats DIR`
# (DIR/level-stats-auto.csv) with the hand annotations (hints/level-stats-manual.csv), works out a difficulty index
# per level and the targets for our own levels, and writes level-stats.md, level-stats.csv and level-stats.png.
#
#   scripts/level-stats.py AUTO_CSV MANUAL_CSV HINTS_DIR OUT_DIR
#   scripts/level-stats.py --own OWN_CSV OWN_MANUAL_CSV LEVEL_STATS_CSV   (one of our levels against its target)
#
# A level without a finished run (12, 13) is measured along the test player's way through its plan: that way is
# shorter than a real run (no detours, no going back), so its length is estimated from the walkthrough's time at the
# rolling rate (path per second of the walkthrough) of the late levels' runs, the shares of the kinds of track kept
# from the plan's way; what only a run shows (turns, flights, times) is estimated too; such numbers are marked.
import csv, math, os, statistics, sys

AUTO, MANUAL, HINTS, OUT = (sys.argv[1:5] + [""] * 4)[:4]

CLASSES = ["corridor", "wide", "trough", "posts", "curbs", "open", "narrow", "rails", "monorail", "siderail",
           "module", "air"]
RU = {
    "corridor": "коридор (стенки/перила)", "wide": "широкий пол", "trough": "жёлоб", "posts": "столбики",
    "curbs": "бортики (обычная трасса)", "open": "открытый край", "narrow": "узкая без перил (<6)",
    "rails": "два рельса", "monorail": "монорельс", "siderail": "рельсы вбок/вертикально",
    "module": "на механизмах", "air": "в воздухе",
}
# how hard a unit of each kind of track is to roll along (the owner's order: corridor easiest, rails hardest)
WEIGHT = {"corridor": 0.3, "wide": 0.4, "trough": 0.6, "curbs": 0.8, "posts": 1.0, "open": 1.2, "module": 1.5,
          "air": 1.5, "narrow": 2.0, "rails": 2.0, "siderail": 3.0, "monorail": 3.0}
# The track's share of the index: the owner's decision (2026-11-09) is that the track is about 20% of the index
# (it was about 47% of the originals' sum, 100 of 215 a level on average); the weights above are scaled by this, so
# the originals' average is 29 of the track against 115 of the rest. The other parts are as they were.
TRACK_SCALE = 0.29
GROUPS = [("лёгкая (коридор, широкий, жёлоб)", ["corridor", "wide", "trough"]),
          ("с бортиками/столбиками", ["curbs", "posts"]),
          ("открытый край / узкая", ["open", "narrow"]),
          ("рельсы (все)", ["rails", "monorail", "siderail"]),
          ("механизмы и воздух", ["module", "air"])]


def num(v):
    try:
        return float(v)
    except (TypeError, ValueError):
        return 0.0


# the difficulty index: five parts, added up (d: a level's measured row, m: its hand annotations)
def index_of(d, m):
    track = TRACK_SCALE * sum(d.get(f"len_{c}", 0.0) * w for c, w in WEIGHT.items()) / 100.0
    puzzles = (1.0 * d.get("loose_moved", 0) + 2.0 * (d.get("m01_pushers", 0) + d.get("m34_boxstone", 0)
               + d.get("m25_drawbridges", 0) + d.get("m37_targets", 0) + d.get("m03_lifts", 0))
               + 1.0 * (d.get("m30_seesaws", 0) + d.get("m29_plates", 0) + d.get("m41_tipplates", 0)
               + d.get("m19_flaps", 0)))
    if d.get("estimated"):
        # (no run: the loose things moved are not known; as many as the boxes and loose balls the plan names)
        puzzles += 0.5 * (d.get("boxes", 0) + d.get("loose_stone_balls", 0) + d.get("loose_wood_balls", 0))
    tricks = (1.0 * d.get("transforms", 0) + 1.0 * d.get("fan_flights", 0) + 3.0 * num(m["centrifugal"])
              + 3.0 * num(m["balls_as_keys"]) + 2.0 * num(m["balls_ahead"]) + 2.0 * num(m["timing"]))
    navigation = 3.0 * num(m["hidden_routes"]) + 2.0 * num(m["backtracks"]) + 4.0 * num(m["maze"])
    hazards = (1.0 * (d.get("m26_sacks", 0) + d.get("m08_swings", 0) + d.get("m17_poles", 0))
               + 0.5 * d.get("sharp_turns_at_edge", 0) + 0.3 * d.get("humps", 0))
    return {"track": track, "puzzles": puzzles, "tricks": tricks, "navigation": navigation, "hazards": hazards,
            "total": track + puzzles + tricks + navigation + hazards}


def own_report(own_csv, manual_csv, stats_csv):
    """One of our levels against its target: `--level-stats DIR --open LEVEL --replay RUN` (DIR/level-stats-own.csv),
    its hand annotations (one line, the columns of hints/level-stats-manual.csv, `level` its order) and the report's
    level-stats.csv (the target for that order: the index split as the original's of the same number)."""
    rows = list(csv.DictReader(open(own_csv)))
    whole = {k: num(v) for k, v in rows[0].items() if k not in ("level", "source", "finished_runs")}
    m = next(csv.DictReader(l for l in open(manual_csv) if not l.startswith("#")))
    n = int(m["level"])
    orig = next(r for r in csv.DictReader(open(stats_csv)) if int(r["level"]) == n and int(r["sector"]) == 0)
    i = index_of(whole, m)
    target = num(orig["target_index"])
    print(f"{rows[0]['level']} against our level {n}'s target (2x the original {n}'s trend)")
    t = whole["time_s"]
    print(f"  path {whole['path_len']:.0f} (target {num(orig['target_length']):.0f}), time {int(t // 60)}:{t % 60:04.1f}, "
          f"sectors {len(rows) - 1}")
    for k in ("track", "puzzles", "tricks", "navigation", "hazards", "total"):
        share = num(orig[f"index_{k}"]) / max(num(orig["index_total"]), 1e-9)
        print(f"  {k:10s} {i[k]:6.1f}   target {target * share:6.1f}")
    print("  track by class: " + ", ".join(f"{c} {whole.get(f'len_{c}', 0):.0f}" for c in CLASSES))
    for k in ("extra_points", "extra_lives", "taken_extra_points", "taken_extra_lives", "transforms", "fan_flights", "loose_moved",
              "m01_pushers", "m34_boxstone", "m30_seesaws", "m29_plates", "sharp_turns", "sharp_turns_at_edge", "ramps_up",
              "ramps_down", "humps",
              "standing_share", "speed_p10", "speed_p50", "speed_p90"):
        if k in whole:
            print(f"  {k} {whole[k]:g}")
    for r in rows[1:]:
        print(f"  sector {r['sector']}: path {num(r['path_len']):.0f}, time {num(r['time_s']):.0f} s")


if len(sys.argv) > 1 and sys.argv[1] == "--own":
    own_report(*sys.argv[2:5])
    sys.exit(0)

auto = list(csv.DictReader(open(AUTO)))
manual = {int(r["level"]): r for r in csv.DictReader(l for l in open(MANUAL) if not l.startswith("#"))}
levels = sorted({int(r["level"]) for r in auto})


def rows(level, kind):
    return {int(r["sector"]): r for r in auto if int(r["level"]) == level and r["source"].startswith(kind)}


# the run/plan length ratio, from the levels that have both
ratios = []
for n in levels:
    run, plan = rows(n, "run"), rows(n, "plan")
    if run and plan and num(plan[0]["path_len"]) > 0:
        ratios.append(num(run[0]["path_len"]) / num(plan[0]["path_len"]))
SCALE = statistics.median(ratios) if ratios else 1.0

# the path the runs of the late levels (9-11) roll a second of the walkthrough's time
manual_time = {int(r["level"]): num(r["expert_time_s"]) for r in manual.values()}
RATE = statistics.median(num(rows(n, "run")[0]["path_len"]) / manual_time[n]
                         for n in levels if 9 <= n <= 11 and rows(n, "run"))

# densities from the runs of the late levels (9-11) for what a plan's way cannot show
late = [rows(n, "run")[0] for n in levels if 9 <= n <= 11 and rows(n, "run")]
late_len = sum(num(r["path_len"]) for r in late) or 1.0
DENSITY = {k: sum(num(r.get(k)) for r in late) / late_len for k in
           ["sharp_turns", "sharp_turns_at_edge", "bends", "ramps_up", "ramps_down", "humps", "jumps", "air_time_s"]}

# the autopilot's pace against the walkthrough's
auto_vs_expert = statistics.median(
    num(rows(n, "run")[0]["time_s"]) / num(manual[n]["expert_time_s"]) for n in levels if rows(n, "run"))


def plan_counts(n):
    """Transformers and fans named in the plan (the ones the way takes)."""
    path = os.path.join(HINTS, f"Level_{n:02}.plan")
    t = f = 0
    if os.path.exists(path):
        for line in open(path):
            name = line.split("#")[0].strip()
            t += name.startswith("P_Trafo")
            f += name.startswith("P_Modul_18")
    return t, f


# one record a level and sector, from its run, else its plan's way (scaled, estimated)
data = {"scales": {}}
for n in levels:
    run = rows(n, "run")
    src = run or rows(n, "plan")
    estimated = not run
    for s, r in src.items():
        d = {k: num(v) for k, v in r.items() if k not in ("level", "source", "finished_runs", "sector")}
        d["estimated"] = estimated
        if estimated:
            plan_len = num(src[0]["path_len"]) or 1.0
            scale = RATE * manual_time[n] / plan_len
            data.setdefault("scales", {})[n] = (scale, plan_len)
            for k in list(d):
                if k == "path_len" or k.startswith("len_"):
                    d[k] *= scale
            for k, v in DENSITY.items():
                d[k] = v * d["path_len"]
            if s == 0:
                t, f = plan_counts(n)
                d["transforms"], d["fan_flights"] = t, f
            d["time_s"] = 0.0
        data[(n, s)] = d
    data[(n, 0)]["sectors"] = max(s for s in src)
    data[(n, 0)]["source"] = "прогон автопилота" if run else "путь по плану (оценка)"
    data[(n, 0)]["runs"] = next(iter(src.values()))["finished_runs"]


def L(d, c):
    return d.get(f"len_{c}", 0.0)


def index(n):
    return index_of(data[(n, 0)], manual[n])


idx = {n: index(n) for n in levels}
expert = {n: num(manual[n]["expert_time_s"]) for n in levels}
# the trend: a straight line through log(index) (difficulty grows by a steady factor a level)
xs = levels
ys = [math.log(idx[n]["total"]) for n in levels]
mx, my = statistics.mean(xs), statistics.mean(ys)
b = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sum((x - mx) ** 2 for x in xs)
a = my - b * mx
trend = {n: math.exp(a + b * n) for n in levels}
growth = math.exp(b) - 1
tl = [math.log(expert[n]) for n in levels]
mt = statistics.mean(tl)
bt = sum((x - mx) * (y - mt) for x, y in zip(xs, tl)) / sum((x - mx) ** 2 for x in xs)
time_trend = {n: math.exp(mt + bt * (n - mx)) for n in levels}

# ---- the CSV: every number, by level and sector, with the index on the whole-level rows
keys = []
for d in data.values():
    for k in d:
        if k not in keys and k not in ("estimated", "source", "runs", "sectors"):
            keys.append(k)
man_keys = ["hidden_routes", "maze", "backtracks", "centrifugal", "balls_as_keys", "balls_ahead", "timing",
            "expert_time_s", "confidence"]
idx_keys = ["track", "puzzles", "tricks", "navigation", "hazards", "total"]
with open(os.path.join(OUT, "level-stats.csv"), "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["level", "sector", "source", "estimated"] + keys + [f"manual_{k}" for k in man_keys]
               + [f"index_{k}" for k in idx_keys] + ["index_trend", "target_index", "target_length",
                                                     "target_time_s"])
    for (n, s), d in sorted((k, v) for k, v in data.items() if k != "scales"):
        row = [n, s, data[(n, 0)]["source"], int(d["estimated"])] + [round(d.get(k, 0.0), 2) for k in keys]
        if s == 0:
            row += [manual[n][k] for k in man_keys] + [round(idx[n][k], 1) for k in idx_keys]
            row += [round(trend[n], 1), round(2 * trend[n], 1), round(2 * d["path_len"]),
                    round(2 * expert[n])]
        else:
            row += [""] * (len(man_keys) + len(idx_keys) + 4)
        w.writerow(row)

# ---- the chart: the index of the originals, its trend, the targets; and what the track is made of
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

BLUE, ORANGE, GRAY = "#2a78d6", "#eb6834", "#8a8984"
STACK = ["#2a78d6", "#1baf7a", "#eda100", "#eb6834", "#e87ba4"]
INK, MUTED = "#1f1f1e", "#6b6a64"
plt.rcParams.update({"font.size": 10, "axes.edgecolor": "#c9c8c1", "axes.labelcolor": MUTED,
                     "xtick.color": MUTED, "ytick.color": MUTED, "axes.spines.top": False,
                     "axes.spines.right": False})
fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(10, 9), height_ratios=[3, 2], constrained_layout=True)
ax1.grid(axis="y", color="#ecebe6", linewidth=1)
ax1.plot(levels, [idx[n]["total"] for n in levels], color=BLUE, linewidth=2, marker="o", markersize=7,
         markeredgecolor="white", markeredgewidth=1.5, label="оригинал: индекс сложности", zorder=3)
ax1.plot(levels, [trend[n] for n in levels], color=GRAY, linewidth=1.5, linestyle="--",
         label=f"тренд оригинала (+{growth * 100:.0f}% за уровень)", zorder=2)
ax1.plot(levels, [2 * trend[n] for n in levels], color=ORANGE, linewidth=2, marker="s", markersize=7,
         markeredgecolor="white", markeredgewidth=1.5, label="цель для наших уровней (2× тренд)", zorder=3)
for n in levels:
    est = data[(n, 0)]["estimated"]
    ax1.annotate(f"{idx[n]['total']:.0f}" + ("*" if est else ""), (n, idx[n]["total"]),
                 textcoords="offset points", xytext=(0, -16), ha="center", fontsize=8, color=INK)
ax1.annotate(f"{2 * trend[13]:.0f}", (13, 2 * trend[13]), textcoords="offset points", xytext=(10, 0),
             va="center", fontsize=8, color=INK)
ax1.set_xticks(levels)
ax1.set_xlabel("уровень")
ax1.set_ylabel("индекс сложности (очки)")
ax1.set_ylim(bottom=0)
ax1.set_title("Сложность уровней Ballance и цель для наших (* - уровень без прогона, оценка)", loc="left",
              color=INK, fontsize=12)
ax1.legend(frameon=False, loc="upper left")
bottom = [0.0] * len(levels)
for (name, cs), color in zip(GROUPS, STACK):
    vals = [sum(L(data[(n, 0)], c) for c in cs) / max(data[(n, 0)]["path_len"], 1) * 100 for n in levels]
    ax2.bar(levels, vals, bottom=bottom, color=color, width=0.7, label=name, edgecolor="white", linewidth=1)
    bottom = [b_ + v for b_, v in zip(bottom, vals)]
ax2.set_xticks(levels)
ax2.set_ylim(0, 100)
ax2.set_ylabel("% длины пути")
ax2.set_xlabel("уровень")
ax2.set_title("Из чего состоит путь", loc="left", color=INK, fontsize=12)
ax2.legend(frameon=False, ncols=3, loc="upper center", bbox_to_anchor=(0.5, -0.18), fontsize=9)
fig.savefig(os.path.join(OUT, "level-stats.png"), dpi=120)

# ---- the report
o = []
p = o.append


def f0(v):
    return f"{v:.0f}"


def mmss(s):
    return f"{int(s // 60)}:{int(s % 60):02d}" if s else "-"


p("# Статистика 13 оригинальных уровней Ballance")
p("")
p("**Изменение от 2026-11-09 (решение владельца).** Трасса теперь даёт около 20% индекса, а не около 47%: веса типов "
  "трассы умножены на 0,29 (`TRACK_SCALE` в `scripts/level-stats.py`); остальные части индекса не менялись. У "
  "оригиналов в среднем трасса 28 очков из 138 (было 100 из 215). Индексы и цели для наших уровней ниже пересчитаны "
  "(цель = в 2 раза больше тренда оригинала, как раньше). Замечание: за время с прошлого отчёта у уровня 12 появился "
  "законченный прогон (`Level_12.replay`), поэтому он теперь измерен по прогону, а не оценён по плану, и его "
  "остальные части индекса тоже другие (в т.ч. тренд и цели).")
p("")
p("Замеры для проектирования наших 13 уровней (каждый примерно в 2 раза длиннее своего оригинала, сложность растёт "
  "так же, число секторов то же). Ничего ещё не спроектировано - только измерено.")
p("")
p("**Как читать.** Числа без пометки посчитаны программой автоматически (`drahtseilakt --level-stats`, ветка "
  "`level-stats`): по типам объектов уровня, геометрии полов и рельсов и по прогону автопилота от старта до финиша. "
  "Пометка **(р)** - ручная оценка по маршрутам, подсказкам и раскадровке прохождения "
  "(`hints/level-stats-manual.csv`, там же описание каждой колонки). Пометка **\\*** - у уровня нет законченного "
  "прогона (12 и 13): путь взят из плана автопилота, длины умножены на поправку, остальное оценено (см. в конце).")
p("")
p("Единица длины - игровая единица; шар имеет диаметр 4. Длины - вдоль реального пути шара (без попыток, "
  "закончившихся потерей шара, и без времени в трансформаторе).")
p("")
p("## Сводка по уровням")
p("")
p("| Ур. | Секторов | Путь, ед. | Время автопилота | Время прохождения (видео) (р) | Бонус уровня | "
  "Очки времени (шт.) | Доп. жизни (шт.) | Превращений | Полётов на вентиляторах | Индекс сложности |")
p("|---|---|---|---|---|---|---|---|---|---|---|")
for n in levels:
    d = data[(n, 0)]
    st = "\\*" if d["estimated"] else ""
    autot = mmss(d["time_s"]) if d["time_s"] else f"~{mmss(expert[n] * auto_vs_expert)}\\*"
    p(f"| {n} | {d['sectors']} | {f0(d['path_len'])}{st} | {autot} | {mmss(expert[n])} | "
      f"{f0(d.get('level_bonus', 0))} | {f0(d.get('extra_points', 0))} | {f0(d.get('extra_lives', 0))} | "
      f"{f0(d.get('transforms', 0))}{st} | {f0(d.get('fan_flights', 0))}{st} | **{idx[n]['total']:.0f}**{st} |")
p("")
p("Время автопилота - его чистый прогон (едет осторожно, без риска); время прохождения - глава уровня в видео "
  "прохождения (опытный игрок, с финальной заставкой ~10-20 с; уровень 13 - отдельное видео). Отношение "
  f"автопилот/видео в среднем {auto_vs_expert:.2f}. «Очки времени» и «доп. жизни» - сколько их разложено на уровне "
  "(собрать можно все); сколько собрал автопилот - в CSV (`taken_*`).")
p("")
p("## Трасса: длина по типам (от лёгкой к трудной)")
p("")
p("Каждая точка пути (через 0,5 ед.) классифицируется по поперечному профилю: что по бокам (стенка выше 1,6 или "
  "склон выше 2,5 - коридор; пологий склон 1,25-2,5 - жёлоб; низкий бортик - бортики; обрыв - открытый край), "
  "ширина пола, и на чём шар: пол, два рельса, один рельс (монорельс), рельсы сбоку/сверху (шар держится "
  "скоростью или опирается на верхний рельс), подвижная часть механизма, воздух. «Столбики» - бортик, который "
  "перемежается с обрывом (определяются ненадёжно). Обычная трасса Ballance - пол шириной ~7,5 с низким скошенным "
  "бортиком ~0,75: она попадает в «бортики».")
p("")
p("| Ур. | " + " | ".join(RU[c] for c in CLASSES) + " | ширина ровного пола до бортика (медиана) | расстояние между рельсами |")
p("|---|" + "---|" * (len(CLASSES) + 2))
for n in levels:
    d = data[(n, 0)]
    st = "\\*" if d["estimated"] else ""
    p(f"| {n}{st} | " + " | ".join(f0(L(d, c)) for c in CLASSES)
      + f" | {d.get('floor_width_median', 0):.1f} | {d.get('rail_spread_median', 0):.1f} |")
p("")
p("Доли групп по уровням:")
p("")
p("| Ур. | " + " | ".join(g for g, _ in GROUPS) + " |")
p("|---|" + "---|" * len(GROUPS))
for n in levels:
    d = data[(n, 0)]
    p(f"| {n} | " + " | ".join(f"{sum(L(d, c) for c in cs) / max(d['path_len'], 1) * 100:.0f}%"
                               for _, cs in GROUPS) + " |")
p("")
p("## Головоломки, техники, опасности, неочевидные пути")
p("")
p("| Ур. | Ящики/шары сдвинуты | Толкатели | Ящик под камнем | Разводные мосты | Мишени | Подъёмники | Качели-доски | "
  "Висячие плиты | Откидные плиты | Неочевидные места (р) | Лабиринт (р) | Возвраты (р) | Центробежные (р) | "
  "Шары как ключ (р) | Шар впереди (р) | Ждать момент (р) | Мешки | Качели/ящики | Шесты | Резкие повороты у края | "
  "Горки/горбы |")
p("|---|" + "---|" * 21)
for n in levels:
    d, m = data[(n, 0)], manual[n]
    st = "\\*" if d["estimated"] else ""
    moved = "?" if d["estimated"] else f0(d.get("loose_moved", 0))
    p(f"| {n} | {moved} | {f0(d.get('m01_pushers', 0))} | {f0(d.get('m34_boxstone', 0))} | "
      f"{f0(d.get('m25_drawbridges', 0))} | {f0(d.get('m37_targets', 0))} | {f0(d.get('m03_lifts', 0))} | "
      f"{f0(d.get('m30_seesaws', 0))} | {f0(d.get('m29_plates', 0))} | {f0(d.get('m41_tipplates', 0) + d.get('m19_flaps', 0))} | "
      f"{m['hidden_routes']} | {m['maze']} | {m['backtracks']} | {m['centrifugal']} | {m['balls_as_keys']} | "
      f"{m['balls_ahead']} | {m['timing']} | {f0(d.get('m26_sacks', 0))} | {f0(d.get('m08_swings', 0))} | "
      f"{f0(d.get('m17_poles', 0))} | {f0(d.get('sharp_turns_at_edge', 0))}{st} | "
      f"{f0(d.get('ramps_up', 0) + d.get('ramps_down', 0))}/{f0(d.get('humps', 0))}{st} |")
p("")
p("Механизмы посчитаны по объектам уровня (все, что стоят на уровне). «Ящики/шары сдвинуты» - сколько свободных "
  "ящиков и шаров автопилот сдвинул больше чем на 2 ед. за прогон. «Горки/горбы» - участки уклоном больше 0,2 "
  "(вверх+вниз) / подъём, сразу переходящий в спуск. «Резкие повороты у края» - поворот больше 60° на 8 ед. пути "
  "там, где край открыт или шар на рельсах.")
p("")
p("Ручные заметки (р), уверенность и источник:")
p("")
p("| Ур. | Уверенность | Источник | Что там |")
p("|---|---|---|---|")
for n in levels:
    m = manual[n]
    conf = {"high": "высокая", "medium": "средняя", "low": "низкая"}[m["confidence"]]
    p(f"| {n} | {conf} | {m['source']} | {m['notes_ru']} |")
p("")
p("## Индекс сложности")
p("")
p("Индекс - сумма пяти частей в «очках». Веса подобраны так, чтобы трасса (управление шаром) давала около "
  "20% (с 2026-11-09; раньше половину), остальное - головоломки, техники, поиск пути и опасности:")
p("")
p("- **Трасса** = сумма по типам (длина / 100 × вес). Вес (до умножения на 0,29): коридор 0,3, широкий пол "
  "0,4, жёлоб 0,6, бортики 0,8, столбики 1,0, открытый край 1,2, механизмы и воздух 1,5, узкая без перил 2, два "
  "рельса 2, монорельс и рельсы вбок 3. То есть 100 ед. обычной трассы = 0,23 очка, 100 ед. монорельса = 0,87 очка.")
p("- **Головоломки** = 1 за каждый сдвинутый ящик/шар + 2 за толкатель, ящик под камнем, разводной мост, мишень, "
  "подъёмник + 1 за качели-доску, висячие и откидные плиты.")
p("- **Техники** = 1 за превращение + 1 за полёт на вентиляторе + 3 за центробежный участок (р) + 3 за шар-ключ (р) "
  "+ 2 за шар впереди (р) + 2 за ожидание момента (р).")
p("- **Поиск пути** (р) = 3 за неочевидное место + 2 за возврат + 4 за степень лабиринта.")
p("- **Опасности** = 1 за мешок, качели, шест + 0,5 за резкий поворот у края + 0,3 за горб.")
p("")
p("Индекс растёт с длиной уровня (длинный уровень - больше мест, где можно ошибиться). «Плотность» - индекс на "
  "минуту прохождения - показывает, насколько трудна каждая минута.")
p("")
p("| Ур. | Трасса | Головоломки | Техники | Поиск пути (р) | Опасности | **Итого** | Тренд | Плотность (очков/мин) |")
p("|---|---|---|---|---|---|---|---|---|")
for n in levels:
    i = idx[n]
    st = "\\*" if data[(n, 0)]["estimated"] else ""
    p(f"| {n}{st} | {i['track']:.0f} | {i['puzzles']:.0f} | {i['tricks']:.0f} | {i['navigation']:.0f} | "
      f"{i['hazards']:.0f} | **{i['total']:.0f}** | {trend[n]:.0f} | {i['total'] / (expert[n] / 60):.1f} |")
p("")
p(f"Тренд - экспонента через все 13 точек: сложность растёт в среднем на **{growth * 100:.0f}% за уровень** "
  f"(от {trend[1]:.0f} у 1-го до {trend[13]:.0f} у 13-го, в {trend[13] / trend[1]:.1f} раза). Сами уровни "
  "колеблются вокруг тренда: заметно выше (больше чем на 10%) - "
  + (", ".join(str(n) for n in levels if idx[n]["total"] > 1.1 * trend[n]) or "нет")
  + "; заметно ниже - " + (", ".join(str(n) for n in levels if idx[n]["total"] < 0.9 * trend[n]) or "нет")
  + f". Самый трудный по индексу - {max(levels, key=lambda n: idx[n]['total'])}-й. Время прохождения растёт "
  f"примерно на {(math.exp(bt) - 1) * 100:.0f}% за уровень.")
p("")
p("![Кривая сложности](level-stats.png)")
p("")
p("## Цели для наших уровней")
p("")
p("Правило: длина и время - в 2 раза больше оригинала того же номера; число секторов то же (секторы длиннее); "
  "индекс - в 2 раза больше тренда оригинала (тренд, а не сам уровень, чтобы рост был ровным, без провала на 2-м "
  "уровне). При удвоенной длине плотность (сложность минуты) остаётся как у оригинала - рост между уровнями "
  "тот же. Разбивка по частям - в той же пропорции, что у оригинала (головоломки и секторы крупнее, а не больше "
  "типов).")
p("")
p("| Ур. | Секторов | Путь, ед. | На сектор, ед. | Время опытного игрока | Индекс | из них трасса | головоломки | "
  "техники | поиск пути | опасности | Очки времени / жизни (×2) |")
p("|---|---|---|---|---|---|---|---|---|---|---|---|")
for n in levels:
    d, i = data[(n, 0)], idx[n]
    t = 2 * trend[n]
    share = {k: i[k] / i["total"] for k in ["track", "puzzles", "tricks", "navigation", "hazards"]}
    p(f"| {n} | {d['sectors']} | {2 * d['path_len']:.0f} | {2 * d['path_len'] / d['sectors']:.0f} | "
      f"{mmss(2 * expert[n])} | **{t:.0f}** | {t * share['track']:.0f} | {t * share['puzzles']:.0f} | "
      f"{t * share['tricks']:.0f} | {t * share['navigation']:.0f} | {t * share['hazards']:.0f} | "
      f"{2 * d.get('extra_points', 0):.0f} / {2 * d.get('extra_lives', 0):.0f} |")
p("")
p("## По секторам")
p("")
p("Для каждого уровня: длина пути в секторе, время автопилота, из чего трасса (ед.), что разложено и что "
  "встречается. В CSV - все колонки.")
for n in levels:
    d0 = data[(n, 0)]
    st = " (оценка по плану\\*)" if d0["estimated"] else ""
    p("")
    p(f"### Уровень {n}{st}")
    p("")
    p("| Сектор | Путь | Время | Бортики/коридор/широкий | Открытый/узкий | Рельсы (2/1/вбок) | Механизмы/воздух | "
      "Очки времени | Жизни | Трансформаторы (Д/К/Б) | Головоломки | Опасности |")
    p("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for s in range(1, d0["sectors"] + 1):
        d = data.get((n, s))
        if not d:
            continue
        easy = L(d, "curbs") + L(d, "posts") + L(d, "corridor") + L(d, "wide") + L(d, "trough")
        puzzles = []
        for k, ru in [("m01_pushers", "толк."), ("m34_boxstone", "ящ.под камнем"), ("m25_drawbridges", "мост"),
                      ("m37_targets", "мишень"), ("m03_lifts", "подъёмн."), ("m30_seesaws", "доска"),
                      ("m29_plates", "плиты"), ("m41_tipplates", "откидн."), ("m19_flaps", "люк"),
                      ("boxes", "ящики"), ("m18_fans", "вент.")]:
            if d.get(k, 0):
                puzzles.append(f"{ru} {d[k]:.0f}")
        hz = []
        for k, ru in [("m26_sacks", "мешки"), ("m08_swings", "качели"), ("m17_poles", "шесты"),
                      ("sharp_turns_at_edge", "резк.пов."), ("humps", "горбы")]:
            if d.get(k, 0) >= 0.5:
                hz.append(f"{ru} {d[k]:.0f}")
        p(f"| {s} | {d['path_len']:.0f} | {mmss(d.get('time_s', 0))} | {easy:.0f} | "
          f"{L(d, 'open') + L(d, 'narrow'):.0f} | {L(d, 'rails'):.0f}/{L(d, 'monorail'):.0f}/{L(d, 'siderail'):.0f} | "
          f"{L(d, 'module'):.0f}/{L(d, 'air'):.0f} | {d.get('extra_points', 0):.0f} | {d.get('extra_lives', 0):.0f} | "
          f"{d.get('trafo_wood', 0):.0f}/{d.get('trafo_stone', 0):.0f}/{d.get('trafo_paper', 0):.0f} | "
          f"{', '.join(puzzles) or '-'} | {', '.join(hz) or '-'} |")
p("")
p("## Что ручное и наименее надёжное")
p("")
p("- **Ручные (р):** неочевидные места, лабиринт, возвраты, центробежные участки, шары-ключи, шар впереди, "
  "ожидание момента, время прохождения из видео. Надёжнее всего для уровней 1, 2, 3, 6, 8, 11 (есть подробный "
  "маршрут и законченный прогон); средне - 4, 5, 10; **наименее надёжно - 7, 9, 12, 13** (только план и раскадровка).")
est = "; ".join(f"{n}: план {pl:.0f} ед. → оценка {pl * sc:.0f} ед." for n, (sc, pl) in sorted(data["scales"].items()))
p(f"- **Уровни 12 и 13** не пройдены автопилотом. Длина пути = время прохождения из видео × {RATE:.1f} ед./с "
  f"(сколько пути автопилота приходится на секунду видео у уровней 9-11); кратчайший путь по плану короче "
  f"(у уровней, где есть и то и другое, прогон в среднем в {SCALE:.2f} раза длиннее плана) - это нижняя граница ({est}). "
  "Доли типов трассы - с пути по плану; повороты, горбы - по плотности уровней 9-11; превращения и полёты - по "
  "числу трансформаторов и вентиляторов в плане (план называет не каждый вентилятор группы - полётов скорее больше); "
  "сдвинутые ящики не известны (в индексе - половина ящиков и шаров уровня).")
p("- **Автоматика, где возможны ошибки:** «столбики» (путаются с краями на развилках и поворотах); граница "
  "«бортики/жёлоб/коридор» зависит от порогов высоты; «механизмы» - шар рядом с любой частью модуля (в т.ч. над "
  "вентилятором); «воздух» включает прыжки и падения. Длины - по пути автопилота: он ездит аккуратно и иногда "
  "возвращается, поэтому путь длиннее, чем у опытного игрока.")
p("- Сырые точки пути с классом каждой (для проверки на карте): `samples/Level_NN-run.csv` и `-plan.csv` в папке "
  "замеров (`target/level-stats/` ветки).")
p("")
p("Как пересчитать: `drahtseilakt --data <Ballance> --level-stats DIR` (из корня репозитория, читает `hints/`), "
  "затем `scripts/level-stats.py DIR/level-stats-auto.csv hints/level-stats-manual.csv hints OUT`.")
open(os.path.join(OUT, "level-stats.md"), "w").write("\n".join(o) + "\n")
print("written", OUT)
