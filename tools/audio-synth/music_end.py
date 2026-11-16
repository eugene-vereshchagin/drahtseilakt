"""The end music of the neuro theme, composed by rules like music.py:
- Music_EndCheckpoint: the loop near the finish. Measured in the original: 14.11 s = 64 eighth notes at 0.2205 s
  (136 per minute, steady), D minor colour (D, A, F, C), a low drone for the first ~3.4 s, then a driving plucked
  ostinato around 200 Hz over a pulse; loudest at its start (-13 dB), then about -17 dB. Ours: the same length and
  grid, a pentatonic D ostinato that builds bar by bar (more notes, a drum, a higher line), rendered cyclically
  (reverb and ringing tails wrap round) so it loops without a seam.
- Music_Final: 26.55 s, -22.6 dB; a struck opening, then slow ringing notes in A (A, E, B, D, C#), dying over ~20 s.
- Music_LastFinal: 7.69 s, -25.2 dB; a bright high chime (A4-D6) with a soft low rumble, ringing out by ~5 s."""
import zlib, numpy as np
from dsp import SR, lowpass
from music import pluck, drum, reverb, place, hz, scale, NOTE, YU, GONG, bend_wave, bend_slide

def _level(y, rms, peak=.9):
    y = y * 10 ** (rms / 20) / (np.sqrt(np.mean(y ** 2)) + 1e-12)
    pk = np.abs(y).max()
    return y * (peak / pk) if pk > peak else y

def end_checkpoint():
    r = np.random.default_rng(zlib.crc32(b"end-checkpoint"))
    e = .2205; n = int(round(64 * e * SR)); L = n
    buf = np.zeros((3 * n, 2))                                   # two passes: the second one is the loop
    sc = scale('D', YU, 50, 74)                                  # D F G A C
    cell = [0, 2, 1, 2, 0, 3, 1, 2]                              # an ostinato figure over the scale degrees
    for p in range(2):
        for bar in range(8):
            t0 = p * L / SR + bar * 8 * e
            # the drone of the first bars, the bass on every bar
            if bar < 2:
                place(buf, .5 * pluck(hz(38), 4 * e * 8 / 4, vel=.9, bright=.3, bend=bend_wave(.1)), t0, -.1)
            place(buf, .6 * pluck(hz(38 if bar % 4 != 3 else 45), 1.5, vel=.9, bright=.4, damp=.5), t0)
            shift = [0, 0, 1, 0, 2, 1, 3, 2][bar]                # the figure climbs as the loop builds
            for k in range(8):
                if bar < 2 and k % 2:                            # sparse at first
                    continue
                m = sc[min(len(sc) - 1, cell[k] + shift + (1 if bar >= 4 and k == 5 else 0))]
                place(buf, pluck(hz(m), .6, vel=(.75 if k % 2 == 0 else .5) * (1 + .04 * bar), r=r, damp=.09),
                      t0 + k * e, pan=-.2 + .05 * k)
            if bar >= 2:                                         # the pulse: a soft drum on the beats
                for k in range(0, 8, 2):
                    place(buf, .35 * drum(.5, f0=58, vel=.8 if k == 0 else .5, r=r), t0 + k * e)
            if bar >= 4:                                         # a high answering line, every other beat
                for k in (3, 7):
                    m = sc[min(len(sc) - 1, 5 + (bar + k) % 4)] + 12
                    place(buf, .35 * pluck(hz(m), .9, vel=.6, r=r, damp=.2), t0 + k * e + e / 2, pan=.4)
            if bar == 7:                                         # a run into the loop's start
                for k in range(6):
                    place(buf, .4 * pluck(hz(sc[4 + k]), .5, vel=.4 + .08 * k, r=r, damp=.1), t0 + 6 * e + k * e / 3, pan=.3)
    # the first pass's tails ring into the second, as the loop's own tails ring past its wrap: the second pass is
    # the seamless loop; the reverb is made cyclic the same way
    y = buf[n:2 * n].copy()
    yy = reverb(np.concatenate([y, y]), rt=1.1, wet=.15, seed=7)[n:]
    return _level(yy, -16.5)

def final():
    r = np.random.default_rng(zlib.crc32(b"final"))
    d = 26.55; n = int(d * SR); buf = np.zeros((n, 2))
    place(buf, drum(2.5, f0=hz(33), vel=1., r=r), .02)          # the struck opening
    place(buf, .45 * pluck(hz(45), 9., vel=1., bright=.4), .03)
    for k, m in enumerate(scale('A', GONG, 57, 81)):             # a sweep up the strings (gua zou)
        place(buf, .5 * pluck(hz(m), 6., vel=.4 + .03 * k, r=r), .05 + k * .05, pan=-.4 + .07 * k)
    phrase = [(2.1, 76, None), (2.5, 74, None), (3.9, 71, bend_slide(2)), (6.7, 69, bend_wave(.3)),
              (8.7, 64, None), (8.8, 69, None), (12.4, 73, None), (13.7, 71, None), (14.1, 69, bend_slide(2)),
              (14.85, 64, None), (15.4, 69, bend_wave(.35)), (15.5, 57, None)]
    for t, m, b in phrase:
        place(buf, pluck(hz(m), 7., vel=r.uniform(.6, .85), bend=b, r=r), t, pan=r.uniform(-.3, .3))
    place(buf, .35 * pluck(hz(45), 9., vel=.9, bright=.4), 8.7)
    place(buf, .4 * pluck(hz(33), 10., vel=.9, bright=.3), 15.5)
    y = reverb(buf, rt=3.0, wet=.32, seed=11)
    t = np.arange(n) / SR
    y *= np.clip((d - t) / 3, 0, 1)[:, None]
    return _level(y, -22.6)

def last_final():
    r = np.random.default_rng(zlib.crc32(b"last-final"))
    d = 7.69; n = int(d * SR); buf = np.zeros((n, 2)); t = np.arange(n) / SR
    rumble = lowpass(r.normal(0, 1, n), 45, 2) * np.exp(-t / 2.0) * np.clip(t / .3, 0, 1)
    buf += .08 * rumble[:, None] / (np.abs(rumble).max() + 1e-9)
    hi = scale('A', GONG, 69, 86)                                # A B C# E F# up to D6 and over
    times = [.32, .52, .66, .8, .94, 1.96, 2.11, 2.25, 2.4, 2.56, 2.93, 3.26]
    idx = [4, 6, 7, 8, 9, 5, 7, 8, 9, 10, 8, 6]
    for tt, i in zip(times, idx):
        m = hi[min(i, len(hi) - 1)]
        place(buf, pluck(hz(m), 4., vel=r.uniform(.55, .8), bright=1.3, r=r), tt, pan=r.uniform(-.5, .5))
    y = reverb(buf, rt=2.4, wet=.35, seed=13)
    y *= np.clip((d - t) / 1.5, 0, 1)[:, None]
    return _level(y, -25.2)

def render(name):
    return {'Music_EndCheckpoint': end_checkpoint, 'Music_Final': final, 'Music_LastFinal': last_final}[name]()
