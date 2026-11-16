"""Generated level music for the neuro theme: short, quiet East-Asian plucked-string phrases (guzheng-like),
one for each of the original's Music_Theme_<n>_<part> roles, plus the end music (Music_EndCheckpoint loop,
Music_Final, Music_LastFinal).

Nothing of the original is used: from its files only numbers were measured (length, loudness, the pitch classes
that sound most, the onset grid and density, how long the phrase plays before its tail) and written into SPEC
below. Every phrase is composed here by its own seeded rules (contour, rhythm, ornaments), not one loop transposed.

usage: python3 music.py [role ...]   -> generated/music/<role>.wav (stereo) and compare/music_*_compare.wav"""
import sys, os, zlib, numpy as np, scipy.io.wavfile as wf
from scipy.signal import fftconvolve, resample_poly
from dsp import SR, lowpass, highpass, resonator
from analyse import D

H = os.path.dirname(os.path.abspath(__file__))
NOTE = {n: i for i, n in enumerate('C C# D D# E F F# G G# A A# B'.split())}

def hz(m): return 440 * 2 ** ((m - 69) / 12)

def scale(root, steps, lo, hi):
    """MIDI notes of a pentatonic scale (root name, semitone steps) between lo and hi."""
    r = NOTE[root]
    return [m for m in range(lo, hi + 1) if (m - r) % 12 in steps]

GONG = (0, 2, 4, 7, 9)      # do re mi sol la
SHANG = (0, 2, 5, 7, 10)    # re mi sol la do, from re
YU = (0, 3, 5, 7, 10)       # la do re mi sol, from la (minor pentatonic)

# measured from the original's parts: duration, RMS dB, onset grid (s), onsets, seconds the phrase plays, mode
# (root, steps), bass root (MIDI) and the style of the theme
SPEC = {
 'Music_Theme_1_1': dict(dur=17.46, rms=-20.2, grid=.203, notes=40, play=13.8, mode=('E', SHANG), bass=40, style='ostinato'),
 'Music_Theme_1_2': dict(dur=14.91, rms=-21.5, grid=.195, notes=32, play=10.5, mode=('E', SHANG), bass=40, style='ostinato'),
 'Music_Theme_1_3': dict(dur=9.14, rms=-21.9, grid=.195, notes=21, play=5.1, mode=('E', SHANG), bass=40, style='ostinato'),
 'Music_Theme_2_1': dict(dur=7.33, rms=-23.6, grid=.245, notes=12, play=4.4, mode=('E', SHANG), bass=45, style='call'),
 'Music_Theme_2_2': dict(dur=3.88, rms=-20.2, grid=.245, notes=5, play=1.45, mode=('E', YU), bass=40, style='call'),
 'Music_Theme_2_3': dict(dur=5.13, rms=-20.1, grid=.245, notes=11, play=2.25, mode=('E', SHANG), bass=40, style='call'),
 'Music_Theme_3_1': dict(dur=16.07, rms=-21.6, grid=.40, notes=15, play=9.6, mode=('C', GONG), bass=36, style='slow'),
 'Music_Theme_3_2': dict(dur=8.82, rms=-21.9, grid=.415, notes=11, play=4.6, mode=('A', YU), bass=36, style='slow'),
 'Music_Theme_3_3': dict(dur=10.19, rms=-24.5, grid=.42, notes=8, play=6.8, mode=('G', GONG), bass=36, style='slow'),
 'Music_Theme_4_1': dict(dur=11.19, rms=-23.7, grid=.1, notes=5, play=.65, mode=('C#', YU), bass=37, style='gliss'),
 'Music_Theme_4_2': dict(dur=12.91, rms=-23.4, grid=.17, notes=9, play=2.1, mode=('E', YU), bass=40, style='gliss'),
 'Music_Theme_4_3': dict(dur=22.89, rms=-17.6, grid=.175, notes=17, play=20.9, mode=('B', YU), bass=35, style='waves'),
 'Music_Theme_5_1': dict(dur=17.93, rms=-17.1, grid=.385, notes=16, play=12.1, mode=('C', YU), bass=36, style='drums'),
 'Music_Theme_5_2': dict(dur=20.53, rms=-18.6, grid=.385, notes=13, play=12.1, mode=('A#', YU), bass=34, style='drums'),
 'Music_Theme_5_3': dict(dur=21.83, rms=-17.3, grid=.385, notes=11, play=8.9, mode=('A', YU), bass=33, style='drums'),
}

# ------------------------------------------------------------------ instruments
def pluck(f0, dur, vel=1., bend=None, trem=0., bright=1., r=None, damp=None):
    """A guzheng-like plucked string by additive synthesis: inharmonic-free partials shaped by the plucking point
    (near the bridge: bright), each dying faster the higher it is; a pitch track (`bend`: function of time ->
    semitones) for the press-bends and slides of the left hand; `trem` adds the yaolun tremolo (fast re-plucks)."""
    n = int(dur * SR); t = np.arange(n) / SR
    semis = bend(t) if bend else 0.
    f = f0 * 2 ** (semis / 12) * (1 + .0015 * np.sin(2 * np.pi * 5.2 * t) * np.clip(t - .3, 0, 1))
    ph = 2 * np.pi * np.cumsum(f * np.ones(n)) / SR
    y = np.zeros(n); pos = .13
    T = 1.2 + 2.6 * (110 / f0) ** .5                                   # low strings ring longer
    for k in range(1, 24):
        if k * f0 > 9000: break
        a = abs(np.sin(np.pi * k * pos)) / k ** (1.25 - .25 * bright)
        tau = T / (1 + .045 * k * k)
        y += a * np.sin(k * ph * (1 + .0004 * k)) * np.exp(-t / tau)
    att = int(.004 * SR); y[:att] *= np.linspace(0, 1, att)
    if r is not None:                                                  # the nail's click
        m = int(.012 * SR); y[:m] += .15 * highpass(r.normal(0, 1, m), 2500) * np.exp(-np.arange(m) / 80)
    if trem:
        y *= 1 - trem * .5 * (1 - np.cos(2 * np.pi * 11 * t)) * np.clip(t / .2, 0, 1)
    if damp:                                                           # a damped (muted) note: short and dry
        y *= np.exp(-t / damp)
    return vel * y

def drum(dur=1.2, f0=55, vel=1., r=None):
    """A low barrel drum: a pitched membrane thump gliding down a little, with a soft skin noise."""
    n = int(dur * SR); t = np.arange(n) / SR
    f = f0 * (1 + .5 * np.exp(-t / .03))
    y = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / .35)
    y += .4 * np.sin(2 * np.pi * np.cumsum(f * 1.6) / SR) * np.exp(-t / .15)
    if r is not None:
        y += .3 * lowpass(r.normal(0, 1, n), 900) * np.exp(-t / .04)
    return vel * y

def reverb(x, rt=1.8, wet=.28, seed=1):
    """A soft room: stereo, decorrelated decaying noise as the impulse response, darker as it fades."""
    r = np.random.default_rng(seed); n = int(rt * 1.2 * SR); t = np.arange(n) / SR
    out = []
    for c in range(2):
        ir = r.normal(0, 1, n) * np.exp(-6.9 * t / rt)
        ir = .6 * lowpass(ir, 5000, 2) + .4 * lowpass(ir, 1500, 2)
        ir[:int(.012 * SR)] = 0; ir /= np.sqrt((ir ** 2).sum())
        out.append(fftconvolve(x[:, c], ir)[:len(x)])
    return (1 - wet) * x + wet * np.stack(out, 1) * 3

def place(buf, y, t, pan=0.):
    i = int(t * SR)
    if i >= len(buf): return
    j = min(len(buf), i + len(y)); g = np.array([np.cos((pan + 1) * np.pi / 4), np.sin((pan + 1) * np.pi / 4)])
    buf[i:j] += y[:j - i, None] * g

# ------------------------------------------------------------------ composition
def bend_press(amount, at):
    """Press the string after the pluck: up `amount` semitones from `at` s, gently (an)."""
    return lambda t: amount * np.clip((t - at) / .12, 0, 1)

def bend_slide(amount):
    """Slide into the note from `amount` semitones below (hua yin)."""
    return lambda t: -amount * np.clip(1 - t / .1, 0, 1)

def bend_wave(depth):
    """Rou xian: the left hand presses and releases, a slow wide vibrato after the pluck."""
    return lambda t: depth * np.sin(2 * np.pi * 4.5 * t) * np.clip((t - .25) / .3, 0, 1)

def melody(sp, r, notes, grid, span, lo, hi):
    """Onset times and pitches of a phrase: a contour walking the pentatonic scale in small steps, arching up and
    coming home to the mode's root, its rhythm on the grid with rests and the odd pair of quick notes."""
    sc = scale(sp['mode'][0], sp['mode'][1], lo, hi)
    home = [i for i, m in enumerate(sc) if (m - NOTE[sp['mode'][0]]) % 12 == 0]
    i = home[len(home) // 2] if home else len(sc) // 2
    slots = max(notes, int(span / grid))
    times = np.sort(r.choice(slots, size=min(notes, slots), replace=False)) * grid
    if r.random() < .6 and len(times) > 3:                            # a pair of quick notes somewhere
        k = r.integers(1, len(times) - 1); times[k] = times[k - 1] + grid / 2
    out = []
    for k, t in enumerate(times):
        arch = np.sin(np.pi * k / max(1, len(times) - 1))
        target = (home[len(home) // 2] if home else i) + int(round(3 * arch))
        step = int(np.sign(target - i)) if r.random() < .6 else r.choice([-1, 1, -2, 2])
        i = int(np.clip(i + step, 0, len(sc) - 1))
        out.append((float(t), sc[i]))
    if home:                                                         # end on the root
        out[-1] = (out[-1][0], sc[min(home, key=lambda h: abs(h - i))])
    return out

def compose(name, sp):
    r = np.random.default_rng(zlib.crc32(name.encode()))
    n = int(sp['dur'] * SR); buf = np.zeros((n, 2)); g = sp['grid']; style = sp['style']
    ring = lambda m, d: max(1.5, min(5.5, d))
    if style in ('ostinato', 'call', 'slow', 'waves', 'drums'):
        lo, hi = (55, 81) if style != 'slow' else (52, 79)
        mel = melody(sp, r, sp['notes'], g, sp['play'], lo, hi)
        for k, (t, m) in enumerate(mel):
            last = k == len(mel) - 1
            orn = r.random()
            if last:
                bend = bend_wave(.35); trem = 0
            elif orn < .18:
                bend = bend_slide(2 if (m % 12) in (2, 7, 9) else 1); trem = 0
            elif orn < .3:
                bend = bend_press(1 if r.random() < .5 else 2, .15); trem = 0
            elif orn < .38 and style in ('slow', 'waves'):
                bend = bend_wave(.25); trem = .5
            else:
                bend = None; trem = 0
            d = 4.5 if last else ring(m, 3.0)
            y = pluck(hz(m), d, vel=r.uniform(.55, .9) * (1.15 if last else 1), bend=bend, trem=trem, r=r)
            place(buf, y, t + .03, pan=r.uniform(-.35, .35))
        # bass: the root (and its fifth) under the phrase, every bar of the grid
        bar = g * (16 if style == 'ostinato' else 8)
        t = 0.
        while t < sp['play'] - 1.:
            m = sp['bass'] + (7 if (r.random() < .3 and t > 0) else 0)
            place(buf, .45 * pluck(hz(m), 4., vel=.8, bright=.4), t + .03, pan=-.1)
            if style == 'ostinato':                                    # the octave answer half a bar later
                place(buf, .3 * pluck(hz(m + 12), 2.5, vel=.6, bright=.6), t + .03 + bar / 2, pan=.15)
            t += bar
    if style in ('gliss', 'waves'):
        # gua zou: a sweep up the strings, then the ringing chord left to die
        sc = scale(sp['mode'][0], sp['mode'][1], sp['bass'], sp['bass'] + 36)
        t0 = .1 if style == 'gliss' else 0.
        up = r.random() < .7 or style == 'waves'
        seq = sc if up else sc[::-1]
        step = sp['grid'] * .35 if style == 'gliss' else .055
        reps = 1 if style == 'gliss' else 2
        for rep in range(reps):
            tt = t0 + rep * (sp['play'] * .55 if style == 'waves' else 0)
            for k, m in enumerate(seq):
                place(buf, pluck(hz(m), 6., vel=.35 + .4 * k / len(seq), r=r), tt + k * step, pan=-.4 + .8 * k / len(seq))
            # the closing notes: a root low and high, with a slow rou xian on the top one
            tend = tt + len(seq) * step + .15
            place(buf, pluck(hz(sp['bass']), 7., vel=.9, bright=.5), tend)
            place(buf, pluck(hz(sp['bass'] + 24), 6., vel=.6, bend=bend_wave(.4)), tend + sp['grid'] * 2, pan=.2)
    if style == 'drums':
        # low drums on a slow pulse, a roll before the phrase turns, and a glissando sweep between
        beat = sp['grid'] * 2
        t = 0.
        while t < sp['play']:
            place(buf, drum(f0=hz(sp['bass']) * (1 if r.random() < .7 else 1.5), vel=r.uniform(.7, 1.), r=r), t)
            if r.random() < .25:
                for q in range(3):
                    place(buf, drum(.6, f0=hz(sp['bass'] + 7), vel=.35 + .1 * q, r=r), t + beat / 2 + q * beat / 6)
            t += beat * (2 if r.random() < .4 else 1)
        sc = scale(sp['mode'][0], sp['mode'][1], sp['bass'] + 12, sp['bass'] + 40)
        ts = sp['play'] * r.uniform(.25, .45)
        seq = sc if r.random() < .5 else sc[::-1]
        for k, m in enumerate(seq):
            place(buf, pluck(hz(m), 4., vel=.3 + .25 * k / len(seq), r=r), ts + k * .045, pan=-.5 + k / len(seq))
    y = reverb(buf, rt=2.2 if style in ('slow', 'waves', 'gliss') else 1.7, seed=zlib.crc32(name.encode()) & 0xffff)
    # level: the original part's RMS, the phrase's tail faded so the file ends in silence
    t = np.arange(n) / SR
    y *= np.clip((sp['dur'] - t) / 1.5, 0, 1)[:, None]
    y *= 10 ** (sp['rms'] / 20) / (np.sqrt(np.mean(y ** 2)) + 1e-12)
    pk = np.abs(y).max()
    if pk > .9: y *= .9 / pk
    return y

def load_stereo(name):
    sr, x = wf.read(D + name + '.wav'); x = x.astype(float) / 32768
    if x.ndim == 1: x = np.stack([x, x], 1)
    if sr != SR: x = resample_poly(x, SR // 50, sr // 50, axis=0)
    return x

def write(name, y):
    os.makedirs(f'{H}/generated/music', exist_ok=True)
    wf.write(f'{H}/generated/music/{name}.wav', SR, (np.clip(y, -1, 1) * 32767).astype(np.int16))
    o = load_stereo(name)                                              # listening aid only, never shipped
    cmp_ = np.concatenate([o, np.zeros((SR, 2)), y])
    tag = {'Music_EndCheckpoint': 'music_end_checkpoint', 'Music_Final': 'music_end_final',
           'Music_LastFinal': 'music_end_lastfinal'}.get(name, name.replace('Music_', 'music_').lower())
    wf.write(f'{H}/compare/{tag}_compare.wav', SR, (np.clip(cmp_, -1, 1) * 32767).astype(np.int16))
    rms = lambda x: 20 * np.log10(np.sqrt(np.mean(x ** 2)) + 1e-12)
    print(f'{name:22s} dur {len(o) / SR:6.2f}/{len(y) / SR:6.2f}  rms {rms(o):6.1f}/{rms(y):6.1f}  '
          f'peak {np.abs(o).max():.2f}/{np.abs(y).max():.2f}')

def main():
    roles = sys.argv[1:] or list(SPEC)
    for name in roles:
        if name in SPEC:
            write(name, compose(name, SPEC[name]))
        else:
            import music_end
            write(name, music_end.render(name))

if __name__ == '__main__': main()
