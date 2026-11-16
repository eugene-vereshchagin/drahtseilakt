"""Similarity between an original and a generated sound (no librosa).
Metrics: spectral-envelope correlation (mean log-mel), temporal envelope
correlation (RMS dB on a time-normalised grid), MFCC-DTW distance."""
import numpy as np
from scipy.fft import dct
from scipy.signal import resample_poly

SR = 22050

def to_sr(x, sr):
    if sr == SR: return x
    from math import gcd
    g = gcd(sr, SR); return resample_poly(x, SR // g, sr // g)

def melfb(n_fft, n_mels=40, sr=SR, fmin=40, fmax=10000):
    m = lambda f: 2595 * np.log10(1 + f / 700)
    im = lambda m_: 700 * (10 ** (m_ / 2595) - 1)
    pts = im(np.linspace(m(fmin), m(fmax), n_mels + 2))
    f = np.fft.rfftfreq(n_fft, 1 / sr); fb = np.zeros((n_mels, len(f)))
    for i in range(n_mels):
        l, c, r = pts[i:i + 3]
        fb[i] = np.clip(np.minimum((f - l) / (c - l), (r - f) / (r - c)), 0, None)
    return fb

def logmel(x, n_fft=1024, hop=256):
    if len(x) < n_fft: x = np.pad(x, (0, n_fft - len(x)))
    fr = np.lib.stride_tricks.sliding_window_view(x, n_fft)[::hop] * np.hanning(n_fft)
    P = np.abs(np.fft.rfft(fr, axis=1)) ** 2
    return 10 * np.log10(P @ melfb(n_fft).T + 1e-10)  # frames x mels

def env(x, n=60):
    seg = np.array_split(x, n)
    return np.array([10 * np.log10(np.mean(s ** 2) + 1e-9) for s in seg])

def dtw(A, B):
    D = np.sqrt(((A[:, None, :] - B[None, :, :]) ** 2).sum(-1))
    n, m = D.shape; C = np.full((n + 1, m + 1), np.inf); C[0, 0] = 0
    for i in range(1, n + 1):
        Ci, Cp, Di = C[i], C[i - 1], D[i - 1]
        for j in range(1, m + 1):
            Ci[j] = Di[j - 1] + min(Cp[j], Ci[j - 1], Cp[j - 1])
    return C[n, m] / (n + m)

def corr(a, b):
    a = a - a.mean(); b = b - b.mean()
    return float((a * b).sum() / (np.sqrt((a * a).sum() * (b * b).sum()) + 1e-12))

def compare(orig, osr, gen, gsr, loop=False):
    o = to_sr(orig, osr); g = to_sr(gen, gsr)
    o = o / (np.abs(o).max() + 1e-9); g = g / (np.abs(g).max() + 1e-9)
    Lo, Lg = logmel(o), logmel(g)
    # weight frames by energy so silence doesn't dominate the mean spectrum
    def mean_spec(L):
        w = 10 ** (L.max(1) / 10); return 10 * np.log10((10 ** (L / 10) * w[:, None]).sum(0) / w.sum())
    # floor 50 dB below the peak so spectral 'holes' between partials do not dominate
    so, sg = mean_spec(Lo), mean_spec(Lg)
    spec = corr(np.maximum(so, so.max() - 50), np.maximum(sg, sg.max() - 50))
    if loop:  # stationary loops: compare fluctuation depth, not the time course
        eo, eg = env(o, 40), env(g, 40); e = float(np.exp(-abs(eo.std() - eg.std()) / 3))
    else:
        e = corr(np.maximum(env(o), -70), np.maximum(env(g), -70))
    # MFCC-DTW on <=150 frames each
    def mf(L):
        L = np.maximum(L, L.max() - 80)
        M = dct(L, type=2, axis=1, norm='ortho')[:, 1:13]
        step = max(1, len(M) // 150); return M[::step]
    d = dtw(mf(Lo), mf(Lg))
    durr = min(len(o), len(g)) / max(len(o), len(g))
    score = 100 * (0.35 * max(spec, 0) + 0.25 * max(e, 0) + 0.25 * np.exp(-d / 40) + 0.15 * durr)
    return dict(score=round(float(score), 1), spec=round(spec, 3), env=round(e, 3), mfcc_dtw=round(float(d), 1), dur_ratio=round(durr, 2))
