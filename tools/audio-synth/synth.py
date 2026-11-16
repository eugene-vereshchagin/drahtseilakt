"""Procedural re-creations of Ballance-like sound effects.
Nothing here reads the original samples: all parameters are numbers
chosen by ear/by analysis (pitches, decays, bands, durations)."""
import numpy as np, zlib
from dsp import *

# ---------------------------------------------------------------- jingles
def start_level(p):
    """Bell/pad chord that slowly dies, then a deep descending 'drop' tone."""
    n = int(4.25 * SR); t = t_axis(4.25); y = np.zeros(n); r = rng(1)
    chord = [(231, 1.0, 1.6), (285, .55, 1.4), (463, .75, 1.1), (738, .6, 1.3), (861, .4, .8), (668, .2, .7)]
    for f, g, tau in chord:
        for det in (-0.6, 0.6):  # slight chorus/beating
            ph = r.uniform(0, 6.28)
            env = (1 - np.exp(-t / 0.02)) * np.exp(-t / (tau * p['chord_decay']))
            y += g * env * np.sin(2 * np.pi * (f + det) * t + ph + 0.3 * np.sin(2*np.pi*f*2*t) * np.exp(-t/0.15))
    # descending low 'whoom' 115 Hz -> 38 Hz, with a few harmonics
    s0 = p['drop_start']; m = n - int(s0 * SR)
    f = glide(115, 38, m); ph = phase(f)
    tone = np.sin(ph) + .35 * np.sin(2 * ph) + .15 * np.sin(3 * ph)
    u = np.arange(m) / SR
    tone *= (1 - np.exp(-u / 0.25)) * np.exp(-u / 1.6) * p['drop_gain']
    y[-m:] += tone
    return fade(y, 0.001, 0.2)

def checkpoint(p):
    """Gong/bell: modal partials with beating pairs, long decay, plus
    inharmonic high partials (2-8 kHz) that give the metallic shimmer."""
    n = int(3.47 * SR); exc = np.zeros(n); exc[0] = 1
    exc += np.r_[rng(2).normal(0, 1, 300) * np.exp(-np.arange(300) / 60), np.zeros(n - 300)] * .05
    modes = [(280, 1.1 * p['dec'], 1.0), (280 + p['beat'], 1.0 * p['dec'], .7), (161, .9 * p['dec'], .35),
             (161 + p['beat'] * .9, .8 * p['dec'], .25), (118, .25, .8), (431, .7 * p['dec'], .2),
             (528, .35, .35), (603, .3, .25), (345, .5, .12), (1490, .6, .12), (1530, .5, .08), (991, .4, .05)]
    modes += [(f, .5 * p['dec'], p['hi'] * g) for f, g in [(2210, .6), (2870, .5), (3640, .4), (4480, .35), (5390, .3), (6420, .25), (7700, .2)]]
    y = modal(exc, modes)
    t = t_axis(3.47); y *= 1 + .25 * np.sin(2 * np.pi * 0.8 * t)
    return fade(y, .001, .1)

def lightning(p):
    """New-ball electric crackle: sparse sparks whose rate and level rise
    for ~2.5 s, then a short burst and a low zap."""
    dur = 3.46; n = int(dur * SR); t = t_axis(dur); r = rng(4); y = np.zeros(n)
    tt = 0.05
    while tt < 3.0:
        lvl = 10 ** ((-70 + 50 * min(tt / 2.55, 1) ** p['curve']) / 20)
        L = int(r.uniform(.0005, .004) * SR)
        spark = r.normal(0, 1, L) * np.exp(-np.arange(L) / (L / 4))
        place(y, spark * lvl * r.uniform(.4, 1.2), tt)
        rate = 30 + 900 * min(tt / 2.6, 1) ** 2
        tt += r.exponential(1 / rate)
    y = highpass(y, 1000, 2) + .4 * resonator(y, 7140, 6) + .5 * resonator(y, 4500, 3) + .3 * resonator(y, 2500, 2)
    y = lowpass(y, 9000, 2)
    hiss = bandpass(r.normal(0, 1, n), 1500, 8000, 2) * 10 ** ((-75 + 50 * np.clip(t / 2.55, 0, 1)) / 20)
    hiss *= np.where(t > 2.65, np.exp(-(t - 2.65) / .1), 1)
    y = y * np.where(t > 2.75, np.exp(-(t - 2.75) / .08), 1) + .5 * hiss
    zap_n = int(.6 * SR); u = np.arange(zap_n) / SR
    zap = np.sin(phase(glide(900, 90, zap_n))) * np.exp(-u / .12) * p['zap']
    zap += lowpass(r.normal(0, 1, zap_n), 1800) * np.exp(-u / .08) * .1
    place(y, zap, 2.62)
    return fade(y, .01, .1)

def extra_start(p):
    """Extra-point sparkle: low pluck, then a bright shimmering cluster that
    sweeps down 7.2 kHz -> 1.8 kHz and rings out with a high glitter; a
    breathy noise band follows the sweep."""
    dur = 2.97; n = int(dur * SR); t = t_axis(dur); r = rng(5); y = np.zeros(n)
    exc = np.zeros(n); exc[:200] = r.normal(0, 1, 200)
    y += modal(exc, [(151, .15, .5), (248, .15, .4), (301, .12, .4), (1540, .1, .3), (2358, .12, .3), (590, .15, .3), (80, .1, .3)]) * .02
    y += p['low'] * lowpass(r.normal(0, 1, n), 2500, 2) * np.exp(-t / .5)  # soft airy body under the sparkle
    s = int(.25 * SR); m = int(1.05 * SR)
    fc = np.r_[np.full(s, 7200.), glide(7200, 1800, m), np.full(n - s - m, 1800.)]
    for k in range(8):
        f = fc[s:] * (1 + r.uniform(-.04, .04))
        env = np.r_[np.linspace(0, 1, int(.04 * SR)), np.ones(n - s - int(.04 * SR))] * np.exp(-np.arange(n - s) / SR / p['tau'])
        trem = 1 + .5 * np.sin(2 * np.pi * r.uniform(9, 17) * np.arange(n - s) / SR)
        y[s:] += .12 * np.sin(phase(f) + r.uniform(0, 6)) * env * trem
    # time-varying band of noise following the sweep (block-wise filtering)
    nz = r.normal(0, 1, n); band = np.zeros(n); B = 2048
    for i in range(0, n, B):
        c = fc[i]; seg = bandpass(nz[max(0, i - B):i + B], c * .5, min(c * 1.6, 20000), 2)
        band[i:i + B] = seg[-len(nz[i:i + B]):] if i else seg[:B]
    y += p['noise'] * band * np.exp(-np.maximum(t - .25, 0) / p['tau']) * np.clip((t - .05) / .2, 0, 1)
    glit = resonator(r.normal(0, 1, n), 7130, 25) * np.exp(-np.maximum(t - .3, 0) / p['gtau']) * (t > .25) * p['glit']
    glit += highpass(r.normal(0, 1, n), 8000) * .015 * np.exp(-np.maximum(t - .3, 0) / .3) * (t > .25)
    y = (y + glit) * np.exp(-(t / p['end']) ** 4)  # steepening fade-out like the original
    return fade(y, .001, .1)

def extra_hit(p):
    """Collecting one extra-point ball: tiny bright 'tsing'."""
    dur = .5; n = int(dur * SR); t = t_axis(dur); r = rng(6)
    y = sum(resonator(r.normal(0, 1, n), f, 30) for f in (5100, 5850, 6200, 6450, 9300)) * .3
    y += sum(np.sin(2 * np.pi * f * t + r.uniform(0, 6)) * .3 for f in (5060, 5330, 6100))
    y += p['wide'] * bandpass(r.normal(0, 1, n), 900, 5000, 2)
    env = np.exp(-t / p['tau']) * (1 + .3 * np.sin(2 * np.pi * 22 * t))
    return fade(y * env, .001, .05)

def extra_life_blob(p):
    """'Bloop': fast upward sine chirp 600 -> 1300 Hz in ~20 ms, a soft
    low thump under it and a faint bubbly 0.8-1.1 kHz tail."""
    dur = .32; n = int(dur * SR); t = t_axis(dur); r = rng(11)
    f = 600 * (1300 / 600) ** np.clip(t / .02, 0, 1)
    y = np.sin(phase(f)) * np.clip(t / .004, 0, 1) * np.exp(-np.maximum(t - .006, 0) / p['tau'])
    y += p['thump'] * np.sin(2 * np.pi * 86 * t) * np.exp(-t / .09) * np.clip(t / .01, 0, 1)
    bub = resonator(r.normal(0, 1, n), 1000, 6) * np.exp(-t / .08) * (1 + np.sin(2 * np.pi * 30 * t))
    return fade(y + .02 * bub, .0005, .02)

def extraball(p):
    """Extra ball: breathy whistle bending 1650 -> 1800 Hz and back, with
    octave partials, air noise, a bright attack tick and a faint low tail."""
    dur = .91; n = int(dur * SR); t = t_axis(dur); r = rng(12)
    f = 1650 + 150 * np.sin(np.pi * np.clip(t / .55, 0, 1)) ** 1.5 + 12 * np.sin(2 * np.pi * 7 * t)
    ph = phase(f)
    y = np.sin(ph) + .35 * np.sin(2 * ph) + .1 * np.sin(3 * ph) + .2 * np.sin(4 * ph)
    y += p['air'] * bandpass(r.normal(0, 1, n), 300, 9000, 2)
    env = np.clip(t / .02, 0, 1) * np.exp(-np.maximum(t - .12, 0) / p['tau'])
    y *= env
    y += .5 * np.sin(2 * np.pi * 6900 * t) * np.exp(-t / .03)
    y += .03 * np.sin(2 * np.pi * 1082 * t) * np.exp(-t / .3)
    return fade(y, .0005, .05)

def trafo(p):
    """Transformer: electric zap bursts stepping down through bands
    (8 kHz -> 4 -> 1.2 kHz) with a buzzing 700-950 Hz body."""
    dur = 3.11; n = int(dur * SR); r = rng(7); y = np.zeros(n)
    def burst(t0, d, lo, hi, tau, g, buzz=0):
        m = int(d * SR); u = np.arange(m) / SR
        b = bandpass(r.normal(0, 1, m), lo, hi) * np.exp(-u / tau)
        if buzz:
            saw = 2 * ((buzz * u) % 1) - 1
            b += .5 * bandpass(saw, 600, 1000, 2) * np.exp(-u / tau)
        place(y, g * fade(b, .002, .01), t0)
    burst(.2, .1, 500, 1100, .05, .4, 50)
    burst(.3, .2, 5000, 12000, .06, 1.0)
    burst(.42, .1, 3000, 9000, .03, .4)
    burst(.92, .9, 7500, 9500, p['tau1'], 1.0)
    burst(1.8, .2, 6000, 11000, .05, .7)
    burst(1.92, .5, 3200, 4300, .12, .8)
    burst(2.38, .25, 700, 2500, .05, .9, 100)
    burst(2.6, .2, 2000, 3000, .06, .5)
    burst(2.72, .18, 3000, 10000, .04, 1.0, 100)
    return fade(y, .001, .05)

def rope_tears(p):
    """Rope snaps: a creaking fibre-crackle bed with three sharp snaps
    (35 ms, ~95 ms, ~170 ms); resonances around 0.7/1.4/1.9 kHz."""
    dur = .19; n = int(dur * SR); r = rng(8); y = np.zeros(n)
    tt = 0.
    while tt < dur:  # fibre crackle bed
        m = int(.002 * SR); place(y, r.normal(0, 1, m) * np.exp(-np.arange(m) / 20) * p['bed'], tt); tt += r.exponential(.004)
    for t0, g in [(.035, .45), (.09, .8), (.1, .6), (.168, 1.0), (.174, .5)]:
        m = int(.01 * SR); e = r.normal(0, 1, m) * np.exp(-np.arange(m) / (m / 6))
        place(y, e * g, t0)
    y = resonator(y, 1400, 4) + .7 * resonator(y, 700, 5) + .5 * resonator(y, 1900, 4) + .2 * resonator(y, 240, 3) + .3 * highpass(y, 3000)
    return fade(y, .0005, .003)

def menu_click(p):
    """Click: 3.6 kHz ping + 8 kHz tick at the start, then a short low
    1.2 kHz/400 Hz body; total ~20 ms."""
    dur = .03; n = int(dur * SR); t = t_axis(dur); r = rng(9)
    y = np.sin(2 * np.pi * p['f'] * t) * np.exp(-t / .005)
    y += .4 * highpass(r.normal(0, 1, n), 6000) * np.exp(-t / .003)
    y += .5 * np.sin(2 * np.pi * 1250 * t) * np.exp(-np.abs(t - .011) / .003)
    y += p['body'] * lowpass(r.normal(0, 1, n), 1500, 2) * np.exp(-np.abs(t - .013) / .003)
    y *= t < .019
    return fade(y, .0002, .003)

def fan(p):
    """Fan loop (1.15 s, seamless): blade-pass at ~41 Hz, motor hum 124 Hz
    and air noise amplitude-modulated by the blades."""
    dur = 1.15; n = int(dur * SR); t = t_axis(dur); r = rng(10)
    fb = round(41 * dur) / dur; fm = round(124 * dur) / dur  # integer cycles -> loops
    y = sum(g * np.sin(2 * np.pi * k * fb * t + k) for k, g in [(1, 1), (2, .35), (3, .5), (4, .15)])
    y += .6 * np.sin(2 * np.pi * fm * t) + .2 * np.sin(2 * np.pi * 2 * fm * t)
    # cyclic noise: generate in freq domain so it loops seamlessly
    N = np.fft.rfft(r.normal(0, 1, n)); f = np.fft.rfftfreq(n, 1 / SR)
    N *= 1 / (1 + (f / 250) ** 2) + .3 / (1 + ((f - 1500) / 1200) ** 2) * (f < 6000)
    air = np.fft.irfft(N, n); air /= np.abs(air).max()
    y = y / np.abs(y).max() + p['air'] * air * (1 + .6 * np.sin(2 * np.pi * fb * t))
    return y

# ---------------------------------------------------------------- materials
# modal sets per floor material (freq Hz, decay s, gain); ball adds its own
MAT = {
    'wood':  [(213, .05, 1), (229, .045, .8), (395, .03, .4), (612, .02, .25), (1080, .012, .1)],
    'stone': [(420, .012, .6), (880, .01, .7), (1250, .008, .5), (2100, .005, .3), (95, .03, .6)],
    'metal': [(285, .25, .8), (291, .22, .7), (497, .18, .5), (870, .15, .4), (1420, .12, .3), (2310, .08, .25), (3600, .05, .15)],
    'paper': [(1430, .008, .3), (3100, .004, .4), (4500, .003, .4), (6800, .002, .3)],
}
BALL = {'wood': [(57, .06, 1.0), (221, .04, .5)], 'stone': [(47, .04, .8), (95, .03, .4)], 'paper': []}

# per ball/floor pair: (freq, level dB, level change dB over the 2nd half)
# read off the originals' spectra; turned into modal decays below
PAIR = {
 ('wood', 'wood'):   [(221, 0, -26), (59, -6, -7), (382, -13, -26), (161, -14, -11), (108, -15, -9), (495, -16, -25), (560, -25, -16), (641, -25, -20), (818, -26, -21)],
 ('stone', 'stone'): [(1233, 0, -32), (48, -3, -7), (97, -4, -26), (2272, -24, -28), (2885, -25, -31), (2503, -34, -19), (4608, -40, -3), (3688, -41, -4), (6449, -42, -3)],
 ('stone', 'metal'): [(65, 0, -11), (237, -1, -23), (97, -1, -14), (285, -2, -17), (86, -3, -8), (135, -5, -10), (145, -6, -8), (178, -6, -11), (339, -8, -15), (404, -8, -15)],
 ('wood', 'metal'):  [(97, 0, -11), (145, 0, -14), (350, -1, -23), (431, -2, -17), (129, -3, -7), (199, -5, -10), (178, -6, -9), (264, -7, -11), (608, -7, -15), (506, -8, -15)],
 ('stone', 'wood'):  [(291, 0, -17), (86, -2, -15), (199, -8, -14), (135, -9, -20), (237, -10, -20), (339, -15, -10), (377, -18, -16), (172, -19, -14), (420, -21, -18)],
 ('wood', 'stone'):  [(361, 0, -39), (1184, -2, -35), (4140, -35, -13), (4016, -39, -14), (3671, -43, -24), (6358, -46, -26), (4554, -47, -27)],
 ('paper', 'paper'): [(3849, -6, -14), (2471, -7, -17), (3596, -8, -12), (3714, -8, -13), (3494, -8, -11), (2541, -9, -15)],
}

PRE = {('wood', 'wood'): .005, ('stone', 'stone'): .0026, ('stone', 'metal'): .006, ('wood', 'metal'): .004, ('stone', 'wood'): .0048, ('wood', 'stone'): .0072}

def pair_modes(ball, floor, dur):
    out = []
    for f, db, dd in PAIR[(ball, floor)]:
        tau = (dur / 2) / max(-dd / 8.686, 0.3)
        out.append((f, min(tau, 2.), 10 ** (db / 20) / (tau * 50)))
    return out

def hit(ball, floor, dur, p):
    n = int(dur * SR); r = rng(zlib.crc32(f"{ball}{floor}".encode()))
    L = int(p['contact'] * SR)  # contact time -> excitation brightness
    exc = np.zeros(n); exc[:L] = np.hanning(L)
    exc[:int(.003 * SR)] += r.normal(0, p['grit'], int(.003 * SR)) * np.exp(-np.arange(int(.003 * SR)) / 30)
    modes = [(f * p['ftune'], d * p['dec'], g) for f, d, g in pair_modes(ball, floor, dur)]
    y = modal(exc, modes)
    y += p['ballg'] * .05 * np.abs(y).max() * lowpass(r.normal(0, 1, n), 3000) * np.exp(-np.arange(n) / SR / .01)  # contact noise
    if floor == 'paper':
        y += bandpass(r.normal(0, 1, n), 1500, 9000) * np.exp(-np.arange(n) / SR / .08) * .3 * (1 + np.sin(2*np.pi*35*np.arange(n)/SR))
    pre = int(PRE.get((ball, floor), 0) * SR)  # same short lead-in as the game's files
    return fade(np.r_[np.zeros(pre), y[:n - pre]], .0003, .01)

def roll(ball, floor, dur, p):
    """Rolling loop: stream of micro-impacts (Poisson, rate ~ surface grain)
    exciting the floor/ball modes + low rumble; built cyclically so it loops."""
    n = int(dur * SR); r = rng(zlib.crc32(f"{ball}{floor}r".encode()))
    exc = np.zeros(n)
    k = r.poisson(p['rate'] * dur)
    pos = r.integers(0, n, k); exc[pos] += r.exponential(1, k) ** p['spiky']
    exc += r.normal(0, p['noise'], n)
    # slow wobble (ball irregularity)
    tt = np.arange(n) / SR; wob = 1 + p['wob'] * np.sin(2 * np.pi * round(p['wobf'] * dur) / dur * tt)
    exc *= wob
    modes = [(f * p['ftune'], d * p['dec'], g) for f, d, g in MAT[floor]] + [(f, d, g * p['ballg']) for f, d, g in BALL[ball]]
    modes += [(f, d, g * p.get('xg', 1)) for f, d, g in p.get('extra', [])]
    # cyclic convolution trick: render twice, keep the second copy -> loopable
    y = modal(np.r_[exc, exc], modes)[n:]
    if floor == 'paper':
        y = y + highpass(np.r_[exc, exc], 1500)[n:] * .5
    rum = np.fft.rfft(r.normal(0, 1, n)); f = np.fft.rfftfreq(n, 1 / SR)
    rum *= 1 / (1 + ((f - p['rumf']) / (p['rumf'] * .6)) ** 2)
    rum = np.fft.irfft(rum, n); rum *= p['rum'] * np.abs(y).max() / (np.abs(rum).max() + 1e-9)
    hs = np.fft.rfft(r.normal(0, 1, n)); hs *= (f > 1500) / (1 + (f / 9000) ** 2); hs = np.fft.irfft(hs, n)
    hs *= p.get('hiss', 0) * np.abs(y).max() / (np.abs(hs).max() + 1e-9)
    return y + rum * wob + hs * wob


# ---------------------------------------------------------------- paper (redesigned 2026-11-02, owner: "not paper")
def _crackles(n, rate, r, lo, hi, burst_ms=(0.4, 4.0), cyclic=False):
    """A stream of tiny paper crackles: short noise bursts, each band-passed around its own centre between lo and hi,
    with random levels (a few loud snaps among many soft ticks). Cyclic: bursts wrap round, so the result loops."""
    y = np.zeros(n + (int(.01 * SR) if not cyclic else 0))
    k = r.poisson(rate * n / SR)
    for _ in range(k):
        L = max(8, int(r.uniform(*burst_ms) / 1000 * SR))
        b = r.normal(0, 1, L) * np.exp(-np.arange(L) / (L / 3))
        c = np.exp(r.uniform(np.log(lo), np.log(hi)))
        b = resonator(b, c, r.uniform(1.2, 4.0))
        g = r.pareto(2.2) + .15
        i = r.integers(0, n)
        if cyclic:
            idx = (i + np.arange(L)) % n; np.add.at(y, idx, g * b)
        else:
            j = min(len(y), i + L); y[i:j] += g * b[:j - i]
    return y[:n]

def paper_roll(dur, p):
    """Rolling paper ball: a soft rustle bed (band-limited noise, slow swells as the crumpled ball turns) with a dense
    granular crackle on top; built cyclically so it loops without a seam."""
    n = int(dur * SR); r = rng(zlib.crc32(b"paper-roll-2"))
    tt = np.arange(n) / SR
    turns = max(1, round(p['turn'] * dur))
    swell = 1 + p['swell'] * (0.5 * np.sin(2 * np.pi * turns / dur * tt) + 0.5 * np.sin(2 * np.pi * (2 * turns + 1) / dur * tt + 1.3))
    bed = np.fft.rfft(r.normal(0, 1, n)); f = np.fft.rfftfreq(n, 1 / SR)
    bed *= ((f > p['bed_lo']) & (f < 12000)) / (1 + (f / p['bed_hi']) ** 2)
    bed = np.fft.irfft(bed, n); bed /= np.abs(bed).max() + 1e-9
    cr = _crackles(n, p['rate'], r, 1200, 11000, cyclic=True); cr /= np.abs(cr).max() + 1e-9
    return (p['bed'] * bed + cr) * swell

# ---------------------------------------------------------------- ball on an iron post (redesigned 2026-11-02, owner: "rubber damper")
# free-free beam (a post/tube struck sideways): bending mode ratios
BEAM = [1.0, 2.756, 5.404, 8.933, 13.34, 19.9]

def metal_hit(ball, dur, p):
    """The ball against a steel post: a sharp contact click exciting the post's inharmonic bending modes, each a
    close pair (a slight beat), ringing long; the ball's own dull body under it (heavier for stone)."""
    n = int(dur * SR); r = rng(zlib.crc32(f"{ball}-post".encode()))
    t = np.arange(n) / SR
    L = max(4, int(p['contact'] * SR))
    exc = np.zeros(n); exc[:L] = np.hanning(L)
    exc[:int(.002 * SR)] += r.normal(0, p['click'], int(.002 * SR))
    modes = []
    for k, ratio in enumerate(BEAM):
        f = p['f0'] * ratio
        if f > 9000: break
        tau = p['ring'] / (1 + .35 * k)          # higher partials die sooner, as in metal
        g = p['bright'] ** k / (1 + k)
        for split in (-p['beat'], p['beat']):
            modes.append((f + split * (1 + k), tau, g * .5))
    y = modal(exc, modes)
    body_f = 140 if ball == 'stone' else 220
    body = modal(exc, [(body_f, .03, 1.0), (body_f * 2.3, .02, .4)])
    y = y / (np.abs(y).max() + 1e-9) + p['body'] * body / (np.abs(body).max() + 1e-9)
    return fade(y, .0002, .03)


# ---------------------------------------------------------------- second redesign 2026-11-02 (owner listened again)
# Five roles were redone with a different model each, not a parameter tweak; what the original's numbers show and
# what physical sound that is, is written in each docstring.

def _narrow(n, bw, r):
    """Complex Gaussian noise band-limited to +-bw Hz around 0 (analytic baseband); times exp(i*phase) it becomes
    a narrow noise band that can follow any frequency track."""
    z = r.normal(0, 1, n) + 1j * r.normal(0, 1, n)
    b, a = butter(2, bw, 'low', fs=SR)
    z = lfilter(b, a, z.real) + 1j * lfilter(b, a, z.imag)
    return z / (np.sqrt(np.mean(np.abs(z) ** 2)) + 1e-12)

def fall2(p):
    """Ball falls off the level: the original is a rushing wind made of repeated upward sweeps, a 'whoo-whoo' of air
    past a tumbling ball. Each cycle a noise band with a few harmonics (a tube resonance shortening, like wind over
    a pipe) glides up by about half an octave and is cut off; the cycles come faster (0.30 s -> ~0.13 s) and start
    higher (doubling every ~1.5 s) as the ball drops away, while the whole thing fades over ~5 s."""
    dur = 5.57; n = int(dur * SR); t = t_axis(dur); r = rng(zlib.crc32(b"fall-2"))
    per = p['pend'] + (p['p0'] - p['pend']) / (1 + (t / p['ptime']) ** 3)   # cycle length, shrinking
    cyc = np.cumsum(1 / per) / SR; u = cyc % 1                               # position inside the cycle 0..1
    fa = p['f0'] * 2 ** (np.minimum(t, p['tmax']) / p['oct'])              # where a sweep starts (rises, then stays)
    f0 = fa * p['sweep'] ** (u ** 1.6)                                       # glides up, steepest at the end
    ph = phase(f0)
    y = np.zeros(n)
    widen = 1 + t / p['blur']                                                # later sweeps smear into wind
    for k, g in enumerate([1., .5, .3, .15], 1):
        f = k * f0
        w = g / (1 + (f / p['fc']) ** 2) * (f < 9000)
        z = _narrow(n, p['bw'] * k, r)
        y += w * np.real(z * np.exp(1j * k * ph)) / widen
        y += w * np.real(_narrow(n, p['bw'] * k * 3, r) * np.exp(1j * k * ph)) * (widen - 1) / widen
    y = y / (np.abs(y).max() + 1e-9)
    shape = u ** .35 * np.clip((1 - u) / .06, 0, 1)                          # swell through the cycle, cut at its end
    y = y * (.25 + .75 * shape)
    air = lowpass(r.normal(0, 1, n), 700, 2); air /= np.abs(air).max()       # steady wind under the sweeps
    air = air + p['low'] * lowpass(r.normal(0, 1, n), 220, 4) / .2 * (.4 + .6 * shape)  # low rumble of the gusts
    pipe = resonator(r.normal(0, 1, n), 350, 30) + 1.3 * resonator(r.normal(0, 1, n), 540, 30)
    pipe /= np.abs(pipe).max()                                               # the faint steady tube tones
    y = y + p['air'] * air + p['pipe'] * pipe
    db = -p['slope'] * np.maximum(t - p['hold'], 0) - 12 * np.maximum(t - 4.9, 0) ** 2
    env = np.clip(t / .015, 0, 1) * 10 ** (db / 20)
    y = y * env
    late = lowpass(y, 1600, 2)                                               # the far-away tail loses its top
    k = np.clip((t - 2.5) / 1.5, 0, 1); y = (1 - k) * y + k * late
    y = lowpass(y, 3800, 4)
    return fade(y, .001, .1)

def paper_hit2(dur, p):
    """Paper ball landing: in the original it is not a few crackles but a dense, almost white 'shhk' (crest factor
    ~3, flat spectrum 100 Hz-8 kHz): many overlapping friction grains for ~0.1 s as the crumpled ball squashes,
    one sharper crunch, then a few soft rustles as it settles, over a faint air puff."""
    n = int(dur * SR); t = np.arange(n) / SR; r = rng(zlib.crc32(b"paper-hit-3"))
    white = r.normal(0, 1, n)
    white = white + .4 * bandpass(white, 1000, 4000, 2)                      # a slight lift where paper rustles
    white = lowpass(white, 9000, 2)
    # grain envelope: many overlapping 1-6 ms grains with random levels -> dense but rough
    g = np.zeros(n)
    def grains(t0, t1, rate, lvl):
        for _ in range(r.poisson(rate * (t1 - t0))):
            L = int(r.uniform(.001, .006) * SR); i = int(r.uniform(t0, t1) * SR)
            j = min(n, i + L); g[i:j] += lvl * r.lognormal(0, .5) * np.hanning(L)[:j - i]
    grains(.002, p['scrunch'], 1500, 1.)
    grains(p['scrunch'] * .7, p['scrunch'], 600, .5)                         # lift just before the crunch
    grains(p['snap'], p['snap'] + .015, 2500, 1.6)                            # the crunch
    for _ in range(int(p['rustles'])):                                        # settling rustles
        a = r.uniform(p['snap'] + .025, dur - .03)
        grains(a, a + r.uniform(.006, .02), 1500, r.uniform(.25, .6))
    floor = p['floor'] * np.exp(-t / .4)                                      # the paper keeps settling quietly
    y = white * g + floor * lowpass(white, 7000, 2)
    puff = lowpass(r.normal(0, 1, n), 40, 2); puff *= np.exp(-t / .15) / (np.abs(puff).max() + 1e-9)
    y = y / (np.abs(y).max() + 1e-9) + p['puff'] * puff
    return fade(y, .0005, .01)

def stone_metal_hit(dur, p):
    """Stone ball against an iron post: the original is a heavy, short 'clank' (0.22 s), loudest 60-300 Hz but with
    a dense field of metal partials up to ~4 kHz, all dying within ~0.1 s, and the ball chatters against the
    metal a few times in the first 90 ms. So: a dense random modal field (many partials, short decays) around the
    measured strong ones, struck by a train of 5-6 diminishing impacts."""
    n = int(dur * SR); r = rng(zlib.crc32(b"stone-post-2"))
    exc = np.zeros(n)
    L = max(4, int(p['contact'] * SR)); tt = .006
    for k in range(6):                                                         # the chatter
        a = p['chat'] ** max(0, k - 2) * r.uniform(.7, 1.)                    # first three about equal
        i = int(tt * SR); exc[i:i + L] += a * np.hanning(L)
        m = int(.002 * SR); exc[i:i + m] += a * p['grit'] * r.normal(0, 1, m) * np.exp(-np.arange(m) / 25)
        tt += r.uniform(.008, .025)
    strong = [(61, 0), (72, -7), (86, -4), (99, -2), (134, -5), (145, -6), (177, -8), (234, -2), (285, -3),
              (336, -9), (406, -9), (500, -12), (721, -13), (750, -9), (1073, -14), (1294, -14), (1329, -13),
              (1684, -12), (2142, -13), (2775, -13), (3606, -19)]
    modes = [(f, p['tau'] * (1.3 if f < 200 else 1.) / (1 + f / 9000), 10 ** (db / 20)) for f, db in strong]
    for f in np.exp(r.uniform(np.log(400), np.log(4500), int(p['dense']))):    # the dense metal field
        modes.append((f, p['tau'] * r.uniform(.6, 1.2) / (1 + f / 9000), 10 ** (r.uniform(-20, -10) / 20) * p['metal'] * (f / 1000) ** p['tilt']))
    y = modal(exc, modes)
    y = y / (np.abs(y).max() + 1e-9)
    y += p['noise'] * bandpass(r.normal(0, 1, n), 200, 6000, 2) * np.exp(-np.arange(n) / SR / .02)
    return fade(y, .0003, .015)

def roll_stone_metal2(dur, p):
    """Stone ball rolling on iron rails: the original is a steady low hum (~122 Hz with 79-94 Hz neighbours, the
    rail and the ball turning) plus a grinding clatter: about nine bursts a second (rail joints, grit), each a sharp
    click followed by 30-100 ms of rough broadband scrape coloured by the rail (broad bands near 0.5-0.9, 1.5, 3
    and 6 kHz plus a dense field of short metal partials), over a quieter continuous grind. Built cyclically."""
    n = int(dur * SR); r = rng(zlib.crc32(b"roll-stone-metal-3")); f = np.fft.rfftfreq(n, 1 / SR)
    tt = np.arange(n) / SR
    H = np.zeros(len(f))
    for fc, db, bw in [(122, 0, 6), (113, -6, 5), (130, -6, 5), (83, -7, 4), (94, -9, 4), (79, -9, 4), (180, -18, 6)]:
        H += 10 ** (db / 20) / (1 + ((f - fc) / bw) ** 2)
    hum = np.fft.irfft(np.fft.rfft(r.normal(0, 1, n)) * H, n); hum /= np.abs(hum).max()
    # burst envelope: jittered grid of events (never two on top of each other, never a long hole), cyclic
    k = max(1, round(p['rate'] * dur)); g = np.zeros(n); clicks = np.zeros(n)
    for j in range(k):
        i = int((j + r.uniform(-.35, .35)) * n / k) % n
        L = int(r.uniform(.03, .1) * SR); a = r.lognormal(0, p['spread'])
        idx = (i + np.arange(L)) % n
        g[idx] += a * np.exp(-np.arange(L) / (L / 3)) * np.clip(np.arange(L) / 40, 0, 1)
        c = int(.0015 * SR); clicks[(i + np.arange(c)) % n] += a * r.normal(0, 1, c) * np.exp(-np.arange(c) / 15)
    g = g / g.max() + p['grind']                                              # the quieter continuous grind
    src = np.r_[r.normal(0, 1, n), np.zeros(0)]
    def colour(x):                                                            # broad rail bands, cyclic (rendered twice)
        x2 = np.r_[x, x]
        y = (p['mid'] * resonator(x2, 700, 2.5) + .9 * resonator(x2, 1500, 3) + 1.1 * resonator(x2, 3000, 3)
             + .9 * resonator(x2, 6200, 3) + .6 * highpass(x2, 7000, 2))
        return y[n:]
    scrape = colour(src * g)
    metal = [(fr, p['tau'] * r.uniform(.6, 1.2), r.uniform(.3, 1.)) for fr in np.exp(r.uniform(np.log(430), np.log(7500), 60))]
    ring = modal(np.r_[clicks, clicks] + .05 * np.r_[src * g, src * g], metal)[n:]
    y = scrape / (np.abs(scrape).max() + 1e-9) + p['ring'] * ring / (np.abs(ring).max() + 1e-9)
    y += p['click'] * clicks / (np.abs(clicks).max() + 1e-9)
    y /= np.abs(y).max() + 1e-9
    sway = 1 + .3 * np.sin(2 * np.pi * round(3.2 * dur) / dur * tt)       # slow swell of the hum, loops
    return p['hum'] * hum * sway + y

def roll_stone_wood2(dur, p):
    """Stone ball rolling on wooden planks: the original is a smooth, deep 'rrrr' — a dense stream of small
    contacts (~60 a second, none much louder than the rest) booming in the hollow planks (72-200 Hz); no steady
    tones above that: the 200-900 Hz colour moves as the ball crosses the boards, and the level wanders by
    10-20 dB in blobs of a few tenths of a second. No knocks. Built cyclically so it loops."""
    n = int(dur * SR); r = rng(zlib.crc32(b"roll-stone-wood-3")); tt = np.arange(n) / SR
    f = np.fft.rfftfreq(n, 1 / SR)
    def slow(hz, depth):                                                      # smooth cyclic random wander, in dB
        z = np.fft.rfft(r.normal(0, 1, n)) * (f < hz) * (f > 0); z = np.fft.irfft(z, n)
        return 10 ** (depth * z / (np.abs(z).max() + 1e-9) / 20)
    exc = np.zeros(n); k = r.poisson(p['rate'] * dur)
    pos = r.integers(0, n, k); np.add.at(exc, pos, r.gamma(4, .25, k))       # gamma(4): even, no outliers
    exc += r.normal(0, p['noise'], n)
    exc *= slow(p['wfreq'], p['wander'])
    soft = lowpass(np.r_[exc, exc], 1500, 2)[n:]                              # heavy ball: soft contacts
    lo = [(41, -6), (72, 0), (88, -3), (102, -4), (113, -4), (118, -7), (130, -5), (139, -4), (145, -6),
          (158, -6), (183, -5), (196, -6)]
    hi = [(235, -6), (262, -7), (290, -8), (340, -10), (400, -12), (470, -14), (560, -18), (790, -18)]
    m = lambda bank, s, tau: [(fr * s, tau * (1.4 if fr < 100 else 1.), 10 ** (db / 20)) for fr, db in bank]
    a = modal(np.r_[soft, soft], m(lo, 1, p['tau']))[n:]; a /= np.abs(a).max() + 1e-9
    # the boards' colour moves: three shifted, well damped copies of the upper bank, crossfaded slowly
    ups = [modal(np.r_[soft, soft], m(hi, s, p['tau'] / 3))[n:] for s in (.8, 1., 1.25)]
    ups = [u / (np.abs(u).max() + 1e-9) for u in ups]
    w = [slow(1.5, 20) for _ in ups]; tot = sum(w)
    b = sum(u * wi / tot for u, wi in zip(ups, w))
    grit = highpass(np.r_[exc, exc], 1500, 2)[n:]; grit /= np.abs(grit).max() + 1e-9
    return a + p['upper'] * b + p['grit'] * grit


# ---------------------------------------------------------------- the roles the system theme used to fill (2026-11-03)
# The game also asks for these; the neuro theme now has its own, so it needs no fallback.

def _modes(table, scale_tau=1.):
    """(freq, level dB, drop dB over 0.5 s) -> modal (freq, decay s, gain)."""
    return [(f, min(3., .5 / max(drop / 8.686, .05)) * scale_tau, 10 ** (db / 20)) for f, db, drop in table]

DOME = {  # measured peaks: (Hz, dB under the strongest, dB lost over half a second)
    'stone': [(88, -18, 19), (98, -2, 15), (220, -15, 13), (282, -23, 15), (387, 0, 20), (406, -18, 18), (436, -6, 13),
              (635, -19, 27), (656, -22, 32), (693, -15, 37), (757, -23, 35), (1150, -28, 40), (1700, -32, 45)],
    'wood': [(25, -30, 13), (43, 0, 8), (56, -35, 8), (87, -32, 6), (92, -38, 4), (127, -16, 12), (177, -35, 29),
             (255, -10, 10), (261, -14, 10), (413, -24, 20), (598, -22, 25), (602, -26, 25), (950, -35, 40)],
}

def dome_hit(ball, dur, p):
    """A ball on the big dome (a hollow shell): the shell's low modes ring for a second or two — for the stone ball
    a bright 390/440 Hz pair over a 98 Hz body, for the wood ball a deep 43 Hz boom with a 255 Hz ring — struck
    through a short contact with a burst of broadband knock."""
    n = int(dur * SR); r = rng(zlib.crc32(f"dome-{ball}".encode())); t = np.arange(n) / SR
    L = max(4, int(p['contact'] * SR)); exc = np.zeros(n); exc[:L] = np.hanning(L)
    y = modal(exc, _modes(DOME[ball], p['ring']))
    y /= np.abs(y).max() + 1e-9
    knock = lowpass(r.normal(0, 1, n), p['knock_hi'], 2) * np.exp(-t / p['knock_tau'])
    y += p['knock'] * knock / (np.abs(knock).max() + 1e-9)
    return fade(y, .0003, .05)

def menu_load(p):
    """The level loading: a struck high bell (2.67 kHz with a slow 10 Hz beat, short partials at 6.2, 8.4, 9.2 and
    10 kHz) under which thin tones sweep upward, faster as they rise (each doubling in ~0.7 s), and one falls back
    near the end; ~5 s to silence."""
    dur = 5.25; n = int(dur * SR); t = t_axis(dur); r = rng(zlib.crc32(b"menu-load"))
    exc = np.zeros(n); exc[0] = 1
    y = modal(exc, [(2662, p['ring'], 1.), (2672, p['ring'] * .9, .8), (6218, .8, .7), (8400, .25, .25),
                    (9200, .2, .2), (10000, .15, .15), (10900, .12, .12)])
    y /= np.abs(y).max() + 1e-9
    sw = np.zeros(n)
    for k in range(int(p['sweeps'])):
        t0 = r.uniform(0, .6); f0 = np.exp(r.uniform(np.log(500), np.log(1300))); m = n - int(t0 * SR)
        u = np.arange(m) / SR; f = f0 * np.exp(u / p['rise'])
        a = (f < 11000) * np.exp(-u / 1.4) * np.clip(u / .05, 0, 1)
        sw[-m:] += r.uniform(.4, 1.) * a * np.sin(2 * np.pi * np.cumsum(np.minimum(f, 11000)) / SR)
    m = int(1.0 * SR); u = np.arange(m) / SR; f = 11000 * np.exp(-u / .8)          # the one that falls
    place(sw, .5 * np.sin(2 * np.pi * np.cumsum(f) / SR) * np.sin(np.pi * u / 1.0), 2.5)
    hiss = highpass(r.normal(0, 1, n), 3000) * np.exp(-t / .8)
    y = y + p['sweep'] * sw / (np.abs(sw).max() + 1e-9) + .03 * hiss
    return fade(y, .0005, .2)

def thunder(p):
    """Thunder (the lightning levels): a dry crack, then a long rumble — overlapping low rolls of filtered noise
    (strongest 45-150 Hz) arriving at random, the later ones fainter and darker, fading out over ~12 s."""
    dur = 12.07; n = int(dur * SR); t = t_axis(dur); r = rng(zlib.crc32(b"thunder"))
    y = np.zeros(n)
    crack = r.normal(0, 1, int(.25 * SR)) * np.exp(-np.arange(int(.25 * SR)) / SR / .05)
    place(y, p['crack'] * lowpass(crack, 1500, 2), .1)
    tt = .12
    while tt < 9.:
        m = int(r.uniform(.4, 1.6) * SR); u = np.arange(m) / SR
        roll = r.normal(0, 1, m) * np.clip(u / r.uniform(.03, .2), 0, 1) * np.exp(-u / r.uniform(.25, .7))
        hi = 900 * np.exp(-tt / 3) + 200
        roll = lowpass(roll, hi, 2) + 1.5 * lowpass(roll, 160, 2)
        place(y, roll * np.exp(-tt / p['fade']) * r.uniform(.4, 1.), tt)
        tt += r.exponential(p['gap'])
    rumble = lowpass(r.normal(0, 1, n), 120, 2); rumble /= np.abs(rumble).max()
    y = y / (np.abs(y).max() + 1e-9) + .25 * rumble * np.exp(-t / 5) * np.clip((t - .1) / .5, 0, 1)
    return fade(y, .05, 1.)

ATMO = {  # (Hz, level dB, from s, to s): the sustained tones of each atmosphere, read off the original's spectra
    1: [(401, -1, 10, 18), (553, -4, 0, 11), (575, -6, 0, 9), (750, 0, 7, 26), (763, -3, 9, 24), (841, -5, 13, 27),
        (872, -6, 7, 20), (1048, -4, 7, 26), (1156, -8, 12, 27), (250, -8, 17, 23), (1600, -9, 17, 27),
        (2000, -8, 0, 10)],
    3: [(1793, 0, 3, 25), (238, -8, 6, 24), (265, -8, 2, 20), (289, -6, 7, 25), (409, -9, 4, 22), (442, -8, 8, 25),
        (664, -8, 2, 25), (752, -9, 5, 22), (890, -8, 0, 21), (1429, -9, 9, 25), (1627, -9, 3, 24),
        (2495, -9, 6, 25)],
}

def atmo(k, p):
    """The quiet atmosphere under a level (Music_Atmo_<k>). 1: glassy sustained tones entering and leaving in slow
    swells; 2: a breathing band of wind around 1 kHz that widens and darkens; 3: a breathy noise pad with faint
    tones in it. Each fades in slowly and out at the end."""
    dur = {1: 27.59, 2: 26.46, 3: 25.87}[k]; n = int(dur * SR); t = t_axis(dur); r = rng(zlib.crc32(f"atmo{k}".encode()))
    y = np.zeros(n)
    if k in ATMO:
        for f, db, a, b in ATMO[k]:
            sw = np.clip((t - a) / 3, 0, 1) * np.clip((b - t) / 3, 0, 1)
            sw = sw * (1 + .25 * np.sin(2 * np.pi * r.uniform(.05, .15) * t + r.uniform(0, 6)))
            ph = 2 * np.pi * np.cumsum(f * (1 + .002 * np.sin(2 * np.pi * r.uniform(.1, .3) * t))) / SR
            tone = np.sin(ph) + .5 * np.sin(ph * 1.003 + 1)                     # a slightly detuned pair: shimmer
            y += 10 ** (db / 20) * sw * tone
        y /= np.abs(y).max() + 1e-9
    if k in (2, 3):
        # wind: noise through a slowly moving band
        nz = r.normal(0, 1, n); B = 4096; band = np.zeros(n)
        for i in range(0, n, B):
            c = (1050 if k == 2 else 900) * (1 + .15 * np.sin(2 * np.pi * i / SR / 9.)) * (1 - .3 * i / n if k == 2 else 1)
            w = (.25 if k == 2 else .9) * (1 + .6 * i / n if k == 2 else 1)
            seg = bandpass(nz[max(0, i - B):i + B], c * (1 - w / 2), c * (1 + w), 2)
            band[i:i + B] = seg[-len(nz[i:i + B]):]
        band *= 1 + .3 * np.sin(2 * np.pi * .11 * t)
        band /= np.abs(band).max() + 1e-9
        y = (p['tones'] * y + band) if k == 3 else band
    if k == 3:
        y = lowpass(y, 4000, 4)                                                 # the original is band-limited too
    env = 10 ** (np.interp(t, *zip(*ATMO_ENV[k])) / 20)
    return fade(y * env, .01, .3)

# their loudness over time (s, dB), measured second by second: a slow swell and a long fade
ATMO_ENV = {1: [(0, -40), (1.5, -19), (3, -9), (8, -4), (13, 0), (16, -2), (19, -7), (22, -17), (25, -27), (27.6, -60)],
            2: [(0, -19), (2, -7), (5, -5), (8, 0), (12, -10), (16, -18), (22, -24), (24.5, -38), (26.4, -55)],
            3: [(0, -31), (2, -12), (5, -4), (9, 0), (13, -4), (17, -10), (20, -23), (23, -39), (25.8, -60)]}
