# Measures the original's sounds (length, loudness, centroid, peaks, envelope): the numbers the synthesis is tuned to.
# The original's files are only read and measured, never copied into anything made here.
# usage: python3 analyse.py NAME ...   (NAME as Misc_Checkpoint, from the folder D below)
import os, sys, numpy as np, scipy.io.wavfile as w, scipy.signal as ss
# the original's Sounds folder of the player's own copy: ORIGINAL_SOUNDS, else where the game keeps the copy it
# imported (<data dir>/drahtseilakt/levels/original/Sounds)
D = os.path.join(os.environ.get('ORIGINAL_SOUNDS') or os.path.join(
    os.environ.get('XDG_DATA_HOME') or os.path.expanduser('~/.local/share'),
    'drahtseilakt', 'levels', 'original', 'Sounds'), '')
def load(p):
    sr,x=w.read(p); x=x.astype(float)
    if x.ndim>1: x=x.mean(1)
    x/= (32768 if np.abs(x).max()>1.5 else 1)
    return sr,x
if __name__=="__main__":
 for n in sys.argv[1:]:
     sr,x=load(D+n+'.wav')
     dur=len(x)/sr; rms=np.sqrt(np.mean(x**2))
     f,P=ss.welch(x,sr,nperseg=4096); cen=(f*P).sum()/P.sum()
     pk=f[np.argsort(P)[-6:][::-1]]
     hop=int(sr*0.05); env=[np.sqrt(np.mean(x[i:i+hop]**2)) for i in range(0,len(x),hop)]
     env=np.array(env); e=' '.join('%d'%(20*np.log10(v+1e-6)) for v in env[:40:max(1,len(env)//20 or 1)])
     print(f'{n}: sr={sr} dur={dur:.2f}s rms={20*np.log10(rms+1e-9):.1f}dB peak={np.abs(x).max():.2f} cen={cen:.0f}Hz peaks={np.round(pk).astype(int)} attack_max@{env.argmax()*0.05:.2f}s')
     print('   env(dB/50ms):',' '.join('%d'%(20*np.log10(v+1e-6)) for v in env[:60]))
 