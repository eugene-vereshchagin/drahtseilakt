# The strongest spectral peaks of an original sound over time. usage: python3 peaks.py NAME [STEP_SECONDS]
import os, sys, numpy as np, scipy.signal as ss
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyse import load, D
n=sys.argv[1]; step=float(sys.argv[2]) if len(sys.argv)>2 else 0.1
sr,x=load(D+n+'.wav')
N=4096 if len(x)>8192 else 1024
for t0 in np.arange(0,len(x)/sr,step):
    i=int(t0*sr); seg=x[i:i+N]
    if len(seg)<256: break
    seg=np.pad(seg,(0,N-len(seg)))*np.hanning(N)
    S=np.abs(np.fft.rfft(seg)); f=np.fft.rfftfreq(N,1/sr)
    p,_=ss.find_peaks(S,distance=5); p=p[np.argsort(S[p])[-6:][::-1]]
    lv=20*np.log10(S.max()+1e-9)
    print(f'{t0:5.2f} {lv:5.0f}dB', ' '.join(f'{f[k]:.0f}({20*np.log10(S[k]/S.max()):.0f})' for k in p))
