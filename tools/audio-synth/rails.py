"""Rolling on iron rails, third model (2026-11-09): the owner said the neuro rail sound was "not at all what it
should be". Measured (the original's files only analysed, never copied):

- the old neuro loops were a *train over rail joints*: a regular clatter of ~9.6 bursts a second (wood: 74 %
  amplitude wobble at 9.7 Hz; stone: a jittered grid of 9 clicks a second with broadband scrape), and no
  sustained lines at all (wood: modes dying in ~0.3 x their period, one weak peak at 355 Hz).
- the original's wood ball on rails is a *singing rail*: a dense field of narrow, sustained lines (each ~1 Hz wide,
  ~8 Hz apart on average) from ~130 to ~900 Hz, strongest 160-520 Hz (252, 495/502, 163, 186, 242, 275, 322,
  388, 402, 434 Hz ...), centroid ~490 Hz, only ~3 % of the energy above 2 kHz; the level swells softly and
  irregularly (1-2.5 Hz, ~4 dB), no regular clatter. -37 LUFS, 1.77 s.
- the original's stone ball on rails is a *growl*: 78 % of the energy in 80-160 Hz, a dense cluster of lines
  77-163 Hz ~8.5 Hz apart (they beat: a rough, slightly wavering hum around 120-130 Hz), a weaker ringing cluster
  590-830 Hz that comes and goes, and a few irregular soft scrapes (0.5-8 kHz, 50-120 ms) at ~3-4 a second.
  -27.5 LUFS, 2.24 s.

The new model: a smooth rolling-contact noise (soft for wood, gritty for stone) drives a dense bank of high-Q rail
modes (modal synthesis); slow, smooth, cyclic random swells instead of a clatter. Everything is rendered
cyclically (the filters run over three copies of the loop, the last one kept), so the loops have no seam.

usage: python3 rails.py OUTDIR      -> OUTDIR/Roll_<Ball>_Metal_<V>.ogg/.wav, demos, features.txt
Each variant is a pure function of its seed and parameters: rerunning gives the same files.
"""
import os, sys, subprocess, zlib, numpy as np, scipy.signal as ss
from dsp import SR, modal, lowpass, highpass, bandpass, resonator

def rng(name): return np.random.default_rng(zlib.crc32(name.encode()))

def cyc(fn, x, reps=3):
    """Apply a (causal) filter as if the loop had always been playing: run it over reps copies, keep the last."""
    return fn(np.tile(x, reps))[-len(x):]

def wander(n, r, hz, depth_db):
    """Smooth cyclic random level wander: band-limited noise (0 < f < hz) scaled to +-depth_db, as a gain."""
    f = np.fft.rfftfreq(n, 1 / SR); z = np.fft.rfft(r.normal(0, 1, n)) * ((f > 0) & (f < hz))
    z = np.fft.irfft(z, n); z /= np.abs(z).max() + 1e-12
    return 10 ** (depth_db * z / 20)

def field(r, lo, hi, count, tau, shape, anchors=(), anchor_tau=None):
    """A dense random field of rail modes: count lines spread log-uniformly in [lo, hi], gains from shape(f) (dB)
    with +-4 dB scatter, decays tau=(min, max) s; plus anchor lines (f, dB)."""
    modes = []
    for f in np.exp(r.uniform(np.log(lo), np.log(hi), count)):
        modes.append((f, r.uniform(*tau), 10 ** ((shape(f) + r.uniform(-4, 4)) / 20)))
    for f, db in anchors:
        modes.append((f, r.uniform(*(anchor_tau or tau)), 10 ** (db / 20)))
    return modes

def norm(x): return x / (np.sqrt(np.mean(x ** 2)) + 1e-12)

def scrapes(n, r, rate, dur_ms, lo, hi, spread):
    """Irregular soft scrapes, rate per second on average: the gaps between them drawn from 0.4-1.6 x the mean gap
    (never a regular clatter, never a cluster), each a band of noise lo-hi Hz with a quick 4-10 ms attack and an
    exponential tail; lognormal sizes. Cyclic."""
    env = np.zeros(n); mean = SR / rate; t = [r.uniform(0, n)]
    while t[-1] - t[0] < n - 1.6 * mean: t.append(t[-1] + r.uniform(.4, 1.6) * mean)
    for i in np.array(t).astype(int):
        L = int(r.uniform(*dur_ms) / 1000 * SR); a = r.lognormal(0, spread); u = np.arange(3 * L)
        e = a * np.minimum(u / (r.uniform(.004, .010) * SR), 1) * np.exp(-u / L)
        env[(i + u) % n] += e
    src = r.normal(0, 1, n) * env
    return cyc(lambda x: bandpass(x, lo, hi, 2), src), env

# ------------------------------------------------------------------ the wood ball on rails
WOOD_ANCHORS = [(133, -11), (142, -12), (163, -6), (186, -7), (196, -9), (226, -11), (232, -9), (242, -7),
                (253, 0), (259, -10), (275, -4), (282, -11), (322, -5), (329, -10), (346, -10), (388, -9),
                (402, -7), (409, -13), (424, -11), (434, -7), (487, -11), (495, -2), (502, -5), (510, -12),
                (557, -14), (644, -13), (654, -15), (698, -14), (719, -18), (764, -21)]

def wood_shape(f):          # the original's band balance: 160-640 Hz on top, steep below 150, ~12 dB/oct above 560
    return -3 - 24 * max(0, np.log2(150 / f)) - 12 * max(0, np.log2(f / 560))

def bumps(n, r, rate, ms, spread):
    """The ball turning: soft irregular swells (~rate a second, raised-cosine, ms long, lognormal sizes). Cyclic."""
    env = np.zeros(n); k = max(1, round(rate * n / SR)); u0 = r.uniform(0, n)
    for j in range(k):
        i = int(u0 + (j + r.uniform(-.3, .3)) * n / k); L = int(r.uniform(*ms) / 1000 * SR)
        env[(i + np.arange(L)) % n] += r.lognormal(0, spread) * np.sin(np.pi * np.arange(L) / L) ** 2
    return env

WOOD = dict(A=dict(fill=110, ftau=(.25, .55), fill_db=-10, anchors=True, atau=(.5, 1.0), hi=950,
                   wan=(2.6, 7), bump=(2.3, (40, 110), .5, 4.0), hiss=-13, rumble=-30),
            B=dict(fill=150, ftau=(.2, .45), fill_db=0, anchors=False, atau=None, hi=1000,
                   wan=(3.0, 6), bump=(3.0, (30, 80), .6, 3.0), hiss=-12, rumble=-26),
            C=dict(fill=55, ftau=(.5, 1.1), fill_db=-4, anchors=True, atau=(.8, 1.4), hi=720,
                   wan=(1.8, 5), bump=(1.6, (60, 160), .4, 2.4), hiss=-18, rumble=-24))

def roll_wood_metal(v, dur=1.77):
    """A: singing rail (the original's balance: a few measured strong lines over a dense random fill);
    B: rolling whirr (an own dense field, no measured lines, its colour moving between two rail sections, quicker
    and grittier); C: hollow tube (fewer, darker lines ringing twice as long, a tonal hum more than a whirr)."""
    n = int(dur * SR); r = rng(f"rail-wood-{v}"); P = WOOD[v]
    sh = wood_shape if v != 'C' else (lambda f: wood_shape(f) - 6 * max(0, np.log2(f / 300)))
    e = r.normal(0, 1, n)
    rate, ms, spread, gain = P['bump']; b = bumps(n, r, rate, ms, spread)
    g = wander(n, r, *P['wan']) * (1 + gain * b / (b.max() + 1e-12))
    soft = lambda s: lowpass(s, 1400, 2)
    def fld():
        m = field(r, 125, P['hi'], P['fill'], P['ftau'], lambda f: sh(f) + P['fill_db'])
        if P['anchors']: m += [(f, r.uniform(*P['atau']), 10 ** (db / 20)) for f, db in WOOD_ANCHORS if f < P['hi']]
        return m
    if v == 'B':
        m1, m2 = fld(), fld(); x = wander(n, r, 1.2, 1); xf = (x - x.min()) / (np.ptp(x) + 1e-12)
        y = (cyc(lambda s: modal(soft(s), m1), e * g * np.sqrt(1 - xf))
             + cyc(lambda s: modal(soft(s), m2), e * g * np.sqrt(xf)))
    else:
        y = cyc(lambda s: modal(soft(s), fld()), e * g)
    y = norm(y)
    # the contact's own hiss rides on the bumps (they reach up to 8 kHz in the original), the ball's soft body under
    hg = wander(n, r, 4, 3) * (.35 + b / (b.max() + 1e-12))
    hiss = norm(cyc(lambda s: bandpass(s, 1200, 6000, 2), r.normal(0, 1, n) * hg))
    rumble = norm(cyc(lambda s: bandpass(s, 120, 300, 2), e * g))
    y = y + 10 ** (P['hiss'] / 20) * hiss + 10 ** (P['rumble'] / 20) * rumble
    return cyc(lambda s: highpass(s, 125, 4), y)          # the resonators' low skirts: the original has ~nothing below 130 Hz

# ------------------------------------------------------------------ the stone ball on rails
STONE = dict(A=dict(low=(80, 175, 26, (.25, .45)), mid=(450, 900, 40, (.15, .35)), hi=(1300, 2400, 10, (.08, .2)),
                    mid_db=-9, hi_db=-24, scr=(4, (25, 80), .6), scr_db=-12, floor_db=-24, wan=(4.5, 9)),
             B=dict(low=(78, 180, 34, (.18, .32)), mid=(450, 900, 25, (.15, .3)), hi=(1300, 2400, 6, (.08, .2)),
                    mid_db=-14, hi_db=-30, scr=(3, (25, 70), .7), scr_db=-15, floor_db=-28, wan=(4.0, 11)),
             C=dict(low=(80, 175, 22, (.3, .55)), mid=(450, 900, 60, (.3, .6)), hi=(1200, 2600, 30, (.15, .35)),
                    mid_db=-7, hi_db=-18, scr=(4.5, (25, 90), .5), scr_db=-13, floor_db=-23, wan=(4.5, 7)))

def roll_stone_metal(v, dur=2.24):
    """A: growl + ring (a rough hum of beating lines round 122 Hz in blobs, a 450-900 Hz ring struck by small
    irregular scrapes, a faint continuous grind); B: heavier growl (wider, rougher, deeper blobs; fewer, softer
    scrapes, less ring); C: bright steel (the ring and the 1.2-2.6 kHz partials louder and longer)."""
    n = int(dur * SR); r = rng(f"rail-stone-{v}"); P = STONE[v]
    low_shape = lambda f: -12 * abs(np.log2(f / 122))
    lo, hi, k, tau = P['low']; ml = field(r, lo, hi, k, tau, low_shape)
    lo, hi, k, tau = P['mid']; mm = field(r, lo, hi, k, tau, lambda f: -6 * abs(np.log2(f / 700)))
    lo, hi, k, tau = P['hi']; mh = field(r, lo, hi, k, tau, lambda f: 0)
    e = r.normal(0, 1, n); g = wander(n, r, *P['wan'])
    growl = norm(cyc(lambda s: highpass(modal(lowpass(s, 600, 2), ml), 72, 4), e * g))
    rate, ms, spread = P['scr']; scr, env = scrapes(n, r, rate, ms, 700, 9000, spread)
    env = env / (env.max() + 1e-12)
    drive = r.normal(0, 1, n) * (.05 * wander(n, r, 2, 8) + env)     # the scrapes strike the rail's ring
    ring = norm(cyc(lambda s: modal(s, mm), drive)); bright = norm(cyc(lambda s: modal(s, mh), drive))
    grind = norm(cyc(lambda s: bandpass(s, 900, 9000, 2), r.normal(0, 1, n) * g))
    return (growl + 10 ** (P['mid_db'] / 20) * ring + 10 ** (P['hi_db'] / 20) * bright
            + 10 ** (P['scr_db'] / 20) * norm(scr) + 10 ** (P['floor_db'] / 20) * grind)

# ------------------------------------------------------------------ output
TARGET = {'Wood': -36.9, 'Stone': -27.5}      # the original's integrated loudness (LUFS), as the old neuro files
LEN = {'Wood': 1.77, 'Stone': 2.24}

def lufs(path):
    o = subprocess.run(['ffmpeg', '-nostats', '-i', path, '-af', 'ebur128', '-f', 'null', '-'],
                       capture_output=True, text=True).stderr
    return float([l for l in o.splitlines() if l.strip().startswith('I:')][-1].split()[1])

def wav(path, y):
    import scipy.io.wavfile as wf
    wf.write(path, SR, (np.clip(y, -1, 1) * 32767).astype(np.int16))

def ogg(src, dst):
    subprocess.run(['ffmpeg', '-loglevel', 'error', '-y', '-i', src, '-c:a', 'libvorbis', '-b:a', '96k', dst],
                   check=True)

def level(y, target, tmp):
    """Scale to the target LUFS (measured on three copies of the loop, so the gate sees a steady sound)."""
    y = y / (np.abs(y).max() + 1e-12) * .5
    wav(tmp, np.tile(y, 3)); return y * 10 ** ((target - lufs(tmp)) / 20)

def demo(y, secs=8.0, top=16.0):
    """The loop as the game plays it while the ball speeds up and slows down again: speed 1 -> top -> 1 m/s,
    volume = speed x 0.05, pitch (playback rate) = 0.5 + speed x 0.01 (ballsound.rs, MultiRollSoundControl)."""
    n = int(secs * SR); t = np.arange(n) / n
    speed = 1 + (top - 1) * np.sin(np.pi * t) ** 2
    pos = np.cumsum(0.5 + speed * 0.01); i = pos.astype(int); fr = pos - i; L = len(y)
    out = (y[i % L] * (1 - fr) + y[(i + 1) % L] * fr) * speed * 0.05
    k = int(.05 * SR); out[:k] *= np.linspace(0, 1, k); out[-k:] *= np.linspace(1, 0, k)
    return out

def main():
    out = sys.argv[1]; os.makedirs(out, exist_ok=True); tmp = os.path.join(out, '.tmp.wav')
    DEMO_GAIN = {'Wood': 10 ** (14 / 20), 'Stone': 10 ** (4 / 20)}   # one gain per ball, so variants compare fairly
    for ball, fn in (('Wood', roll_wood_metal), ('Stone', roll_stone_metal)):
        for v in 'ABC':
            y = level(np.nan_to_num(fn(v, LEN[ball])), TARGET[ball], tmp)
            base = os.path.join(out, f'Roll_{ball}_Metal_{v}')
            wav(base + '.wav', y); ogg(base + '.wav', base + '.ogg')
            wav(tmp, demo(y) * DEMO_GAIN[ball]); ogg(tmp, base + '_demo.ogg')
            print(base, 'peak %.2f' % np.abs(y).max(), flush=True)
    # the old neuro loops through the same demo, to compare
    for ball in ('Wood', 'Stone'):
        old = sys.argv[2] if len(sys.argv) > 2 else None
        if old:
            import scipy.io.wavfile as wf
            subprocess.run(['ffmpeg', '-loglevel', 'error', '-y', '-i', f'{old}/Roll_{ball}_Metal.ogg', tmp], check=True)
            y = wf.read(tmp)[1].astype(float) / 32768
            wav(tmp, demo(y) * DEMO_GAIN[ball]); ogg(tmp, os.path.join(out, f'Roll_{ball}_Metal_OLD_demo.ogg'))
    os.remove(tmp)

if __name__ == '__main__': main()
