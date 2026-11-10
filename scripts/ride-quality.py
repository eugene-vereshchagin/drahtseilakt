#!/usr/bin/env python3
# Driving quality of a replay: run headless with --trace 1, report standing share, edge share, speed spread,
# and stretches standing longer than 1 s (where, how long). usage: ride-quality.py LEVEL|LEVEL_DIR REPLAY
# (the game from this checkout's target/release, or $DRAHTSEILAKT)
import re, subprocess, sys, statistics, os
lvl, rep = sys.argv[1], sys.argv[2]
exe = os.environ.get('DRAHTSEILAKT', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'target', 'release', 'drahtseilakt'))
# a level's number (the original's), or a level folder (one of ours: --open)
which = ['--open', lvl] if os.path.isdir(lvl) else ['--level', lvl]
out = subprocess.run(['nice', '-n', '19', exe] + which + ['--replay', rep, '--headless', '--trace', '1'],
                     capture_output=True, text=True).stdout.splitlines()
pat = re.compile(r'^(\d+):(\d+\.\d+) ball \(\s*(\S+)\s+(\S+)\s+(\S+)\) speed\s+(\S+)')
ticks = []; trans = []; result = edges = ''
for l in out:
    m = pat.match(l)
    if m:
        t = int(m[1]) * 60 + float(m[2]); ticks.append((t, float(m[3]), float(m[4]), float(m[5]), float(m[6])))
    elif 'Transform(' in l:
        mm = re.match(r'^(\d+):(\d+\.\d+)', l); trans.append(int(mm[1]) * 60 + float(mm[2]))
    elif l.startswith('result:'): result = l
    elif l.startswith('edges:'): edges = l
sp = [x[4] for x in ticks]
held = lambda t: any(0 <= t - tt < 2.6 for tt in trans)
stand = [x for x in ticks if x[4] < 0.3]
stand_free = [x for x in stand if not held(x[0])]
print(result); print(edges)
print(f'ticks {len(ticks)}; standing (<0.3) {100*len(stand)/len(ticks):.1f}%, '
      f'without transformer holds {100*len(stand_free)/len(ticks):.1f}%')
print(f'speed {statistics.mean(sp):.1f} ± {statistics.pstdev(sp):.1f}')
# standing stretches > 1 s outside transformer holds
run = []
for x in ticks + [(1e9, 0, 0, 0, 99)]:
    if x[4] < 0.3 and not held(x[0]): run.append(x)
    else:
        if len(run) >= 60:
            a = run[0]; print(f'  still {len(run)/60:5.1f}s from {int(a[0]//60):02d}:{a[0]%60:05.2f} at ({a[1]:.1f} {a[2]:.1f} {a[3]:.1f})')
        run = []
slow = [x for x in ticks if x[4] < 1.0 and not held(x[0])]
print(f'slow (<1.0, no transformer hold) {100*len(slow)/len(ticks):.1f}%')
run = []
for x in ticks + [(1e9, 0, 0, 0, 99)]:
    if x[4] < 1.0 and not held(x[0]): run.append(x)
    else:
        if len(run) >= 120:
            a = run[0]; print(f'  slow {len(run)/60:5.1f}s from {int(a[0]//60):02d}:{a[0]%60:05.2f} at ({a[1]:.1f} {a[2]:.1f} {a[3]:.1f})')
        run = []
