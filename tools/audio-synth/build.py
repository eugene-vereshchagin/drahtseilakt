"""Render every role, score it against the original, write compare files.
usage: python3 build.py [role ...] [--search N]"""
import sys, json, os, numpy as np, scipy.io.wavfile as wf
from scipy.signal import resample_poly
import synth as S
from dsp import SR
from metric import compare
from analyse import load, D

H = os.path.dirname(os.path.abspath(__file__))
def P(**k): return k

# role: (generator(p), original, defaults, search-space {name:(lo,hi)})
def H_(b, f, d): return lambda p: S.hit(b, f, d, p)
def R_(b, f, d): return lambda p: S.roll(b, f, d, p)
HIT = P(contact=.0015, grit=.3, ftune=1., dec=1., ballg=1.)
HITS = dict(contact=(.0003, .006), grit=(0, 1.5), ftune=(.9, 1.1), dec=(.5, 2), ballg=(0, 3))
ROLL = P(hiss=0., xg=1., rate=300, spiky=2., noise=.05, wob=.3, wobf=5, ftune=1., dec=1., ballg=1., rumf=120, rum=.5)
ROLLS = dict(hiss=(0, 1.5), xg=(0, 3), rate=(20, 1500), spiky=(1, 4), noise=(0, .5), wob=(0, .8), wobf=(2, 40), ftune=(.6, 1.6), dec=(.3, 3), ballg=(0, 3), rumf=(40, 500), rum=(0, 2))
# extra resonances per ball/floor pair (from the original's peak list)
XR = {('stone', 'metal'): [(124, .12, 1.5), (129, .12, 1.0), (113, .1, .6)],
      ('stone', 'stone'): [(436, .04, 1.0), (415, .04, .7), (501, .03, .6)],
      ('wood', 'metal'): [(253, .15, 1.0), (495, .12, .8)],
      ('paper', 'paper'): [(2450, .004, .5), (3100, .004, .6), (2900, .003, .4)]}
ROLES = {
 'start_level':     (S.start_level, 'Misc_StartLevel', P(chord_decay=1., drop_start=1.6, drop_gain=.8), dict(chord_decay=(.5, 1.5), drop_start=(1.2, 2.), drop_gain=(.2, 2))),
 'checkpoint':      (S.checkpoint, 'Misc_Checkpoint', P(dec=1., beat=1.6, hi=.05), dict(dec=(.5, 2), beat=(.5, 4), hi=(0, .4))),
 'new_ball_lightning': (S.lightning, 'Misc_Lightning', P(curve=1., zap=.03), dict(curve=(.5, 2), zap=(0, .1))),
 'extra_start':     (S.extra_start, 'Extra_Start', P(tau=.7, noise=.3, gtau=.4, end=2.3, glit=.5, low=.05), dict(glit=(.1, 2), low=(0, .3), tau=(.3, 1.5), noise=(0, 1.5), gtau=(.1, 1), end=(1.5, 3.2))),
 'extra_hit':       (S.extra_hit, 'Extra_Hit', P(tau=.06, wide=.3), dict(tau=(.02, .2), wide=(0, 1.5))),
 'extra_life_blob': (S.extra_life_blob, 'Extra_Life_Blob', P(tau=.012, thump=.4), dict(tau=(.004, .05), thump=(0, 1.5))),
 'extra_ball':      (S.extraball, 'Misc_extraball', P(tau=.07, air=.15), dict(tau=(.03, .2), air=(0, .6))),
 'transformer':     (S.trafo, 'Misc_Trafo', P(tau1=.18), dict(tau1=(.08, .5))),
 'rope_tears':      (S.rope_tears, 'Misc_RopeTears', P(bed=.15), dict(bed=(.03, .5))),
 'menu_click':      (S.menu_click, 'Menu_click', P(f=3600, body=.4), dict(f=(3000, 4500), body=(0, 1.5))),
 'fan_loop':        (S.fan, 'Misc_Ventilator', P(air=.5), dict(air=(.1, 2))),
}
for b, f, d in [('wood', 'wood', .12), ('stone', 'stone', .13), ('stone', 'wood', .21), ('wood', 'stone', .1)]:
    o = 'Hit_Paper' if b == 'paper' else f'Hit_{b.title()}_{f.title()}'
    ROLES[f'hit_{b}_{f}'] = (H_(b, f, d), o, dict(HIT), HITS)
for b, f, d in [('wood', 'wood', 1.03), ('wood', 'stone', 1.64), ('wood', 'metal', 1.77), ('stone', 'stone', 2.12), ('stone', 'wood', 1.92), ('stone', 'metal', 2.24)]:
    o = 'Roll_Paper' if b == 'paper' else f'Roll_{b.title()}_{f.title()}'
    ROLES[f'roll_{b}_{f}'] = (R_(b, f, d), o, dict(ROLL, extra=XR.get((b, f), [])), ROLLS)

# redesigned 2026-11-02 after the owner listened: paper as crackle/rustle, the iron posts as a ringing steel beam.
# Their decays are kept long on purpose (the original's files are cut short; a post must ring), so the search
# ranges keep them from being tuned back into a dull thud.
ROLES['hit_paper_paper'] = (lambda p: S.paper_hit(.3, p), 'Hit_Paper', P(rate=900, tau=.07, thump=.25),
                            dict(rate=(300, 3000), tau=(.03, .14), thump=(0, .8)))
ROLES['roll_paper_paper'] = (lambda p: S.paper_roll(1.49, p), 'Roll_Paper',
                             P(turn=3, swell=.5, bed_lo=1500, bed_hi=6000, bed=.6, rate=500),
                             dict(turn=(1, 8), swell=(0, .9), bed_lo=(800, 3000), bed_hi=(3000, 9000), bed=(.1, 1.5),
                                  rate=(150, 2500)))
POST = dict(contact=(.0002, .001), click=(0, 1.5), f0=(300, 700), ring=(.3, 1.2), bright=(.5, .95), beat=(.5, 3),
            body=(0, 1.2))
ROLES['hit_wood_metal'] = (lambda p: S.metal_hit('wood', .45, p), 'Hit_Wood_Metal',
                           P(contact=.0004, click=.6, f0=520, ring=.6, bright=.75, beat=1.5, body=.4), POST)
ROLES['hit_stone_metal'] = (lambda p: S.metal_hit('stone', .5, p), 'Hit_Stone_Metal',
                            P(contact=.0006, click=.3, f0=430, ring=.7, bright=.7, beat=1.2, body=.8), POST)

# second redesign 2026-11-02 (owner listened again and rejected these five): a different model for each, see synth.py
ROLES['ball_fall'] = (S.fall2, 'Misc_Fall',
                      P(p0=.30, pend=.11, ptime=2.2, f0=280, oct=1.4, tmax=2.5, low=.3, sweep=2.3, fc=1500, bw=40, air=.35, pipe=.15, blur=1.5,
                        hold=1.0, slope=10.5),
                      dict(p0=(.24, .36), pend=(.08, .16), ptime=(1.5, 3), oct=(1, 2.5), sweep=(1.3, 2), fc=(800, 3000),
                           bw=(10, 80), air=(0, .8), pipe=(0, .4), blur=(.5, 4), hold=(.8, 1.6), slope=(8, 14)))
ROLES['hit_paper_paper'] = (lambda p: S.paper_hit2(.3, p), 'Hit_Paper', P(scrunch=.1, snap=.11, rustles=5, puff=.04, floor=1.4),
                            dict(scrunch=(.07, .12), snap=(.1, .13), rustles=(2, 8), puff=(0, .4), floor=(.2, 2)))
ROLES['hit_stone_metal'] = (lambda p: S.stone_metal_hit(.22, p), 'Hit_Stone_Metal',
                            P(contact=.0006, chat=.8, grit=.4, tau=.07, dense=80, metal=1., tilt=1., noise=.1),
                            dict(contact=(.0004, .002), chat=(.7, .95), grit=(0, 1), tau=(.06, .1), metal=(.3, 2),
                                 tilt=(-.5, 1.5), noise=(0, .5)))
ROLES['roll_stone_metal'] = (lambda p: S.roll_stone_metal2(2.24, p), 'Roll_Stone_Metal',
                             P(rate=9, spread=.8, tau=.03, grind=.07, mid=4, ring=.4, click=.5, hum=.6),
                             dict(rate=(6, 13), spread=(.2, 1.), tau=(.02, .08), grind=(.02, .3), mid=(1, 5), ring=(0, 1),
                                  click=(0, 1), hum=(.3, 1.5)))
ROLES['roll_stone_wood'] = (lambda p: S.roll_stone_wood2(1.92, p), 'Roll_Stone_Wood',
                            P(rate=60, noise=.05, tau=.08, upper=.45, wander=8, wfreq=8, grit=.03),
                            dict(rate=(45, 100), noise=(0, .3), tau=(.04, .15), upper=(.1, 1.2), wander=(4, 20),
                                 wfreq=(2, 12), grit=(0, .1)))

# the roles the system theme used to fill (2026-11-03): the neuro theme is complete without a fallback
ROLES['hit_stone_dome'] = (lambda p: S.dome_hit('stone', 1.22, p), 'Hit_Stone_Kuppel',
                           P(contact=.0008, ring=1., knock=.12, knock_hi=800, knock_tau=.04),
                           dict(contact=(.0003, .003), ring=(.6, 1.5), knock=(0, .8), knock_tau=(.01, .1)))
ROLES['hit_wood_dome'] = (lambda p: S.dome_hit('wood', 2.05, p), 'Hit_Wood_Dome',
                          P(contact=.002, ring=1., knock=.15, knock_hi=1500, knock_tau=.03),
                          dict(contact=(.0005, .006), ring=(.6, 1.5), knock=(0, .6), knock_tau=(.01, .1)))
ROLES['menu_load'] = (S.menu_load, 'Menu_load', P(ring=1.3, sweeps=8, rise=1.0, sweep=.12),
                      dict(ring=(.8, 2.), rise=(.6, 1.5), sweep=(.1, .8)))
ROLES['music_thunder'] = (S.thunder, 'Music_thunder', P(crack=.6, fade=1.8, gap=.35),
                          dict(crack=(.2, 1.2), fade=(2, 8), gap=(.15, .8)))
for k in (1, 2, 3):
    ROLES[f'music_atmo_{k}'] = ((lambda k: lambda p: S.atmo(k, p))(k), f'Music_Atmo_{k}', P(fadein=8., tones=.4),
                                dict(fadein=(2, 15), tones=(0, 1)))

RMS_MATCH = {'menu_load', 'music_thunder', 'music_atmo_1', 'music_atmo_2', 'music_atmo_3'}

def render(role, p):
    g, o, _, _ = ROLES[role]
    y = np.nan_to_num(g(p)); return y / (np.abs(y).max() + 1e-12)

def score(role, p, osr, ox):
    return compare(ox, osr, render(role, p), SR, loop=role.startswith(('roll', 'fan')))

def search(role, iters, osr, ox):
    _, _, p0, space = ROLES[role]
    best = dict(p0); bs = score(role, best, osr, ox)['score']
    r = np.random.default_rng(42)
    for i in range(iters):
        cand = dict(best)
        for k, (lo, hi) in space.items():
            if r.random() < .5:  # perturb half the params, shrinking steps
                w = (hi - lo) * (0.5 if i < iters / 3 else 0.15)
                cand[k] = float(np.clip(cand[k] + r.normal(0, w), lo, hi))
        s = score(role, cand, osr, ox)['score']
        if s > bs: best, bs = cand, s
    return best

def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    it = int(sys.argv[sys.argv.index('--search') + 1]) if '--search' in sys.argv else 0
    args = [a for a in args if not a.isdigit()]
    roles = args or list(ROLES)
    os.makedirs(f'{H}/generated', exist_ok=True); os.makedirs(f'{H}/compare', exist_ok=True)
    pf = f'{H}/params.json'; saved = json.load(open(pf)) if os.path.exists(pf) else {}
    sf = f'{H}/scores.json'; scores = json.load(open(sf)) if os.path.exists(sf) else {}
    for role in roles:
        g, o, p0, space = ROLES[role]
        osr, ox = load(D + o + '.wav')
        p = {**p0, **saved.get(role, {})}
        base = score(role, p, osr, ox)
        if it and space:
            p2 = search(role, it, osr, ox); s2 = score(role, p2, osr, ox)
            if s2['score'] > base['score']: p, base = p2, s2
        saved[role] = p
        y = render(role, p) * np.abs(ox).max()  # match the original's peak level
        if role in RMS_MATCH:  # long quiet pieces: match the loudness instead, never above the original's peak
            y *= min(np.sqrt(np.mean(ox ** 2) / (np.mean(y ** 2) + 1e-20)), 1.)
        wf.write(f'{H}/generated/{role}.wav', SR, (y * 32767).astype(np.int16))
        oo = resample_poly(ox, SR // 50, osr // 50) if osr != SR else ox  # listening aid only
        cmp_ = np.r_[oo, np.zeros(SR), y]
        wf.write(f'{H}/compare/{role}_compare.wav', SR, (np.clip(cmp_, -1, 1) * 32767).astype(np.int16))
        scores[role] = dict(original=o, **base)
        print(f'{role:20s} {o:18s}', base, flush=True)
    json.dump(saved, open(pf, 'w'), indent=1); json.dump(scores, open(sf, 'w'), indent=1)

if __name__ == '__main__': main()
