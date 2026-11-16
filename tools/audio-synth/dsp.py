"""Small synthesis toolkit: oscillators, envelopes, resonators, modal banks."""
import numpy as np
from scipy.signal import lfilter, butter, sosfilt

SR = 44100

def t_axis(dur): return np.arange(int(dur * SR)) / SR

def rng(seed): return np.random.default_rng(seed)

def phase(freq):
    """freq: scalar or per-sample array -> phase in radians."""
    f = np.asarray(freq, float)
    return 2 * np.pi * np.cumsum(f) / SR

def sine(freq, n=None):
    f = np.full(n, freq, float) if np.isscalar(freq) else freq
    return np.sin(phase(f))

def glide(f0, f1, n, curve='exp'):
    u = np.linspace(0, 1, n)
    return f0 * (f1 / f0) ** u if curve == 'exp' else f0 + (f1 - f0) * u

def adsr_exp(n, attack, decay_tau, start=0):
    t = np.arange(n) / SR - start
    e = np.where(t < 0, 0, np.where(t < attack, t / max(attack, 1e-6), np.exp(-(t - attack) / decay_tau)))
    return e

def resonator(x, f, q):
    """2-pole constant-peak band-pass (RBJ)."""
    w = 2 * np.pi * f / SR; a = np.sin(w) / (2 * q)
    b = [a, 0, -a]; A = [1 + a, -2 * np.cos(w), 1 - a]
    return lfilter(b, A, x)

def bandpass(x, lo, hi, order=4):
    hi = min(hi, SR / 2 * 0.98)
    return sosfilt(butter(order, [lo, hi], 'band', fs=SR, output='sos'), x)

def lowpass(x, fc, order=4): return sosfilt(butter(order, min(fc, SR/2*.98), 'low', fs=SR, output='sos'), x)
def highpass(x, fc, order=4): return sosfilt(butter(order, fc, 'high', fs=SR, output='sos'), x)

def modal(exc, modes):
    """modes: list of (freq, decay_seconds, gain). Exponentially decaying
    resonators driven by an excitation signal (proper modal synthesis)."""
    y = np.zeros_like(exc)
    for f, tau, g in modes:
        if f >= SR / 2 * 0.95: continue
        r = np.exp(-1 / (tau * SR)); w = 2 * np.pi * f / SR
        y += g * lfilter([np.sin(w)], [1, -2 * r * np.cos(w), r * r], exc)
    return y

def norm(x, peak=0.9):
    m = np.abs(x).max(); return x * (peak / m) if m > 0 else x

def fade(x, fin=0.002, fout=0.01):
    n1, n2 = int(fin * SR), int(fout * SR); x = x.copy()
    if n1: x[:n1] *= np.linspace(0, 1, n1)
    if n2: x[-n2:] *= np.linspace(1, 0, n2)
    return x

def pad(x, n): return np.pad(x, (0, max(0, n - len(x))))[:n]

def place(buf, x, t):
    i = int(t * SR); j = min(len(buf), i + len(x)); buf[i:j] += x[:j - i]; return buf
