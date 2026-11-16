# Centroid and level of an original sound in steps. usage: python3 track.py NAME STEP_SECONDS
import sys, numpy as np
from analyse import load, D
n=sys.argv[1]; step=float(sys.argv[2])
sr,x=load(D+n+'.wav'); N=int(step*sr)
out=[]
for i in range(0,len(x)-N+1,N):
    s=x[i:i+N]*np.hanning(N); S=np.abs(np.fft.rfft(s))**2; f=np.fft.rfftfreq(N,1/sr)
    c=(f*S).sum()/(S.sum()+1e-12); 
    out.append(f'{c/1000:.1f}k/{10*np.log10(np.mean(x[i:i+N]**2)+1e-12):.0f}')
print(n,' '.join(out))
